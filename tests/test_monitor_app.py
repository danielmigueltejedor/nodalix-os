import configparser
import re
import shutil
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
APP = ROOT / "apps/monitor"


class MonitorAppTests(unittest.TestCase):
    def test_nodalia_gtk_is_pinned_to_an_exact_commit(self):
        manifest = (APP / "crates/monitor/Cargo.toml").read_text()
        match = re.search(r'nodalia-gtk\s*=\s*\{[^}]*rev\s*=\s*"([0-9a-f]{40})"', manifest)
        self.assertIsNotNone(match, "nodalia-gtk must be a git dependency with a full commit rev")

    def test_the_lock_file_is_committed_and_pins_the_same_commit(self):
        manifest = (APP / "crates/monitor/Cargo.toml").read_text()
        rev = re.search(r'rev\s*=\s*"([0-9a-f]{40})"', manifest).group(1)
        self.assertIn(rev, (APP / "Cargo.lock").read_text())

    def test_the_stylesheet_has_no_colour_or_size_literals_for_the_theme(self):
        css = (APP / "crates/monitor/src/style.css").read_text()
        self.assertEqual(re.findall(r"#[0-9a-fA-F]{3,8}\b", css), [])
        self.assertNotRegex(css, r"rgba?\(")
        # Font sizes are pt, which GTK scales with the system's text size; px does not scale.
        self.assertNotRegex(css, r"font-size:\s*[\d.]+px")

    def test_the_stylesheet_uses_the_system_font(self):
        css = (APP / "crates/monitor/src/style.css").read_text()
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
        src = APP / "crates/monitor/src"
        catalog = (src / "i18n.rs").read_text()
        translated = set(re.findall(r'^\s*"([^"]+)"\s*=>', catalog, re.M))
        used = set()
        for path in src.glob("*.rs"):
            if path.name == "i18n.rs":
                continue
            used |= set(re.findall(r'\btr\(\s*"([^"]+)"', path.read_text()))
        self.assertTrue(used, "no tr() calls found")
        self.assertEqual(sorted(used - translated), [])

    @unittest.skipUnless(shutil.which("cargo"), "cargo is not installed")
    def test_core_crate_tests_pass(self):
        result = subprocess.run(
            ["cargo", "test", "-p", "monitor-core"], cwd=APP, capture_output=True, text=True
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
