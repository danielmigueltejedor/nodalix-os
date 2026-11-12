//! CPU time from `/proc/stat`.

/// Jiffies of one CPU line, split into busy and total.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CpuTimes {
    pub busy: u64,
    pub total: u64,
}

/// One sample: the aggregate line and every core that is online, each with its number. A core
/// that goes offline leaves a gap in `/proc/stat`, so cores are matched by number, never by
/// position.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CpuSample {
    pub total: CpuTimes,
    pub cores: Vec<(u32, CpuTimes)>,
}

fn parse_line(line: &str) -> Option<CpuTimes> {
    let mut fields = line.split_whitespace().skip(1).map(|f| f.parse::<u64>());
    let mut v = [0u64; 8];
    for slot in v.iter_mut() {
        match fields.next() {
            Some(Ok(n)) => *slot = n,
            Some(Err(_)) => return None,
            None => break,
        }
    }
    // user nice system idle iowait irq softirq steal. Guest time is already inside user.
    let idle = v[3] + v[4];
    let total: u64 = v.iter().sum();
    Some(CpuTimes {
        busy: total.saturating_sub(idle),
        total,
    })
}

/// Parses `/proc/stat`. Lines other than `cpu` and `cpuN` are ignored.
pub fn parse_stat(text: &str) -> Option<CpuSample> {
    let mut sample = CpuSample::default();
    let mut have_total = false;
    for line in text.lines() {
        let name = line.split_whitespace().next().unwrap_or("");
        if name == "cpu" {
            sample.total = parse_line(line)?;
            have_total = true;
        } else if let Some(id) = name.strip_prefix("cpu").and_then(|n| n.parse::<u32>().ok()) {
            sample.cores.push((id, parse_line(line)?));
        }
    }
    have_total.then_some(sample)
}

pub fn read_sample() -> Option<CpuSample> {
    parse_stat(&std::fs::read_to_string("/proc/stat").ok()?)
}

/// Busy fraction between two readings, from 0.0 to 1.0.
pub fn usage(previous: CpuTimes, now: CpuTimes) -> f64 {
    let total = now.total.saturating_sub(previous.total);
    if total == 0 {
        return 0.0;
    }
    (now.busy.saturating_sub(previous.busy) as f64 / total as f64).clamp(0.0, 1.0)
}

/// Number of CPU threads and model name from `/proc/cpuinfo`.
pub fn parse_model(cpuinfo: &str) -> Option<String> {
    cpuinfo
        .lines()
        .find(|l| l.starts_with("model name"))
        .and_then(|l| l.split_once(':'))
        .map(|(_, v)| v.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const STAT: &str = "cpu  100 0 50 800 50 0 0 0 0 0\ncpu0 40 0 20 400 40 0 0 0 0 0\ncpu1 60 0 30 400 10 0 0 0 0 0\nintr 12345\nctxt 9\n";

    #[test]
    fn parses_total_and_cores() {
        let s = parse_stat(STAT).unwrap();
        assert_eq!(s.cores.len(), 2);
        assert_eq!(
            s.total,
            CpuTimes {
                busy: 150,
                total: 1000
            }
        );
        assert_eq!(
            s.cores[0],
            (
                0,
                CpuTimes {
                    busy: 60,
                    total: 500
                }
            )
        );
        assert_eq!(s.cores[1].0, 1);
    }

    #[test]
    fn cores_keep_their_number_when_one_is_offline() {
        // cpu1 is offline: the lines left are cpu0 and cpu2, and cpu2 must not read as cpu1.
        let s = parse_stat("cpu 1 0 0 1 0 0 0 0\ncpu0 1 0 0 1\ncpu2 4 0 0 4\n").unwrap();
        let ids: Vec<u32> = s.cores.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, vec![0, 2]);
    }

    #[test]
    fn rejects_text_without_the_total_line() {
        assert!(parse_stat("cpu0 1 2 3 4\n").is_none());
        assert!(parse_stat("").is_none());
    }

    #[test]
    fn rejects_garbage_numbers() {
        assert!(parse_stat("cpu a b c d\n").is_none());
    }

    #[test]
    fn usage_is_the_busy_share_of_the_interval() {
        let a = CpuTimes {
            busy: 100,
            total: 1000,
        };
        let b = CpuTimes {
            busy: 150,
            total: 1100,
        };
        assert!((usage(a, b) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn usage_survives_no_progress_and_counter_resets() {
        let a = CpuTimes {
            busy: 100,
            total: 1000,
        };
        assert_eq!(usage(a, a), 0.0);
        assert_eq!(
            usage(
                a,
                CpuTimes {
                    busy: 10,
                    total: 500
                }
            ),
            0.0
        );
    }

    #[test]
    fn reads_the_model_name() {
        assert_eq!(
            parse_model("processor\t: 0\nmodel name\t: Ryzen 7 5800X 8-Core\n").as_deref(),
            Some("Ryzen 7 5800X 8-Core")
        );
        assert_eq!(parse_model("processor : 0\n"), None);
    }
}
