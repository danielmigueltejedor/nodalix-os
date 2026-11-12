"""Archivador: Nautilus with the NDL layer. Pinned inputs, loud anchors, opt-in package."""
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ARCH = ROOT / 'gnome/archivador'
NDL = ROOT / 'gnome/control-center/ndl'
PKGBUILD = ROOT / 'packaging/nodalix-archivador/PKGBUILD'

# The upstream text apply.py anchors on, reduced to what the anchors need.
FIXTURE = {
    'src/nautilus-application.c': '#include "nautilus-application.h"\n\nstatic void\nstartup (void)\n{\n'
                                  '    gtk_window_set_default_icon_name (APPLICATION_ID);\n}\n',
    'src/meson.build': "libnautilus_sources = [\n  'nautilus-application.c',\n  'nautilus-application.h',\n]\n",
    'src/resources/nautilus.gresource.xml.in': '<gresource>\n    <file>style.css</file>\n\n    <!-- Icons -->\n</gresource>\n',
    'src/resources/ui/nautilus-window.blp': (
        '        child: Adw.OverlaySplitView split_view {\n          enable-show-gesture: false;\n'
        '          sidebar: Adw.ToolbarView {\n            reveal-bottom-bars: bind x;\n'
        '              title-widget: Adw.WindowTitle {\n                title: _("Files");\n              };\n'
        '          content: Adw.ToolbarView {\n            [top]\n            $NautilusToolbar toolbar {\n'),
    'data/org.gnome.Nautilus.desktop.in.in': '[Desktop Entry]\nName=Files\nComment=x\n',
    'data/org.gnome.Nautilus.metainfo.xml.in.in': '<component>\n  <name>Files</name>\n</component>\n',
}


def run_apply(tree):
    return subprocess.run([sys.executable, '-I', str(ARCH / 'apply.py')], cwd=tree,
                          capture_output=True, text=True)


class ArchivadorTests(unittest.TestCase):
    def make_tree(self):
        tree = Path(tempfile.mkdtemp())
        self.addCleanup(lambda: __import__('shutil').rmtree(tree, ignore_errors=True))
        for relative, text in FIXTURE.items():
            path = tree / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)
        return tree

    def test_layer_registers_sheets_loader_islands_and_name(self):
        tree = self.make_tree()
        result = run_apply(tree)
        self.assertEqual(result.returncode, 0, result.stderr)
        xml = (tree / 'src/resources/nautilus.gresource.xml.in').read_text()
        for name in ('nodalia-light.css', 'nodalia-dark-hc.css', 'nodalia-gtk.css', 'nodalix-overrides.css'):
            self.assertIn(f'<file>ndl/{name}</file>', xml)
            self.assertTrue((tree / 'src/resources/ndl' / name).is_file())
        self.assertIn('nautilus_nodalix_theme_init ();', (tree / 'src/nautilus-application.c').read_text())
        self.assertIn("'nautilus-nodalix-theme.c',", (tree / 'src/meson.build').read_text())
        blueprint = (tree / 'src/resources/ui/nautilus-window.blp').read_text()
        self.assertEqual(blueprint.count('"ndl-island"'), 2)
        self.assertIn('"ndl-split"', blueprint)
        self.assertIn('Name=Archivador', (tree / 'data/org.gnome.Nautilus.desktop.in.in').read_text())

    def test_sheets_are_the_pinned_settings_copies(self):
        tree = self.make_tree()
        self.assertEqual(run_apply(tree).returncode, 0)
        for name in ('nodalia-light.css', 'nodalia-gtk.css', 'nodalia-metrics-regular.css'):
            self.assertEqual((tree / 'src/resources/ndl' / name).read_bytes(), (NDL / name).read_bytes())

    def test_missing_upstream_anchor_fails_loudly(self):
        tree = self.make_tree()
        path = tree / 'src/resources/ui/nautilus-window.blp'
        path.write_text(path.read_text().replace('Adw.OverlaySplitView', 'Adw.Other'))
        result = run_apply(tree)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('anchor must match exactly once', result.stderr)

    def test_every_sheet_the_loader_uses_is_registered(self):
        loader = (ARCH / 'nautilus-nodalix-theme.c').read_text()
        used = set(re.findall(r'"(nodalia-[a-z-]+\.css|nodalix-overrides\.css)"', loader))
        registered = set(re.findall(r"'(nodalia-[a-z-]+\.css)'", (ARCH / 'apply.py').read_text())) | {'nodalix-overrides.css'}
        self.assertTrue(used)
        self.assertLessEqual(used, registered)
        for name in registered - {'nodalix-overrides.css'}:
            self.assertTrue((NDL / name).is_file(), name)
        self.assertTrue((ARCH / 'nodalix-overrides.css').is_file())
        self.assertIn('/org/gnome/nautilus/ndl/', loader)

    def test_loader_can_be_switched_off_and_follows_the_system_style(self):
        loader = (ARCH / 'nautilus-nodalix-theme.c').read_text()
        self.assertIn('NODALIX_NDL', loader)
        self.assertIn('notify::dark', loader)
        self.assertIn('notify::high-contrast', loader)

    def test_overrides_keep_the_system_font_and_use_only_gtk_properties(self):
        css = re.sub(r'/\*.*?\*/', '', (ARCH / 'nodalix-overrides.css').read_text(), flags=re.S)
        self.assertIn('--ndl-font-family-ui: "Adwaita Sans"', css)
        self.assertNotIn('Inter', css)
        self.assertNotRegex(css, r'(?m)^\s*overflow\s*:')

    def test_sources_are_pinned_and_libgxdp_matches_the_nautilus_wrap(self):
        tool = (ROOT / 'tools/build-archivador-package.py').read_text()
        self.assertRegex(tool, r"NAUTILUS_REV = '[0-9a-f]{40}'")
        self.assertRegex(tool, r"LIBGXDP_REV = '[0-9a-f]{40}'")
        self.assertIn('Nautilus pins a different libgxdp', tool)

    def test_package_is_a_drop_in_for_nautilus_but_opt_in(self):
        text = PKGBUILD.read_text()
        self.assertIn('provides=("nautilus=51.1")', text)
        self.assertIn('conflicts=("nautilus")', text)
        # Without `replaces`, a system update never swaps the file manager on its own.
        self.assertNotIn('replaces=', text)
        self.assertIn('pkgver=51.1.0', text)
        # Arch's stack does not ship SELinux: meson fails if the feature is enabled and libselinux is absent.
        self.assertIn('-Dselinux=disabled', text)
        self.assertNotIn('libselinux', text)

    def test_release_script_builds_it(self):
        script = (ROOT / 'tools/build-packages.sh').read_text()
        self.assertIn('build-archivador-package.py', script)


if __name__ == '__main__':
    unittest.main()
