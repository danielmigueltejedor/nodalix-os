"""The Settings NDL layer: pinned sheets stay untouched and every registered sheet exists."""
import hashlib
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CC = ROOT / 'gnome/control-center'
NDL = CC / 'ndl'


class NdlThemeTests(unittest.TestCase):
    def test_vendored_sheets_match_their_pinned_checksums(self):
        rows = [line.split() for line in (NDL / 'SHA256SUMS').read_text().splitlines() if line.strip()]
        self.assertGreaterEqual(len(rows), 8)
        for digest, name in rows:
            self.assertEqual(hashlib.sha256((NDL / name).read_bytes()).hexdigest(), digest, name)

    def test_provenance_names_the_pinned_commit(self):
        self.assertRegex((NDL / 'PROVENANCE.md').read_text(), r'commit\s+`[0-9a-f]{40}`')

    def test_every_sheet_the_loader_uses_is_registered_in_the_overlay(self):
        loader = (CC / 'cc-nodalix-theme.c').read_text()
        used = set(re.findall(r'"(nodalia-[a-z-]+\.css|nodalix-overrides\.css)"', loader))
        registered = set(re.findall(r"'((?:nodalia-|nodalix-)[a-z-]+\.css)'", (CC / 'apply.py').read_text()))
        self.assertTrue(used)
        self.assertLessEqual(used, registered)
        for name in registered:
            self.assertTrue((NDL / name).is_file(), name)

    def test_icons_the_widget_sheet_asks_for_are_shipped_and_registered(self):
        sheet = (NDL / 'nodalia-gtk.css').read_text()
        wanted = set(re.findall(r'-gtk-icontheme\("(ndl-[a-z-]+)"\)', sheet))
        shipped = {p.stem for p in (NDL / 'icons').glob('*.svg')}
        registered = set(re.findall(r"'(ndl-[a-z-]+-symbolic)'", (CC / 'apply.py').read_text()))
        self.assertTrue(wanted)
        self.assertLessEqual(wanted, shipped)
        self.assertLessEqual(wanted, registered)
        self.assertIn('add_resource_path', (CC / 'cc-nodalix-theme.c').read_text())

    def test_font_sizes_follow_the_system_text_scale(self):
        # GTK scales pt and em with Large Text, never px.
        css = re.sub(r'/\*.*?\*/', '', (NDL / 'nodalix-overrides.css').read_text(), flags=re.S)
        for token in re.findall(r'--ndl-font-size-[a-z]+:\s*([^;]+);', css):
            self.assertTrue(token.strip().endswith('pt'), token)
        self.assertNotRegex(css, r'font-size:\s*[\d.]+px')

    def test_overrides_keep_the_system_font_and_use_only_gtk_properties(self):
        css = re.sub(r'/\*.*?\*/', '', (NDL / 'nodalix-overrides.css').read_text(), flags=re.S)
        self.assertIn('--ndl-font-family-ui: "Adwaita Sans"', css)
        self.assertIn('--ndl-font-family-display: "Adwaita Sans"', css)
        self.assertNotIn('Inter', css)
        # GTK CSS has no `overflow`: it is a widget property set in the blueprint.
        self.assertNotRegex(css, r'(?m)^\s*overflow\s*:')

    def test_loader_can_be_switched_off_and_follows_the_system_style(self):
        loader = (CC / 'cc-nodalix-theme.c').read_text()
        self.assertIn('NODALIX_NDL', loader)
        self.assertIn('notify::dark', loader)
        self.assertIn('notify::high-contrast', loader)


if __name__ == '__main__':
    unittest.main()
