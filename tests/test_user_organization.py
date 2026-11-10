import importlib.util
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
def script(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'integrations/scripts' / ('nodalix-' + name + '.py'))
    module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
    return module
layout, icons = script('user-layout'), script('app-icons')

class Organization(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.home = Path(self.temp.name)
        env = patch.dict(os.environ, {}, clear=True); env.start(); self.addCleanup(env.stop)
    def test_language_roundtrip_preserves_content_and_custom_paths(self):
        (self.home/'Documents').mkdir(); (self.home/'Documents/report.txt').write_text('important')
        (self.home/'Documents/link').symlink_to('report.txt')
        (self.home/'vinilo/.git').mkdir(parents=True)
        (self.home/'Desktop').mkdir()
        (self.home/'iphonebridge_pb_empty').mkdir()
        (self.home/'personal').mkdir()
        layout.reconcile(self.home, 'es', True)
        self.assertEqual((self.home/'Documentos/report.txt').read_text(), 'important')
        self.assertTrue((self.home/'Documentos/Proyectos/vinilo/.git').is_dir())
        self.assertFalse((self.home/'Desktop').exists())
        self.assertTrue((self.home/'personal').is_dir())
        self.assertEqual(layout.reconcile(self.home,'es')['moves'], [])
        layout.reconcile(self.home,'en',True)
        layout.reconcile(self.home,'en',True)
        self.assertEqual((self.home/'Documents/link').read_text(),'important')
        self.assertTrue((self.home/'Documents/Projects/vinilo/.git').is_dir())
    def test_appimages_follow_language_without_breaking_old_launch_paths(self):
        apps = self.home / 'Applications'
        apps.mkdir()
        image = apps / 'Audacity.AppImage'
        image.write_bytes(b'appimage')
        image.chmod(0o755)
        layout.reconcile(self.home, 'es', True)
        self.assertTrue(apps.is_symlink())
        self.assertEqual((self.home/'Aplicaciones/Audacity.AppImage').read_bytes(), b'appimage')
        self.assertTrue((self.home/'Aplicaciones/Audacity.AppImage').stat().st_mode & 0o111)
        self.assertIn('Applications', (self.home/'.hidden').read_text().splitlines())
        layout.reconcile(self.home, 'en', True)
        self.assertFalse(apps.is_symlink())
        self.assertEqual((apps/'Audacity.AppImage').read_bytes(), b'appimage')
        layout.reconcile(self.home, 'en', True)
        self.assertEqual(layout.reconcile(self.home, 'en')['moves'], [])

    def test_conflict_stops_before_changes(self):
        for name in ('Documents','Documentos'):
            (self.home/name).mkdir(); (self.home/name/'keep').write_text(name)
        with self.assertRaises(ValueError): layout.reconcile(self.home,'es',True)
        self.assertEqual((self.home/'Documents/keep').read_text(),'Documents')
        self.assertEqual((self.home/'Documentos/keep').read_text(),'Documentos')
    def test_custom_document_path_and_git_worktree_are_preserved(self):
        (self.home/'.config').mkdir()
        (self.home/'.config/user-dirs.dirs').write_text('XDG_DOCUMENTS_DIR="$HOME/Work"\n')
        (self.home/'worktree').mkdir(); (self.home/'worktree/.git').write_text('gitdir: /elsewhere')
        report=layout.reconcile(self.home,'es',True)
        self.assertEqual(report['paths']['DOCUMENTS'],str(self.home/'Work'))
        self.assertTrue((self.home/'worktree/.git').exists())
    def test_icons_preserve_launch_and_actions_and_follow_uninstall(self):
        system=self.home/'system'; apps=system/'applications'; apps.mkdir(parents=True)
        theme=self.home/'themes/Colloid-Teal/apps/scalable'; theme.mkdir(parents=True)
        (theme/'matlab.svg').write_text('<svg/>')
        text='[Desktop Entry]\nType=Application\nName=MATLAB R2026a\nName[es]=MATLAB\nExec=matlab -desktop %F\nIcon=/opt/matlab.png\n\n[Desktop Action test]\nIcon=other\nExec=matlab -test\n'
        source=apps/'matlab.desktop'; source.write_text(text)
        sync=lambda: icons.synchronize(self.home,[system],[self.home/'themes'])
        sync(); target=self.home/'.local/share/applications/matlab.desktop'
        self.assertIn('Exec=matlab -desktop %F',target.read_text())
        self.assertIn('[Desktop Action test]\nIcon=other',target.read_text())
        state=self.home/'.local/state/nodalix/icons'
        manifests=[state/'overrides.json',state/'resolved.json']
        timestamps=[path.stat().st_mtime_ns for path in manifests]
        sync()
        self.assertEqual([path.stat().st_mtime_ns for path in manifests],timestamps)
        source.unlink(); sync(); self.assertFalse(target.exists())
    def test_user_icon_restored_when_theme_removed(self):
        apps=self.home/'.local/share/applications'; apps.mkdir(parents=True)
        theme=self.home/'themes/Colloid-Teal/apps/scalable'; theme.mkdir(parents=True)
        icon=theme/'matlab.svg'; icon.write_text('<svg/>')
        target=apps/'matlab.desktop'; original='[Desktop Entry]\nType=Application\nName=MATLAB\nExec=matlab\nIcon=matlab\n'; target.write_text(original)
        sync=lambda: icons.synchronize(self.home,[],[self.home/'themes'])
        sync(); icon.unlink(); sync(); self.assertEqual(target.read_text(),original)

    def test_current_and_future_craft_icons_use_supplied_assets(self):
        apps=self.home/'.local/share/applications';apps.mkdir(parents=True)
        images=self.home/'.local/share/icons/hicolor/128x128/apps';images.mkdir(parents=True)
        names=['ai.storyteller.texturecraft','ai.storyteller.craft-anything',
               'ai.storyteller.figcraft','io.tangencylabs.solidcraft','com.storytold.future-tool']
        for name in names:
            (images/(name+'.png')).write_bytes(b'official icon')
            (apps/(name+'.desktop')).write_text('[Desktop Entry]\nType=Application\nName='+name+
                '\nExec=/usr/bin/true %F\nIcon='+name+'\n[Desktop Action test]\nIcon=personal-action\nExec=/usr/bin/true\n')
        sync=lambda:icons.synchronize(self.home,[],[])
        chosen=sync()
        for name in names:
            self.assertEqual(chosen[name+'.desktop'],str(images/(name+'.png')))
            text=(apps/(name+'.desktop')).read_text()
            self.assertIn('Exec=/usr/bin/true %F',text)
            self.assertIn('[Desktop Action test]\nIcon=personal-action',text)
        self.assertEqual(sync(),chosen)
        custom=apps/(names[0]+'.desktop');text=custom.read_text().replace('Icon='+chosen[custom.name],'Icon=/personal/my-icon.svg');custom.write_text(text)
        sync();self.assertEqual(custom.read_text(),text)

    def test_packaged_craft_artwork_precedes_old_theme_and_official_fallback(self):
        system = self.home / 'system'
        packaged = system / 'icons/Nodalix-Icons/apps/scalable'
        packaged.mkdir(parents=True)
        identifier = 'ai.storyteller.figcraft'
        expected = packaged / (identifier + '.svg')
        expected.write_text('<svg>nodalix</svg>')
        # Older Colloid installs remain searchable but must not win.
        local = self.home / 'themes/Colloid-Teal/apps/scalable'
        local.mkdir(parents=True)
        (local / expected.name).write_text('<svg>old theme</svg>')
        catalogue = system / 'nodalix/icon-theme/legacy-icons.json'
        catalogue.parent.mkdir(parents=True)
        catalogue.write_text(json.dumps({'schema_version': 1, 'icons': [
            {'icon': identifier, 'sha256': '0' * 64}]}))
        official = self.home / '.local/share/icons/hicolor/128x128/apps'
        official.mkdir(parents=True)
        (official / (identifier + '.png')).write_bytes(b'official')
        apps = self.home / '.local/share/applications'
        apps.mkdir(parents=True)
        entry = apps / (identifier + '.desktop')
        entry.write_text('[Desktop Entry]\nType=Application\nName=FigCraft\nExec=figcraft %F\nIcon=' + identifier + '\n')
        sync = lambda: icons.synchronize(self.home, [system], [self.home / 'themes'])
        self.assertEqual(sync()[entry.name], str(expected))
        # An icon added later to the repository needs no OS-side app list.
        (packaged / 'futureapp.svg').write_text('<svg>new repository icon</svg>')
        (local / 'futureapp.svg').write_text('<svg>stale local theme</svg>')
        future = apps / 'futureapp.desktop'
        future.write_text('[Desktop Entry]\nType=Application\nName=Future App\nExec=futureapp\nIcon=futureapp\n')
        self.assertEqual(sync()[future.name], str(packaged / 'futureapp.svg'))
        personal = self.home / 'my-icon.svg'
        personal.write_text('<svg>personal</svg>')
        text = entry.read_text().replace(str(expected), str(personal))
        entry.write_text(text)
        self.assertNotIn(entry.name, sync())
        self.assertEqual(entry.read_text(), text)

    def test_legacy_portable_icon_is_adopted_and_later_personal_edits_are_preserved(self):
        system = self.home / 'system'
        packaged = system / 'icons/Nodalix-Icons/apps/scalable'
        packaged.mkdir(parents=True)
        expected = packaged / 'audacity4.svg'
        expected.write_text('<svg>maintained audacity</svg>')
        legacy = self.home / 'icons/audacity4-colloid-512.png'
        legacy.parent.mkdir()
        legacy.write_bytes(b'original generated icon')
        catalogue = system / 'nodalix/icon-theme/legacy-icons.json'
        catalogue.parent.mkdir(parents=True)
        catalogue.write_text(json.dumps({'schema_version': 1, 'icons': [{
            'icon': 'audacity4', 'sha256': hashlib.sha256(legacy.read_bytes()).hexdigest()}]}))
        apps = self.home / '.local/share/applications'
        apps.mkdir(parents=True)
        entry = apps / 'org.audacityteam.Audacity4portable.desktop'
        original = '[Desktop Entry]\nType=Application\nName=Audacity 4 Portable\nExec=audacity4 %F\nIcon=' + str(legacy) + '\n'
        entry.write_text(original)
        sync = lambda: icons.synchronize(self.home, [system], [])
        self.assertEqual(sync()[entry.name], str(expected))
        self.assertIn('Exec=audacity4 %F', entry.read_text())
        self.assertEqual(sync()[entry.name], str(expected))
        legacy.write_bytes(b'new personal artwork')
        self.assertNotIn(entry.name, sync())
        self.assertEqual(entry.read_text(), original)
        self.assertEqual(legacy.read_bytes(), b'new personal artwork')

    def test_legacy_icon_recognition_reads_only_bounded_regular_files(self):
        original = self.home / 'original.svg'
        original.write_bytes(b'known artwork')
        known = {hashlib.sha256(original.read_bytes()).hexdigest(): 'known-icon'}
        self.assertEqual(icons.legacy_icon_name(original, known), 'known-icon')
        symlink = self.home / 'link.svg'
        symlink.symlink_to(original)
        pipe = self.home / 'pipe.svg'
        os.mkfifo(pipe)
        large = self.home / 'large.svg'
        with large.open('wb') as stream:
            stream.truncate(2 * 1024 * 1024 + 1)
        for path in [symlink, pipe, large, self.home / 'missing.svg']:
            with self.subTest(path=path.name):
                self.assertIsNone(icons.legacy_icon_name(path, known))

    def test_settings_panels_restore_managed_overrides_and_stay_symbolic(self):
        import json
        system=self.home/'system'; apps=system/'applications'; apps.mkdir(parents=True)
        theme=self.home/'themes/Colloid-Teal/apps/scalable'; theme.mkdir(parents=True)
        (theme/'keyboard.svg').write_text('<svg/>')
        source=apps/'gnome-keyboard-panel.desktop'
        source.write_text('[Desktop Entry]\nType=Application\nName=Keyboard\nExec=gnome-control-center keyboard\nIcon=org.gnome.Settings-keyboard-symbolic\nCategories=X-GNOME-Settings-Panel;\n')
        destination=self.home/'.local/share/applications'/source.name
        destination.parent.mkdir(parents=True)
        old=icons.replace_icon(source.read_text(),str(theme/'keyboard.svg'))
        destination.write_text(old)
        state=self.home/'.local/state/nodalix/icons';state.mkdir(parents=True)
        (state/'overrides.json').write_text(json.dumps({source.name:{'source':str(source),'original':'','written_hash':icons.digest(old)}}))
        sync=lambda: icons.synchronize(self.home,[system],[self.home/'themes'])
        self.assertNotIn(source.name,sync())
        self.assertFalse(destination.exists())
        self.assertNotIn(source.name,sync())
        self.assertFalse(destination.exists())
        self.assertIn('Icon=org.gnome.Settings-keyboard-symbolic',source.read_text())
