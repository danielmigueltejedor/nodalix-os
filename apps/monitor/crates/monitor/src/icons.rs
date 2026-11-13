//! The icon of a process: the one of the application it belongs to, found from its name or
//! command line among the installed applications. The applications are read once and each
//! process name is looked up once, so a refresh never searches again.

use std::cell::RefCell;
use std::collections::HashMap;

use nodalia_gtk::gtk::{gio, prelude::*};

use crate::process_object::Row;

/// A kernel thread or a system helper: no application owns it.
const SYSTEM: &str = "emblem-system-symbolic";
/// A program that is not an installed application.
const PROGRAM: &str = "application-x-executable-symbolic";

/// A shell or other program that lives in a terminal.
const TERMINAL: &str = "utilities-terminal-symbolic";
/// A service: something the system runs on its own, for no one in particular.
const SERVICE: &str = "system-run-symbolic";

/// The icon of a process no application owns, by what kind of program it looks like.
pub fn generic(name: &str, cmdline: &str, user: &str) -> &'static str {
    if cmdline.is_empty() {
        return SYSTEM;
    }
    let program = cmdline
        .split_whitespace()
        .next()
        .and_then(|first| first.rsplit('/').next())
        .unwrap_or(name)
        .trim_start_matches('-');
    if matches!(
        program,
        "bash" | "zsh" | "fish" | "sh" | "dash" | "nu" | "tmux" | "screen" | "sudo" | "ssh"
    ) {
        return TERMINAL;
    }
    // Daemons by convention end in `d` (systemd, dbus-daemon, sshd), and the system's own users
    // run services, not what a person opened.
    if user == "root" || program.ends_with('d') && program.len() > 3 || program.ends_with("daemon")
    {
        return SERVICE;
    }
    PROGRAM
}

/// The names a process may be known by among desktop files, most likely first: the command's
/// file name, then the process name, each also without a `-bin` / `.real` style suffix.
pub fn candidates(name: &str, cmdline: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |text: &str| {
        let text = text.trim().to_lowercase();
        if text.is_empty() || out.contains(&text) {
            return;
        }
        for suffix in ["-bin", ".real", "-wrapped", ".bin"] {
            if let Some(stripped) = text.strip_suffix(suffix) {
                if !stripped.is_empty() && !out.contains(&stripped.to_owned()) {
                    out.push(stripped.to_owned());
                }
            }
        }
        out.push(text);
    };
    // The first word of the command line, without its folders; an interpreter runs a script, so
    // the script's name is the second word then.
    let mut words = cmdline.split_whitespace();
    if let Some(first) = words.next() {
        let program = first.rsplit('/').next().unwrap_or(first);
        let interpreter = program.starts_with("python")
            || matches!(program, "sh" | "bash" | "node" | "perl" | "ruby" | "java");
        if interpreter {
            if let Some(script) = words.find(|w| !w.starts_with('-')) {
                push(script.rsplit('/').next().unwrap_or(script));
            }
        }
        push(program);
    }
    push(name);
    out
}

thread_local! {
    static APPS: RefCell<Option<HashMap<String, gio::Icon>>> = const { RefCell::new(None) };
    static FOUND: RefCell<HashMap<String, gio::Icon>> = RefCell::new(HashMap::new());
}

/// Keys an application can be found by: its name, its desktop id (the last part of a
/// reverse-domain one, then the whole) and its command's file name.
fn keys_for(name: &str, id: Option<&str>, exe: Option<&std::path::Path>) -> Vec<String> {
    let mut keys = vec![name.to_lowercase()];
    if let Some(id) = id {
        let id = id.trim_end_matches(".desktop").to_lowercase();
        if let Some(last) = id.rsplit('.').next() {
            keys.push(last.to_owned());
        }
        keys.push(id);
    }
    if let Some(file) = exe.and_then(|e| e.file_name()).and_then(|n| n.to_str()) {
        keys.push(file.to_lowercase());
    }
    keys
}

/// Every installed application by those keys.
fn index() -> HashMap<String, gio::Icon> {
    let mut map = HashMap::new();
    for app in gio::AppInfo::all() {
        let Some(icon) = app.icon() else { continue };
        // An entry without an `Exec` line has no executable: asking for it returns NULL, which
        // the bindings turn into a crash. Such an entry cannot own a process anyway.
        if app.commandline().is_none() {
            continue;
        }
        let id = app.id();
        let exe = app.executable();
        for key in keys_for(&app.name(), id.as_deref(), Some(&exe)) {
            map.entry(key).or_insert_with(|| icon.clone());
        }
    }
    map
}

/// The icon for a row; the same one every time for the same process name.
pub fn for_row(row: &Row) -> gio::Icon {
    let cached = FOUND.with(|f| f.borrow().get(&row.name).cloned());
    if let Some(icon) = cached {
        return icon;
    }
    let icon = APPS.with(|apps| {
        let mut apps = apps.borrow_mut();
        let apps = apps.get_or_insert_with(index);
        candidates(&row.name, &row.cmdline)
            .iter()
            .find_map(|c| apps.get(c).cloned())
    });
    let icon = icon.unwrap_or_else(|| {
        gio::ThemedIcon::new(generic(&row.name, &row.cmdline, &row.user)).upcast()
    });
    FOUND.with(|f| f.borrow_mut().insert(row.name.clone(), icon.clone()));
    icon
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_command_comes_before_the_truncated_process_name() {
        let c = candidates("firefox-bi", "/usr/lib/firefox/firefox-bin --new-window");
        assert_eq!(c[0], "firefox");
        assert!(c.contains(&"firefox-bin".to_owned()));
        assert!(c.contains(&"firefox-bi".to_owned()));
    }

    #[test]
    fn a_script_is_named_by_the_script_not_the_interpreter() {
        let c = candidates(
            "python3",
            "/usr/bin/python3 -u /usr/bin/nodalix-updater --x",
        );
        assert_eq!(c[0], "nodalix-updater");
    }

    #[test]
    fn a_process_without_an_application_gets_the_icon_of_its_kind() {
        assert_eq!(generic("kworker/0:1", "", "root"), SYSTEM);
        assert_eq!(generic("bash", "-bash", "daniel"), TERMINAL);
        assert_eq!(generic("sshd", "/usr/bin/sshd -D", "daniel"), SERVICE);
        assert_eq!(generic("Xorg", "/usr/lib/Xorg", "root"), SERVICE);
        assert_eq!(generic("mytool", "/opt/mytool/mytool", "daniel"), PROGRAM);
        // A word that merely ends in `d` and is short is a program, not a daemon.
        assert_eq!(generic("od", "/usr/bin/od", "daniel"), PROGRAM);
    }

    #[test]
    fn a_kernel_thread_has_only_its_name() {
        assert_eq!(candidates("kworker/0:1", ""), ["kworker/0:1"]);
    }

    #[test]
    fn an_application_is_found_by_its_name_id_and_command() {
        let keys = keys_for(
            "Brújula",
            Some("os.nodalix.Brujula.desktop"),
            Some(std::path::Path::new("/usr/bin/nodalix-brujula")),
        );
        assert_eq!(
            keys,
            [
                "brújula",
                "brujula",
                "os.nodalix.brujula",
                "nodalix-brujula"
            ]
        );
    }

    #[test]
    fn an_application_without_an_id_or_command_has_only_its_name() {
        assert_eq!(keys_for("Tool", None, None), ["tool"]);
    }
}
