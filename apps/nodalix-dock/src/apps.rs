use std::process::Command;

#[derive(Debug, Clone)]
pub struct PinnedApp {
    pub name: &'static str,
    pub icon: &'static str,
    pub command: &'static str,
    pub class_hint: &'static str,
}

pub fn pinned() -> Vec<PinnedApp> {
    vec![
        PinnedApp {
            name: "Browser",
            icon: "󰖟",
            command: "xdg-open https://",
            class_hint: "browser",
        },
        PinnedApp {
            name: "Files",
            icon: "󰉋",
            command: "nodalix-files",
            class_hint: "nodalix-files",
        },
        PinnedApp {
            name: "Terminal",
            icon: "󰆍",
            command: "kitty",
            class_hint: "kitty",
        },
        PinnedApp {
            name: "Settings",
            icon: "󰒓",
            command: "nodalix-settings",
            class_hint: "nodalix-settings",
        },
        PinnedApp {
            name: "Command",
            icon: "󰘳",
            command: "nodalix-command-bar",
            class_hint: "nodalix-command-bar",
        },
    ]
}

pub fn launch(command: &str) {
    let mut parts = command.split_whitespace();
    let Some(program) = parts.next() else {
        return;
    };
    let args: Vec<&str> = parts.collect();
    let _ = Command::new(program).args(args).spawn();
}
