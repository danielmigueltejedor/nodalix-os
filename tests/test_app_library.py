"""A Trash intent must refer to a registered app, never an arbitrary command."""
import base64
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch, Mock
from xml.etree import ElementTree

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('app_library', ROOT/'nxpm/library/library.py')
library = importlib.util.module_from_spec(spec); spec.loader.exec_module(library)


class LibraryFolderIconTests(unittest.TestCase):
    def test_folder_icon_reuses_active_theme_and_adds_app_grid(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root/'folder.svg'
            source.write_text(
                '<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">'
                '<rect width="64" height="64" fill="#a4caee"/>'
                '<rect width="32" height="24" fill="#438de6"/>'
                '</svg>'
            )
            storage = root/'storage'
            with patch.object(library, 'STORAGE', storage), patch.object(
                library, 'resolve_icon', return_value=source.as_uri()
            ) as resolve:
                uri = library.folder_icon_uri()
                target = Path(library.Gio.File.new_for_uri(uri).get_path())
                self.assertEqual(target.parent, storage)
                self.assertEqual(resolve.call_args.args[0].get_names()[0], 'folder')
                svg = ElementTree.parse(target)
                image = svg.find('{*}image')
                href = image.attrib['{http://www.w3.org/1999/xlink}href']
                self.assertEqual(base64.b64decode(href.split(',', 1)[1]), source.read_bytes())
                self.assertEqual(len(svg.findall('.//{*}rect')), 4)
                self.assertEqual(svg.find('{*}g').get('fill'), '#438de6')
                with patch.object(library.GLib, 'file_set_contents') as write:
                    self.assertEqual(library.folder_icon_uri(), uri)
                    write.assert_not_called()
                source.write_text(source.read_text().replace('#438de6', '#3e928a'))
                self.assertNotEqual(library.folder_icon_uri(), uri)


class LibraryTrashTests(unittest.TestCase):
    def test_only_registered_file_trashed_from_apps_can_uninstall(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp); folder = root/'Applications'; folder.mkdir()
            trash = root/'Trash'; (trash/'info').mkdir(parents=True); (trash/'files').mkdir()
            registry = {'qa.desktop': {'token': 'secret', 'filename': 'QA.app'}}
            file = trash/'files/QA.app'
            file.write_text(json.dumps({'desktop_id': 'qa.desktop', 'token': 'secret'}))
            info = trash/'info/QA.app.trashinfo'
            def origin(path):info.write_text('[Trash Info]\nPath='+str(path)+'\nDeletionDate=2026-11-03T12:00:00\n')
            origin(root/'elsewhere/QA.app')
            self.assertEqual(library.trash_requests(folder, registry, trash), [])
            origin(folder/'QA.app')
            self.assertEqual(len(library.trash_requests(folder, registry, trash)), 1)
            file.write_text(json.dumps({'desktop_id': 'qa.desktop', 'token': 'forged', 'Exec': 'bad'}))
            self.assertEqual(library.trash_requests(folder, registry, trash), [])
            file.unlink();file.symlink_to(root/'payload')
            self.assertEqual(library.trash_requests(folder, registry, trash), [])

    def test_open_rejects_unregistered_descriptors_without_executing(self):
        with tempfile.TemporaryDirectory() as temp:
            file = Path(temp)/'QA.app'
            file.write_text(json.dumps({'format':'nodalix-app-1','desktop_id':'qa.desktop','token':'bad','Exec':'bad'}))
            with patch.object(library, 'desktop_app') as launch, patch.object(library, 'STATE', Path(temp)):
                with self.assertRaises(ValueError):library.open_app(file)
                launch.assert_not_called()

    def test_flatpak_uninstall_keeps_data_and_other_dependencies(self):
        run=Mock(return_value=Mock(returncode=0));app=Mock()
        app.get_id.return_value='qa.desktop';app.get_filename.return_value='/nonexistent/qa.desktop'
        with patch.object(library,'DESKTOPS',Path('/nonexistent')):
            library.uninstall(app,{'kind':'flatpak','scope':'--user','ref':'app/qa/x86_64/stable'},run)
        self.assertEqual(run.call_args.args[0],['/usr/bin/flatpak','uninstall','--user','--noninteractive','-y','--','app/qa/x86_64/stable'])

    def test_native_uninstall_resolves_package_and_uses_nxpm(self):
        run = Mock(return_value=Mock(returncode=0))
        app = Mock()
        app.get_id.return_value = 'qa.desktop'
        app.get_filename.return_value = '/nonexistent/qa.desktop'
        with patch.object(library, 'DESKTOPS', Path('/nonexistent')), \
             patch.object(library, 'system_desktop', return_value=Path('/usr/share/applications/qa.desktop')), \
             patch.object(library.subprocess, 'run', return_value=Mock(stdout='qa-package\n', returncode=0)):
            library.uninstall(app, {'kind': 'package', 'desktop_id': 'qa.desktop'}, run)
        self.assertEqual(run.call_args.args[0], ['/usr/bin/nxpm', 'remove', 'qa-package'])

    def test_personal_file_and_renamed_managed_entry_are_preserved(self):
        with tempfile.TemporaryDirectory() as temp:
            folder=Path(temp);user=folder/'Fixture.app';user.write_text('personal')
            launcher=folder/'qa.desktop';launcher.write_text('[Desktop Entry]\nType=Application\nName=Fixture\nExec=/usr/bin/true\n')
            app=library.Gio.DesktopAppInfo.new_from_filename(str(launcher));registry={}
            with patch.object(library,'icon_uri',return_value=''), patch.object(library,'folder_icon_uri',return_value=''):
                library.reconcile(folder,registry,[app]);first=folder/registry['qa.desktop']['filename'];first.rename(folder/'Personal name.app')
                library.reconcile(folder,registry,[app])
            self.assertEqual(user.read_text(),'personal')
            self.assertEqual(registry['qa.desktop']['filename'],'Personal name.app')
            self.assertEqual(len(list(folder.glob('*.app'))),2)

class PortalLaunchTests(unittest.TestCase):
    def test_flatpak_waits_for_real_document_mount(self):
        from gi.repository import GLib
        with tempfile.TemporaryDirectory() as temp:
            mount=Path(temp);(mount/'by-app/qa').mkdir(parents=True)
            app=Mock();app.get_string.return_value='qa'
            bus=Mock();bus.call_sync.return_value=GLib.Variant('(ay)',(list(os.fsencode(temp)+b'\0'),))
            with patch.object(library.Gio,'bus_get_sync',return_value=bus):
                library.ensure_document_portal(app,attempts=1)
            (mount/'by-app/qa').rmdir()
            with patch.object(library.Gio,'bus_get_sync',return_value=bus):
                with self.assertRaises(ValueError):library.ensure_document_portal(app,attempts=1)

    def test_native_apps_do_not_need_flatpak_portals(self):
        app=Mock();app.get_string.return_value=None
        with patch.object(library.Gio,'bus_get_sync') as bus:
            library.ensure_document_portal(app)
            bus.assert_not_called()
