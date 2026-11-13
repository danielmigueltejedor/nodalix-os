import configparser
import re
import shutil
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
APPS = ROOT / "apps"
APP = APPS / "monitor"
SETTINGS = APPS / "ajustes"
FILES = APPS / "archivador"


class MonitorAppTests(unittest.TestCase):
    def test_nodalia_gtk_is_pinned_to_an_exact_commit(self):
        manifest = (APPS / "Cargo.toml").read_text()
        match = re.search(r'nodalia-gtk\s*=\s*\{[^}]*rev\s*=\s*"([0-9a-f]{40})"', manifest)
        self.assertIsNotNone(match, "nodalia-gtk must be a git dependency with a full commit rev")
        for crate in (APP / "crates/monitor", SETTINGS / "crates/ajustes", FILES / "crates/archivador"):
            self.assertIn("nodalia-gtk.workspace = true", (crate / "Cargo.toml").read_text())

    def test_the_lock_file_is_committed_and_pins_the_same_commit(self):
        manifest = (APPS / "Cargo.toml").read_text()
        rev = re.search(r'rev\s*=\s*"([0-9a-f]{40})"', manifest).group(1)
        self.assertIn(rev, (APPS / "Cargo.lock").read_text())

    def test_the_stylesheets_have_no_colour_or_size_literals_for_the_theme(self):
        sheets = [
            APP / "crates/monitor/src/style.css",
            SETTINGS / "crates/ajustes/src/style.css",
            FILES / "crates/archivador/src/style.css",
            APPS / "crates/nodalix-ui/src/base.css",
        ]
        for sheet in sheets:
            css = sheet.read_text()
            self.assertEqual(re.findall(r"#[0-9a-fA-F]{3,8}\b", css), [], sheet.name)
            self.assertNotRegex(css, r"rgba?\(")
            # Font sizes are pt, which GTK scales with the system's text size; px does not scale.
            self.assertNotRegex(css, r"font-size:\s*[\d.]+px")

    def test_the_shared_stylesheet_uses_the_system_font(self):
        css = (APPS / "crates/nodalix-ui/src/base.css").read_text()
        self.assertIn('--ndl-font-family-ui: "Adwaita Sans"', css)
        self.assertIn('--ndl-font-family-display: "Adwaita Sans"', css)

    def test_desktop_entry(self):
        entry = configparser.ConfigParser(interpolation=None)
        entry.optionxform = str
        entry.read(APP / "data/os.nodalix.Monitor.desktop")
        section = entry["Desktop Entry"]
        self.assertEqual(section["Exec"], "nodalix-monitor")
        self.assertEqual(section["StartupWMClass"], "os.nodalix.Monitor")
        self.assertIn("Name[es]", section)
        self.assertIn("System", section["Categories"])
        self.assertIn("os.nodalix.Monitor", (APP / "crates/monitor/src/main.rs").read_text())

    def test_every_source_string_has_a_spanish_translation(self):
        for src in (APP / "crates/monitor/src", SETTINGS / "crates/ajustes/src"):
            catalog = (src / "i18n.rs").read_text()
            translated = set(re.findall(r'\(\s*"([^"]+)",\s*"[^"]*",?\s*\)', catalog))
            used = set()
            for path in src.rglob("*.rs"):
                if path.name == "i18n.rs":
                    continue
                used |= set(re.findall(r'\btr\(\s*"([^"]+)"', path.read_text()))
            self.assertTrue(used, f"no tr() calls found in {src}")
            self.assertEqual(sorted(used - translated), [], src.name)

    def test_the_settings_app_does_not_build_shell_commands(self):
        # A panel name reaches `gnome-control-center` as one argument, never through a shell.
        source = "\n".join(p.read_text() for p in (SETTINGS / "crates/ajustes/src").rglob("*.rs"))
        self.assertNotRegex(source, r'Command::new\("(sh|bash)"\)')
        self.assertNotIn(".arg(\"-c\")", source)

    @unittest.skipUnless(shutil.which("cargo"), "cargo is not installed")
    def test_core_crate_tests_pass(self):
        result = subprocess.run(
            ["cargo", "test", "-p", "monitor-core", "-p", "ajustes-core"],
            cwd=APPS,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
