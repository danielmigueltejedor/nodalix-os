import argparse
import importlib.machinery
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
loader = importlib.machinery.SourceFileLoader('recovery', str(ROOT/'updater/nodalix-recovery'))
spec = importlib.util.spec_from_loader(loader.name, loader)
r = importlib.util.module_from_spec(spec); loader.exec_module(r)

class RecoveryTests(unittest.TestCase):
    info = {'fstype':'btrfs','fsroot':'/@','uuid':'test-root','separate_home':True}

    def test_layout_must_not_capture_personal_files(self):
        self.assertTrue(r.supported(self.info))
        for change in ({'separate_home':False},{'fstype':'ext4'},{'fsroot':'/other'}):
            self.assertFalse(r.supported(self.info | change))

    def test_existing_backup_destination_is_never_replaced(self):
        for current in ({'backup_device_uuid':'external','btrfs_mode':'true'}, {'backup_device_uuid':'test-root','btrfs_mode':'false'}):
            with self.assertRaises(RuntimeError): r.config_for(self.info,current)
        result = r.config_for(self.info,{})
        self.assertEqual(result['include_btrfs_home_for_restore'],'false')
        self.assertEqual(result['backup_device_uuid'],'test-root')

    def test_unknown_snapshot_and_path_traversal_are_not_listed(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            for name, device in [('2026-09-22_12-00-00','test-root'),('2026-09-22_13-00-00','other-root'),('invalid','test-root')]:
                point = base/name; point.mkdir()
                (point/'info.json').write_text(json.dumps({'sys-uuid':device,'created':'1','comments':'Nodalix manual'}))
            points = r.snapshots(base,'test-root')
            self.assertEqual(len(points),1)
            self.assertFalse(points[0]['restorable'])
            marker = base/points[0]['name']/'@/var/lib/nodalix-recovery/boot-ready.json'
            marker.parent.mkdir(parents=True);marker.write_text('{}')
            self.assertTrue(r.snapshots(base,'test-root')[0]['restorable'])
        self.assertIsNone(r.NAME.fullmatch('../../etc'))
        self.assertIsNone(r.NAME.fullmatch('--delete-all'))

    def test_low_space_refuses_new_snapshot(self):
        from collections import namedtuple
        Disk = namedtuple('Disk','total used free')
        with patch.object(r.shutil,'disk_usage',return_value=Disk(100*1024**3,99*1024**3,1024**3)):
            with self.assertRaises(RuntimeError): r.enough_space()

    def test_failed_timeshift_command_is_not_reported_as_success(self):
        import subprocess
        with patch.object(r.subprocess,'run',return_value=subprocess.CompletedProcess([],1,stdout='failed')):
            with self.assertRaisesRegex(RuntimeError,'failed'): r.run('timeshift','--create')

    def test_automatic_schedule_off_never_creates_snapshot(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(r,'STATE',Path(directory)), patch.object(r,'root_info',return_value=self.info), patch.object(r,'CONFIG',Path(directory)/'config'), patch.object(r,'status',return_value={'automatic':False}), patch.object(r,'preserve_boot') as boot:
            result = r.execute(argparse.Namespace(action='scheduled',value='',confirm=False))
            self.assertFalse(result['automatic']);boot.assert_not_called()
