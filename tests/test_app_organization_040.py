"""Native folders follow launcher precedence, new suite apps and user folders."""
import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SOURCE=Path(__file__).resolve().parents[1]/'integrations/scripts/nodalix-organize-apps.py'
spec=importlib.util.spec_from_file_location('organizer',SOURCE)
organizer=importlib.util.module_from_spec(spec);spec.loader.exec_module(organizer)


class AppOrganization(unittest.TestCase):
    def test_future_suite_apps_and_standard_categories(self):
        for app in ('ai.storyteller.future-tool.desktop','io.tangencylabs.solidcraft.desktop',
                    'com.storytold.future.desktop'):
            self.assertEqual(organizer.classify(app,{'Categories':'Graphics;'}),'nodalix-craft')
        self.assertEqual(organizer.classify('new.desktop',{'X-ArtCraft-App':'true'}),'nodalix-craft')
        self.assertEqual(organizer.classify('minecraft.desktop',{'Categories':'Game;'}),'games')
        self.assertEqual(organizer.classify('editor.desktop',{'Categories':'Utility;TextEditor;'}),'utilities')
        self.assertEqual(organizer.classify('browser.desktop',{'Categories':'Network;WebBrowser;'}),'internet')
        self.assertEqual(organizer.classify('freecad.desktop',{'Categories':'Graphics;'}),'engineering')

    def test_real_gio_folder_updates_are_idempotent_and_preserve_custom_folders(self):
        code=r'''
import json,runpy
from pathlib import Path
from gi.repository import Gio
m=runpy.run_path(SOURCE)
home=Path.home();low=home/'system';high=home/'user'
low.mkdir();high.mkdir()
def entry(folder,name,categories='Utility;',extra=''):
    path=folder/name;path.parent.mkdir(parents=True,exist_ok=True)
    path.write_text('[Desktop Entry]\nType=Application\nName='+name+'\nExec=/usr/bin/true\nCategories='+categories+'\n'+extra)
entry(low,'ai.storyteller.future.desktop','Graphics;')
entry(high,'io.tangencylabs.solidcraft.desktop','Graphics;')
entry(low,'hidden.desktop');entry(high,'hidden.desktop',extra='Hidden=true\n')
entry(low,'editor.desktop','Utility;TextEditor;')
entry(low,'personal.desktop','Graphics;')
entry(low,'kde.desktop',extra='OnlyShowIn=KDE;\n')
entry(low,'missing.desktop',extra='TryExec=/nonexistent/nodalix-test\n')
entry(low,'nested/tool.desktop','Network;')
settings=Gio.Settings.new('org.gnome.desktop.app-folders')
settings.set_strv('folder-children',['my-folder','nodalix-craft'])
custom=Gio.Settings.new_with_path('org.gnome.desktop.app-folders.folder','/org/gnome/desktop/app-folders/folders/my-folder/')
custom.set_string('name','Mi carpeta');custom.set_strv('apps',['personal.desktop','ai.storyteller.future.desktop'])
custom.set_strv('categories',['Graphics'])
suite=Gio.Settings.new_with_path('org.gnome.desktop.app-folders.folder','/org/gnome/desktop/app-folders/folders/nodalix-craft/')
suite.set_strv('excluded-apps',['ai.storyteller.future.desktop'])
groups=m['organize'](home,[low,high])
assert groups['nodalix-craft']==['ai.storyteller.future.desktop','io.tangencylabs.solidcraft.desktop'],groups
assert groups['internet']==['nested-tool.desktop'],groups
assert groups['utilities']==['editor.desktop'],groups
assert custom.get_string('name')=='Mi carpeta'
assert custom.get_strv('apps')==['personal.desktop']
assert set(custom.get_strv('excluded-apps'))==set(groups['nodalix-craft'])
assert suite.get_strv('excluded-apps')==[]
first=settings.get_strv('folder-children');assert 'my-folder' in first
assert m['organize'](home,[low,high])==groups
assert settings.get_strv('folder-children')==first
entry(high,'ai.storyteller.brand-new.desktop','Development;')
assert 'ai.storyteller.brand-new.desktop' in m['organize'](home,[low,high])['nodalix-craft']
backup=home/'.local/state/nodalix/app-folders/before-040-dynamic.json'
assert backup.stat().st_mode & 0o777==0o600
assert json.loads(backup.read_text())['children']==['my-folder','nodalix-craft']
'''
        with tempfile.TemporaryDirectory() as directory:
            env=dict(os.environ,HOME=directory,XDG_STATE_HOME=directory+'/.local/state',
                     GSETTINGS_BACKEND='memory',XDG_CURRENT_DESKTOP='GNOME')
            result=subprocess.run([sys.executable,'-c','SOURCE='+repr(str(SOURCE))+'\n'+code],
                                  env=env,capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stdout+result.stderr)
