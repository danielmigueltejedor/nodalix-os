//! Archivador as the desktop's file manager, on a private session bus: D-Bus starts it for a
//! `org.freedesktop.FileManager1` call, the way browsers and the OpenURI portal reach it, and the
//! command line Nautilus took reaches the running copy.
//!
//! It needs a display and the D-Bus tools (`dbus-run-session`, `gdbus`); without them it says so
//! and passes, as the tests that draw do.

use std::path::{Path, PathBuf};
use std::process::Command;

fn have(program: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {program}")])
        .output()
        .is_ok_and(|o| o.status.success())
}

/// A `file://` URI with everything but the plainest characters escaped.
fn uri(path: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    let mut out = String::from("file://");
    for &b in path.as_os_str().as_bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[test]
fn show_in_folder_starts_archivador_and_selects_the_file() {
    let display = std::env::var_os("DISPLAY").or_else(|| std::env::var_os("WAYLAND_DISPLAY"));
    if display.is_none()
        || !["dbus-run-session", "dbus-daemon", "gdbus"]
            .iter()
            .all(|p| have(p))
    {
        eprintln!("skipped: needs a display, dbus-run-session and gdbus");
        return;
    }
    let root: PathBuf = std::env::temp_dir().join(format!("archivador-fm1-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let home = root.join("home");
    let folder = home.join("Descargas con espacio");
    let file = folder.join("café #1.pdf");
    let other = home.join("otra");
    let services = root.join("services");
    for dir in [&folder, &other, &services] {
        std::fs::create_dir_all(dir).unwrap();
    }
    std::fs::write(&file, b"%PDF-1.4\n").unwrap();
    let selected_by_option = other.join("nota.txt");
    std::fs::write(&selected_by_option, b"hola").unwrap();
    let trace = root.join("trace");
    let bin = env!("CARGO_BIN_EXE_nodalix-archivador");

    // What the package installs, pointed at the binary under test.
    std::fs::write(
        services.join("org.freedesktop.FileManager1.service"),
        format!("[D-BUS Service]\nName=org.freedesktop.FileManager1\nExec={bin} --gapplication-service\n"),
    )
    .unwrap();
    let config = root.join("session.conf");
    std::fs::write(
        &config,
        format!(
            r#"<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN" "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <listen>unix:tmpdir={tmp}</listen>
  <servicedir>{services}</servicedir>
  <policy context="default">
    <allow send_destination="*" eavesdrop="true"/>
    <allow eavesdrop="true"/>
    <allow own="*"/>
  </policy>
</busconfig>
"#,
            tmp = root.display(),
            services = services.display(),
        ),
    )
    .unwrap();

    let script = r#"
set -u
call() { gdbus call --session --dest org.freedesktop.FileManager1 --object-path /org/freedesktop/FileManager1 --method "org.freedesktop.FileManager1.$1" "$2" ""; }
owned() { gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus --method org.freedesktop.DBus.NameHasOwner org.freedesktop.FileManager1; }
# Nothing runs yet: the call starts Archivador through the service file.
owned | grep -q false || { echo "already owned"; exit 2; }
call ShowItems "['$FILE_URI']" || exit 3
call ShowFolders "['$OTHER_URI']" || exit 4
call ShowItemProperties "['$FILE_URI']" || exit 9
# A bad URI is an error, not a crash.
call ShowItems "['https://example.org/x']" 2>/dev/null && { echo "bad URI accepted"; exit 5; }
# The command line reaches the copy that runs.
"$BIN" --select "$SELECT" || exit 6
sleep 2
"$BIN" --quit || exit 7
for i in 1 2 3 4 5 6 7 8 9 10; do owned | grep -q false && exit 0; sleep 0.5; done
echo "still running after --quit"; exit 8
"#;
    let output = Command::new("dbus-run-session")
        .arg(format!("--config-file={}", config.display()))
        .args(["--", "sh", "-c", script])
        .env("BIN", bin)
        .env("FILE_URI", uri(&file))
        .env("OTHER_URI", uri(&other))
        .env("SELECT", &selected_by_option)
        .env("NODALIX_ARCHIVADOR_TRACE", &trace)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("XDG_DATA_HOME", home.join(".local/share"))
        .env("XDG_CACHE_HOME", home.join(".cache"))
        .env("GTK_A11Y", "none")
        .env("GSK_RENDERER", "cairo")
        .output()
        .expect("dbus-run-session runs");
    let lines = std::fs::read_to_string(&trace).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&root);
    assert!(
        output.status.success(),
        "exit {:?}\n{}\n{}\ntrace:\n{lines}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let lines: Vec<&str> = lines.lines().collect();
    let expected = [
        format!("show\t{}\t{}", folder.display(), file.display()),
        format!("show\t{}\t", other.display()),
        format!("properties\t{}\t{}", folder.display(), file.display()),
        format!("show\t{}\t{}", folder.display(), file.display()),
        format!(
            "show\t{}\t{}",
            other.display(),
            selected_by_option.display()
        ),
    ];
    assert_eq!(lines, expected);
}
