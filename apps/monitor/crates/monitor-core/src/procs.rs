//! Processes from `/proc/<pid>`.

use std::collections::HashMap;

/// What one reading of a process gives. CPU time is cumulative; the percentage comes from two
/// readings, see [`cpu_percent`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Process {
    pub pid: u32,
    pub name: String,
    pub state: char,
    pub uid: u32,
    /// utime + stime, in clock ticks.
    pub cpu_ticks: u64,
    /// Start time in clock ticks since boot. With the pid it names one process for good: a pid
    /// can be handed to another process later, its start time cannot repeat.
    pub start_time: u64,
    /// Resident set size in bytes.
    pub rss: u64,
    pub threads: u32,
    /// Full command line, empty for kernel threads.
    pub cmdline: String,
}

/// The fields of `/proc/<pid>/stat` the monitor uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stat {
    pub pid: u32,
    pub name: String,
    pub state: char,
    pub cpu_ticks: u64,
    pub start_time: u64,
    pub rss: u64,
    pub threads: u32,
}

/// The kernel keeps 15 bytes of a process name, so a longer one arrives cut. The command's own
/// file name is the whole of it when it starts with what is left.
pub fn full_name(comm: &str, cmdline: &str) -> String {
    if comm.len() >= 15 {
        let program = cmdline
            .split_whitespace()
            .next()
            .and_then(|first| first.rsplit('/').next())
            .unwrap_or("");
        if program.len() > comm.len() && program.starts_with(comm) {
            return program.to_owned();
        }
    }
    comm.to_owned()
}

/// Parses `/proc/<pid>/stat`. The name sits in parentheses and may itself contain spaces and
/// parentheses, so the last `)` ends it.
pub fn parse_stat(text: &str, page_size: u64) -> Option<Stat> {
    let open = text.find('(')?;
    let close = text.rfind(')')?;
    if close < open {
        return None;
    }
    let pid: u32 = text[..open].trim().parse().ok()?;
    let name = text[open + 1..close].to_string();
    let rest: Vec<&str> = text[close + 1..].split_whitespace().collect();
    // rest[0] is the state; utime and stime are fields 14 and 15 of the full line, which are
    // rest[11] and rest[12]; num_threads is field 20, rest[17]; starttime is field 22, rest[19];
    // rss in pages is field 24, rest[21].
    if rest.len() < 22 {
        return None;
    }
    let state = rest[0].chars().next()?;
    let utime: u64 = rest[11].parse().ok()?;
    let stime: u64 = rest[12].parse().ok()?;
    let threads: u32 = rest[17].parse().ok()?;
    let start_time: u64 = rest[19].parse().ok()?;
    let rss_pages: i64 = rest[21].parse().ok()?;
    Some(Stat {
        pid,
        name,
        state,
        cpu_ticks: utime + stime,
        start_time,
        rss: rss_pages.max(0) as u64 * page_size,
        threads,
    })
}

/// Parses the real uid out of `/proc/<pid>/status`.
pub fn parse_uid(status: &str) -> Option<u32> {
    status
        .lines()
        .find(|l| l.starts_with("Uid:"))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

/// `/proc/<pid>/cmdline` separates arguments with NUL.
pub fn parse_cmdline(raw: &[u8]) -> String {
    let text = String::from_utf8_lossy(raw);
    text.trim_end_matches('\0').replace('\0', " ")
}

fn clock_ticks() -> u64 {
    let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    if hz > 0 {
        hz as u64
    } else {
        100
    }
}

fn page_size() -> u64 {
    let size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if size > 0 {
        size as u64
    } else {
        4096
    }
}

/// Reads every process the kernel lists. A process that exits while being read is skipped.
pub fn read_all() -> Vec<Process> {
    let page = page_size();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|n| n.parse::<u32>().ok())
        else {
            continue;
        };
        let dir = entry.path();
        let Ok(stat) = std::fs::read_to_string(dir.join("stat")) else {
            continue;
        };
        let Some(st) = parse_stat(&stat, page) else {
            continue;
        };
        let uid = std::fs::read_to_string(dir.join("status"))
            .ok()
            .and_then(|s| parse_uid(&s))
            .unwrap_or(0);
        let cmdline = std::fs::read(dir.join("cmdline"))
            .map(|raw| parse_cmdline(&raw))
            .unwrap_or_default();
        out.push(Process {
            pid,
            name: full_name(&st.name, &cmdline),
            state: st.state,
            uid,
            cpu_ticks: st.cpu_ticks,
            start_time: st.start_time,
            rss: st.rss,
            threads: st.threads,
            cmdline,
        });
    }
    out
}

/// CPU share of one process between two readings, as a percentage of one core (so a process
/// using two cores reads 200).
pub fn cpu_percent(previous_ticks: u64, now_ticks: u64, seconds: f64) -> f64 {
    if seconds <= 0.0 {
        return 0.0;
    }
    now_ticks.saturating_sub(previous_ticks) as f64 / clock_ticks() as f64 / seconds * 100.0
}

/// Maps uid to user name from the text of `/etc/passwd`.
pub fn parse_passwd(text: &str) -> HashMap<u32, String> {
    let mut map = HashMap::new();
    for line in text.lines() {
        let mut parts = line.split(':');
        let (Some(name), Some(_), Some(uid)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        if let Ok(uid) = uid.parse() {
            map.entry(uid).or_insert_with(|| name.to_string());
        }
    }
    map
}

pub fn read_users() -> HashMap<u32, String> {
    std::fs::read_to_string("/etc/passwd")
        .map(|t| parse_passwd(&t))
        .unwrap_or_default()
}

/// Signals the process that was read as `pid` with this `start_time`, and nothing else.
///
/// A pid can be reused once its process is gone, and `kill(pid)` would then hit a stranger. So
/// this takes a pidfd first (which pins the pid to one process), checks that process is the one
/// that was sampled by its start time, and signals through the pidfd. Returns the OS error when
/// the kernel refuses (not owner, no such process); a process that is no longer the sampled one
/// reads as "no such process".
pub fn signal(pid: u32, start_time: u64, signal: Signal) -> std::io::Result<()> {
    let sig = match signal {
        Signal::Terminate => libc::SIGTERM,
        Signal::Kill => libc::SIGKILL,
        Signal::Stop => libc::SIGSTOP,
        Signal::Continue => libc::SIGCONT,
    };
    let gone = || std::io::Error::from_raw_os_error(libc::ESRCH);
    // SAFETY: pidfd_open takes two plain integers and returns a descriptor or -1.
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid as libc::pid_t, 0) };
    if fd < 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ENOSYS) {
            return Err(error);
        }
        // A kernel without pidfds: check the identity and accept the small remaining window.
        return if current_start_time(pid) == Some(start_time) {
            // SAFETY: kill takes plain integers and has no memory preconditions.
            if unsafe { libc::kill(pid as libc::pid_t, sig) } == 0 {
                Ok(())
            } else {
                Err(std::io::Error::last_os_error())
            }
        } else {
            Err(gone())
        };
    }
    let fd = fd as libc::c_int;
    // The pidfd now holds the pid, so what /proc says about it is about the process it names.
    let result = if current_start_time(pid) == Some(start_time) {
        // SAFETY: `fd` is the pidfd opened above; a null siginfo and zero flags are allowed.
        let rc = unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                fd,
                sig,
                std::ptr::null::<libc::siginfo_t>(),
                0,
            )
        };
        if rc == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    } else {
        Err(gone())
    };
    // SAFETY: `fd` is open and owned here.
    unsafe { libc::close(fd) };
    result
}

/// Start time of the process now holding `pid`, if there is one.
fn current_start_time(pid: u32) -> Option<u64> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    parse_stat(&text, 1).map(|s| s.start_time)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signal {
    Terminate,
    Kill,
    Stop,
    Continue,
}

/// A state letter in words.
pub fn state_label(state: char) -> &'static str {
    match state {
        'R' => "Running",
        'S' => "Sleeping",
        'D' => "Waiting for disk",
        'Z' => "Zombie",
        'T' | 't' => "Stopped",
        'I' => "Idle",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // pid (comm) state ppid pgrp session tty tpgid flags minflt cminflt majflt cmajflt utime stime
    // cutime cstime priority nice num_threads itrealvalue starttime vsize rss ...
    const STAT: &str = "1234 (my (odd) app) S 1 1234 1234 0 -1 4194560 100 0 0 0 150 50 0 0 20 0 7 0 5000 123456789 2500 18446744073709551615";

    #[test]
    fn a_name_cut_at_fifteen_bytes_is_completed_from_the_command() {
        assert_eq!(
            full_name(
                "gnome-shell-cal",
                "/usr/lib/gnome-shell-calendar-server --x"
            ),
            "gnome-shell-calendar-server"
        );
        // A short name, or one the command does not continue, is left alone.
        assert_eq!(full_name("bash", "/usr/bin/bash"), "bash");
        assert_eq!(
            full_name("exactly15chars!", "/usr/bin/other-thing"),
            "exactly15chars!"
        );
    }

    #[test]
    fn parses_a_name_with_spaces_and_parentheses() {
        let st = parse_stat(STAT, 4096).unwrap();
        assert_eq!(st.pid, 1234);
        assert_eq!(st.name, "my (odd) app");
        assert_eq!(st.state, 'S');
        assert_eq!(st.cpu_ticks, 200);
        assert_eq!(st.start_time, 5000);
        assert_eq!(st.rss, 2500 * 4096);
        assert_eq!(st.threads, 7);
    }

    #[test]
    fn signalling_checks_the_start_time_first() {
        // Our own pid with a start time it cannot have: refused as "no such process", and the
        // signal (SIGCONT, harmless) is never sent through a wrong identity.
        let me = std::process::id();
        let start = current_start_time(me).expect("our own stat");
        let wrong = signal(me, start + 1, Signal::Continue).unwrap_err();
        assert_eq!(wrong.raw_os_error(), Some(libc::ESRCH));
        // The right start time goes through.
        signal(me, start, Signal::Continue).unwrap();
    }

    #[test]
    fn rejects_truncated_stat() {
        assert!(parse_stat("1 (a) S 1 2", 4096).is_none());
        assert!(parse_stat("garbage", 4096).is_none());
        assert!(parse_stat("1 )a( S", 4096).is_none());
    }

    #[test]
    fn reads_the_real_uid() {
        assert_eq!(
            parse_uid("Name:\tx\nUid:\t1000\t1000\t1000\t1000\n"),
            Some(1000)
        );
        assert_eq!(parse_uid("Name:\tx\n"), None);
    }

    #[test]
    fn cmdline_joins_arguments() {
        assert_eq!(
            parse_cmdline(b"/usr/bin/foo\0--bar\0baz\0"),
            "/usr/bin/foo --bar baz"
        );
        assert_eq!(parse_cmdline(b""), "");
    }

    #[test]
    fn cpu_percent_scales_with_interval_and_never_goes_negative() {
        let hz = clock_ticks() as f64;
        let p = cpu_percent(0, hz as u64, 1.0);
        assert!((p - 100.0).abs() < 1e-6);
        assert_eq!(cpu_percent(500, 100, 1.0), 0.0);
        assert_eq!(cpu_percent(0, 100, 0.0), 0.0);
    }

    #[test]
    fn passwd_keeps_the_first_name_for_a_uid() {
        let map = parse_passwd("root:x:0:0::/root:/bin/bash\ndaniel:x:1000:1000::/home/d:/bin/zsh\nroot2:x:0:0::/:/bin/sh\n# c\n");
        assert_eq!(map[&0], "root");
        assert_eq!(map[&1000], "daniel");
    }

    #[test]
    fn reads_this_very_process() {
        let me = std::process::id();
        let all = read_all();
        let mine = all.iter().find(|p| p.pid == me).expect("own pid is listed");
        assert!(mine.rss > 0);
        assert!(mine.threads >= 1);
    }

    #[test]
    fn signalling_a_missing_process_reports_the_error() {
        // pid_max is at most 2^22; this one cannot exist.
        assert!(signal(u32::MAX / 2, 0, Signal::Terminate).is_err());
    }

    #[test]
    fn state_labels() {
        assert_eq!(state_label('R'), "Running");
        assert_eq!(state_label('?'), "Unknown");
    }
}
