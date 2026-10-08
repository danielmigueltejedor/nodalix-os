"""Behavioral regressions for the real GNOME-first migration failures."""
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock
import xml.etree.ElementTree as ET
from gi.repository import Gio

ROOT=Path(__file__).resolve().parents[1]


def load(name, path):
    spec=importlib.util.spec_from_loader(name,importlib.machinery.SourceFileLoader(name,str(ROOT/path)))
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module


PREFS=load('preferences','gnome/migration/user_preferences.py')
POSTER=load('poster','gnome/settings/wallpaper_poster.py')
WORKER=load('migration_worker','gnome/session/nodalix-gnome-user-migrate')
KEYD=load('keyboard_remap','integrations/scripts/nodalix-keyboard-remap.py')


class SessionTests(unittest.TestCase):
    def test_session_executes_gnome_even_if_migrator_would_fail(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);session=root/'gnome-session'
            session.write_text('#!/bin/sh\nprintf "GNOME started: %s\\n" "$*"\n');session.chmod(0o755)
            wrapper=root/'session';wrapper.write_text((ROOT/'gnome/session/nodalix-gnome-session').read_text().replace('/usr/bin/gnome-session',str(session)))
            result=subprocess.run(['sh',str(wrapper)],capture_output=True,text=True,check=True)
            self.assertIn('GNOME started: --session=gnome',result.stdout)
            self.assertNotIn('user-migrate',wrapper.read_text())

    def test_failed_migration_is_logged_and_retried_without_returning_failure(self):
        with tempfile.TemporaryDirectory() as temp,mock.patch.dict(os.environ,{'XDG_STATE_HOME':temp,'XDG_CURRENT_DESKTOP':'GNOME'}),mock.patch.object(WORKER.subprocess,'run',return_value=subprocess.CompletedProcess([],1)) as run:
            self.assertEqual(WORKER.main(),0)
            self.assertEqual(WORKER.main(),0)
            self.assertEqual(run.call_count,2)
            self.assertIn('exit status: 1',(Path(temp)/'nodalix/migrations/user-migration.log').read_text())

    def test_migration_timeout_cannot_change_session_result(self):
        with tempfile.TemporaryDirectory() as temp,mock.patch.dict(os.environ,{'XDG_STATE_HOME':temp,'XDG_CURRENT_DESKTOP':'GNOME'}),mock.patch.object(WORKER.subprocess,'run',side_effect=subprocess.TimeoutExpired('worker',120)):
            self.assertEqual(WORKER.main(),0)
            self.assertIn('incomplete',(Path(temp)/'nodalix/migrations/user-migration.log').read_text())


class PreferencesTests(unittest.TestCase):
    def test_spanish_and_multilayout_system_choices(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);folder=root/'etc/X11/xorg.conf.d';folder.mkdir(parents=True)
            (folder/'00-keyboard.conf').write_text('Option "XkbLayout" "es,us"\nOption "XkbVariant" ",intl"')
            self.assertEqual(PREFS.layout_from_system(root),[('xkb','es'),('xkb','us+intl')])
            (folder/'00-keyboard.conf').unlink()
            (root/'etc/vconsole.conf').write_text('KEYMAP=es\n')
            self.assertEqual(PREFS.layout_from_system(root),[('xkb','es')])
            (root/'etc/vconsole.conf').write_text('KEYMAP=uk\n')
            self.assertEqual(PREFS.layout_from_system(root),[('xkb','gb')])

    def test_ghostty_repeated_migration_preserves_unrelated_lines(self):
        with tempfile.TemporaryDirectory() as temp:
            path=Path(temp)/'config'
            path.write_text('font-size = 17\nwindow-decoration = none\nwindow-decoration = client\nkeybind = ctrl+v=paste_from_clipboard\nkeybind = ctrl+v=paste_from_clipboard\n')
            PREFS.clean_ghostty(path);first=path.read_text();PREFS.clean_ghostty(path)
            self.assertEqual(path.read_text(),first)
            self.assertIn('font-size = 17',first)
            self.assertEqual(first.count('ctrl+v=paste_from_clipboard'),1)
            self.assertEqual(first.count('window-decoration ='),1)
            self.assertIn('performable:ctrl+c=copy_to_clipboard',first)
            self.assertTrue(path.with_name('config.pre-gnome-030').is_file())

    def test_chatgpt_archives_only_the_exact_legacy_exec(self):
        with tempfile.TemporaryDirectory() as temp:
            path=Path(temp)/'chatgpt.desktop';backup=Path(temp)/'backup/chatgpt.desktop'
            original='[Desktop Entry]\nExec='+str(Path.home()/'.config/quickshell/scripts/nodalix-sync-user-language.sh')+' --launch-chatgpt\n'
            path.write_text(original);self.assertTrue(PREFS.retire_chatgpt(path,backup))
            self.assertEqual(backup.read_text(),original)
            path.write_text('[Desktop Entry]\nExec=my-personal-chatgpt\n')
            self.assertFalse(PREFS.retire_chatgpt(path,backup));self.assertTrue(path.exists())

    def test_custom_shortcuts_and_focus_survive_an_upgrade(self):
        source=f'''import sys\nsys.path.insert(0,{str(ROOT/'gnome/migration')!r})
from user_preferences import apply_preferences
from gi.repository import Gio
m=Gio.Settings.new('org.gnome.settings-daemon.plugins.media-keys')
m.set_strv('custom-keybindings',['/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/personal/'])
w=Gio.Settings.new('org.gnome.desktop.wm.preferences');w.set_string('focus-mode','click')
apply_preferences();apply_preferences()
assert w.get_string('focus-mode')=='click'
paths=m.get_strv('custom-keybindings');assert len(paths)==len(set(paths))
assert '/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/personal/' in paths
assert Gio.Settings.new('org.gnome.shell.keybindings').get_strv('toggle-message-tray')==['<Super>n']
'''
        with tempfile.TemporaryDirectory() as temp:
            result=subprocess.run(['python','-c',source],env={**os.environ,'HOME':temp,'XDG_CONFIG_HOME':temp,'XDG_DATA_HOME':temp,'XDG_STATE_HOME':temp,'GSETTINGS_BACKEND':'memory'},capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stderr)


class WallpaperTests(unittest.TestCase):
    def test_real_video_poster_is_cached_and_regenerated_only_on_change(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);video=root/'video.mp4'
            subprocess.run(['ffmpeg','-nostdin','-loglevel','error','-f','lavfi','-i','color=c=teal:s=160x90:d=2','-c:v','mpeg4',str(video)],check=True)
            poster=POSTER.poster_for(video,root/'cache');before=poster.stat().st_mtime_ns
            self.assertEqual(POSTER.poster_for(video,root/'cache'),poster)
            self.assertEqual(poster.stat().st_mtime_ns,before)
            os.utime(video,ns=(video.stat().st_atime_ns,video.stat().st_mtime_ns+1))
            self.assertNotEqual(POSTER.poster_for(video,root/'cache'),poster)
            self.assertEqual(poster.read_bytes()[:3],b'\xff\xd8\xff')

    def test_static_catalog_contains_seven_readable_paths(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)
            subprocess.run(['bash','-c','source "$1"; srcdir="$2"; pkgdir="$3"; package','stage',str(ROOT/'packaging/nodalix-wallpapers/PKGBUILD'),str(ROOT/'packaging/nodalix-wallpapers'),str(root)],check=True)
            catalog=ET.parse(root/'usr/share/gnome-background-properties/nodalix-wallpapers.xml')
            items=catalog.findall('wallpaper');self.assertEqual(len(items),7)
            for item in items:self.assertTrue((root/item.findtext('filename').lstrip('/')).is_file())
            self.assertTrue((root/'usr/share/backgrounds/nodalix/login/nodalix-login-fallback.jpg').is_file())

    def test_no_profile_claims_missing_rounded_blur(self):
        rows=json.loads((ROOT/'gnome/session/extension-defaults.json').read_text())
        self.assertFalse(any('rounded-blur-found' in row['values'] for row in rows))
        dock=rows[0]['values']
        for key,value in {'dock-fixed':'false','autohide':'true','intellihide':'true','multi-monitor':'true','autohide-in-fullscreen':'true'}.items():self.assertEqual(dock[key],value)


class KeyboardTests(unittest.TestCase):
    def fixture(self,root,event,keys,rel=0,name='Physical Keyboard',vendor='1234',product='abcd'):
        device=root/event/'device';(device/'capabilities').mkdir(parents=True);(device/'id').mkdir()
        for key,value in {'key':keys,'rel':rel,'abs':0}.items():(device/'capabilities'/key).write_text(hex(value)[2:])
        (device/'name').write_text(name);(device/'id/vendor').write_text(vendor);(device/'id/product').write_text(product)
    def test_keyboard_detection_rejects_mouse_multimedia_and_keyd(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);keys=sum(1<<k for k in (16,17,18,19,20,21,22,23,24,25,28,30,44,46,47,57,125))
            self.fixture(root,'event1',keys)
            self.fixture(root,'event2',keys,rel=3,name='Mouse with keyboard emulation')
            self.fixture(root,'event3',1<<115,name='Media Receiver')
            self.fixture(root,'event4',keys,vendor='0fac',name='keyd virtual keyboard')
            ids=KEYD.keyboard_ids(root);self.assertEqual(len(ids),1);self.assertNotIn('*',ids)
