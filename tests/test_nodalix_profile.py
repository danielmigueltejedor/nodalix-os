"""The packaged Nodalix Shell desktop profile and its non-destructive migration."""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import unittest

from gi.repository import Gio, GLib

ROOT = Path(__file__).resolve().parents[1]
PROFILE = ROOT / 'gnome/nodalix-shell/profile.json'
HELPER = ROOT / 'gnome/migration/nodalix_profile.py'
# Pinned upstream module that is unpacked at package build time and checked there.
UNPACKED_AT_BUILD = {'nowplaying@epogonii.github.io'}


class ProfileContentTests(unittest.TestCase):
    def setUp(self):
        self.profile = json.loads(PROFILE.read_text())

    def test_rows_match_packaged_schemas(self):
        with tempfile.TemporaryDirectory() as scratch:
            for row in self.profile['extensions']:
                if row['uuid'] in UNPACKED_AT_BUILD:
                    continue
                directory = Path(scratch) / row['uuid']
                directory.mkdir(exist_ok=True)
                for source in (ROOT / 'gnome/extensions' / row['uuid'] / 'schemas').glob('*.xml'):
                    shutil.copy2(source, directory)
                subprocess.run(['glib-compile-schemas', str(directory)], check=True)
                source = Gio.SettingsSchemaSource.new_from_directory(
                    str(directory), Gio.SettingsSchemaSource.get_default(), False)
                schema = source.lookup(row['schema'], False)
                self.assertIsNotNone(schema, row['schema'])
                for key, text in row['values'].items():
                    self.assertTrue(schema.has_key(key), f"{row['schema']} {key}")
                    value = GLib.Variant.parse(None, text, None, None)
                    definition = schema.get_key(key)
                    self.assertEqual(value.get_type_string(), definition.get_value_type().dup_string(), key)
                    self.assertTrue(definition.range_check(value), key)
                    self.assertFalse(value.equal(definition.get_default_value()),
                                     f"{key} repeats the schema default")

    def test_every_profile_module_ships_with_nodalix_shell(self):
        registry = json.loads((ROOT / 'gnome/nodalix-shell/modules.json').read_text())
        shipped = set(registry['bundled'] + registry['desktopPackages'])
        used = {row['uuid'] for row in self.profile['extensions']} | set(self.profile['enableModules'])
        self.assertLessEqual(used, shipped)

    def test_profile_keeps_machine_and_identity_data_out(self):
        text = PROFILE.read_text()
        self.assertNotRegex(text, r'/home/|fingerprint|monitor-indicator|preferred-monitor-by-connector|download-folder')
        self.assertNotRegex(text, r'\b[0-9A-F]{64}\b')

    def test_shell_package_validates_and_installs_the_profile(self):
        pkgbuild = (ROOT / 'packaging/nodalix-gnome-shell/PKGBUILD').read_text()
        self.assertIn('usr/share/nodalix/nodalix-shell-profile.json', pkgbuild)
        self.assertIn("Invalid profile value", pkgbuild)
        self.assertIn("'python-gobject'", pkgbuild.split('makedepends=(')[1].split(')')[0])
        gnome = (ROOT / 'packaging/nodalix-gnome/PKGBUILD').read_text()
        self.assertIn('usr/lib/nodalix/nodalix_profile.py', gnome)
        revision = lambda text: int(re.search(r'^pkgrel=(\d+)', text, re.M).group(1))
        required = int(re.search(r"nodalix-gnome>=0\.5\.0-(\d+)", pkgbuild).group(1))
        self.assertLessEqual(required, revision(gnome))

    def test_system_defaults_cover_the_desktop_preferences(self):
        override = (ROOT / 'gnome/session/90-nodalix.gschema.override').read_text()
        for line in ("color-scheme='prefer-dark'", "accent-color='yellow'", 'enable-hot-corners=false',
                     "accel-profile='flat'", "close=['<Super>q', '<Alt>F4']", 'workspaces-only-on-primary=false'):
            self.assertIn(line, override)


class ProfileMigrationTests(unittest.TestCase):
    """Run in a subprocess so the in-memory settings backend never touches a real session."""

    SCRIPT = r'''
import json, shutil, subprocess, sys
from pathlib import Path
from gi.repository import Gio
root, helper_path, scratch = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
namespace = {}
exec(compile(helper_path.read_text(), str(helper_path), 'exec'), namespace)
profile_path = root/'gnome/nodalix-shell/profile.json'
profile = json.loads(profile_path.read_text())
modules = scratch/'modules'
for uuid in {row['uuid'] for row in profile['extensions']} - {'nowplaying@epogonii.github.io'}:
    source = root/'gnome/extensions'/uuid
    (modules/uuid/'schemas').mkdir(parents=True)
    shutil.copy2(source/'metadata.json', modules/uuid/'metadata.json')
    for xml in (source/'schemas').glob('*.xml'):
        shutil.copy2(xml, modules/uuid/'schemas')
    subprocess.run(['glib-compile-schemas', str(modules/uuid/'schemas')], check=True)
# Now Playing Card is unpacked from its pinned release archive at package build time.
nowplaying = modules/'nowplaying@epogonii.github.io'
(nowplaying/'schemas').mkdir(parents=True)
(nowplaying/'metadata.json').write_text('{}')
(nowplaying/'schemas/org.gnome.shell.extensions.nowplaying.gschema.xml').write_text(
    '<schemalist><schema id="org.gnome.shell.extensions.nowplaying" path="/org/gnome/shell/extensions/nowplaying/">'
    '<key name="panel-box" type="s"><default>\'right\'</default></key>'
    '<key name="panel-index" type="i"><default>0</default></key></schema></schemalist>')
subprocess.run(['glib-compile-schemas', str(nowplaying/'schemas')], check=True)

def settings(path, name):
    source = Gio.SettingsSchemaSource.new_from_directory(str(path), Gio.SettingsSchemaSource.get_default(), False)
    return Gio.Settings.new_full(source.lookup(name, False), None, None)

tiling = settings(modules/'tiling-assistant@leleat-on-github/schemas', 'org.gnome.shell.extensions.tiling-assistant')
menu = settings(modules/'global-menu@unmade.space/schemas', 'org.gnome.shell.extensions.global-menu')
shell = settings(scratch/'shell', 'org.gnome.shell')
menu.set_int('popup-opacity', 77)  # an explicit choice must survive
roots = (modules,)
assert namespace['apply_profile'](profile_path, roots, shell), 'profile left rows pending'
assert tiling.get_int('window-gap') == 8
assert menu.get_int('popup-opacity') == 77, menu.get_int('popup-opacity')
assert menu.get_int('popup-blur-sigma') == 30
assert shell.get_strv('enabled-extensions') == ['nowplaying@epogonii.github.io']
# A module the user turned off stays off, and a second run changes nothing.
shell.set_strv('enabled-extensions', [])
shell.set_strv('disabled-extensions', ['nowplaying@epogonii.github.io'])
assert namespace['apply_profile'](profile_path, roots, shell)
assert shell.get_strv('enabled-extensions') == []
# An incomplete install reports itself so the migration retries at the next login.
assert not namespace['apply_profile'](profile_path, (scratch/'empty',), shell)
print('PROFILE_MIGRATION_OK')
'''

    def test_profile_fills_unset_values_and_respects_user_choices(self):
        with tempfile.TemporaryDirectory() as scratch:
            scratch = Path(scratch)
            (scratch / 'shell').mkdir()
            # org.gnome.shell is not installed with the test runner; compile the two keys used.
            (scratch / 'shell/org.gnome.shell.gschema.xml').write_text(
                '<schemalist><schema id="org.gnome.shell" path="/org/gnome/shell/">'
                '<key name="enabled-extensions" type="as"><default>[]</default></key>'
                '<key name="disabled-extensions" type="as"><default>[]</default></key>'
                '</schema></schemalist>')
            subprocess.run(['glib-compile-schemas', str(scratch / 'shell')], check=True)
            result = subprocess.run(
                [sys.executable, '-I', '-c', self.SCRIPT, str(ROOT), str(HELPER), str(scratch)],
                capture_output=True, text=True, env=dict(os.environ, GSETTINGS_BACKEND='memory'))
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn('PROFILE_MIGRATION_OK', result.stdout)


if __name__ == '__main__':
    unittest.main()
