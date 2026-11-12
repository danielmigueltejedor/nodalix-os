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

/// Keys an application can be found by: its command, its desktop id (whole and last part of a
/// reverse-domain one) and its name.
fn index() -> HashMap<String, gio::Icon> {
    let mut map = HashMap::new();
    for app in gio::AppInfo::all() {
        let Some(icon) = app.icon() else { continue };
        let mut keys = vec![app.name().to_lowercase()];
        if let Some(id) = app.id() {
            let id = id.trim_end_matches(".desktop").to_lowercase();
            if let Some(last) = id.rsplit('.').next() {
                keys.push(last.to_owned());
            }
            keys.push(id);
        }
        let exe = app.executable();
        if let Some(file) = exe.file_name().and_then(|n| n.to_str()) {
            keys.push(file.to_lowercase());
        }
        for key in keys {
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
        gio::ThemedIcon::new(if row.cmdline.is_empty() {
            SYSTEM
        } else {
            PROGRAM
        })
        .upcast()
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
    fn a_kernel_thread_has_only_its_name() {
        assert_eq!(candidates("kworker/0:1", ""), ["kworker/0:1"]);
    }
}
