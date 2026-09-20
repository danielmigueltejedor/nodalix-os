import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
def script(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'shell/scripts' / ('nodalix-' + name + '.py'))
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
        sync(); source.unlink(); sync(); self.assertFalse(target.exists())
    def test_user_icon_restored_when_theme_removed(self):
        apps=self.home/'.local/share/applications'; apps.mkdir(parents=True)
        theme=self.home/'themes/Colloid-Teal/apps/scalable'; theme.mkdir(parents=True)
        icon=theme/'matlab.svg'; icon.write_text('<svg/>')
        target=apps/'matlab.desktop'; original='[Desktop Entry]\nType=Application\nName=MATLAB\nExec=matlab\nIcon=matlab\n'; target.write_text(original)
        sync=lambda: icons.synchronize(self.home,[],[self.home/'themes'])
        sync(); icon.unlink(); sync(); self.assertEqual(target.read_text(),original)
