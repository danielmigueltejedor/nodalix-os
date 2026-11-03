"""Regressions fixed while preparing 0.4.0; never modify the running system."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
import contextlib
import shutil
from unittest import mock
from test_updater import UPDATER
from test_recovery import r as recovery
from test_native_settings import settings

ROOT=Path(__file__).resolve().parents[1]


class IdentityTests(unittest.TestCase):
    def test_missing_corrupt_and_placeholder_identity_are_not_valid_versions(self):
        with tempfile.TemporaryDirectory() as folder:
            path=Path(folder)/'nodalix-release'
            with mock.patch.object(UPDATER,'RELEASE_PATH',path):
                for content in (None,'NAME=Nodalix\n','VERSION_ID="garbage"','VERSION_ID="0.0.0"'):
                    if content is not None:path.write_text(content)
                    with self.subTest(content=content), self.assertRaises(UPDATER.IdentityError):UPDATER.current_version()
                path.write_text('VERSION_ID="0.4.0"\n')
                self.assertEqual(UPDATER.current_version(),'0.4.0')

    def test_cli_returns_structured_error_before_network_or_updates(self):
        with tempfile.TemporaryDirectory() as folder:
            result=subprocess.run(['/usr/bin/python3',str(ROOT/'updater/nodalix-updater'),'info','--json'],
                capture_output=True,text=True,env={**os.environ,'NODALIX_RELEASE_FILE':folder+'/missing',
                'NODALIX_UPDATER_STATE':folder+'/state'})
            self.assertEqual(result.returncode,2)
            data=json.loads(result.stdout)
            self.assertEqual(data['code'],'identity_invalid')
            self.assertNotEqual(data.get('current_version'),'0.0.0')


class UpdateStateTests(unittest.TestCase):
    def setUp(self):
        folder=tempfile.TemporaryDirectory();self.addCleanup(folder.cleanup)
        self.folder=Path(folder.name)
        self.env=mock.patch.dict(os.environ,{'XDG_STATE_HOME':folder.name})
        self.env.start();self.addCleanup(self.env.stop)

    def test_interruption_is_visible_after_service_restart_and_new_boot_clears_reboot(self):
        helper=settings.update_state;path=self.folder/'state.json'
        helper.save(path,{'busy':True,'log':'transaction started','phase':'installing',
                          'reboot_required':True,'reboot_boot_id':'boot1'})
        self.assertEqual(path.stat().st_mode & 0o777,0o600)
        state=helper.restore(path,'boot1')
        self.assertEqual(state['phase'],'failed')
        self.assertTrue(state['reboot_required'])
        self.assertFalse(helper.restore(path,'boot2')['reboot_required'])

    def test_logs_redact_private_paths_and_credentials(self):
        text=settings.update_state.sanitize('https://name:secret@example.org /home/tester/file token=abcdef password:abc')
        for private in ('name:secret','/home/tester','abcdef','password:abc'):self.assertNotIn(private,text)

    def test_package_list_ignores_diagnostics_and_does_not_invent_metadata(self):
        rows=settings.update_state.package_updates('linux 1 -> 2\nwarning: ignored\nmutter 51-1 -> 51-2')
        self.assertEqual([row['name'] for row in rows],['linux','mutter'])
        self.assertNotIn('size',rows[0])

    def test_noop_system_transaction_does_not_require_reboot(self):
        service=settings.SettingsService()
        with mock.patch.object(service,'critical_versions',return_value={'linux':'1'}):service._run([('system',[])])
        self.assertFalse(service.state['reboot_required'])
        self.assertEqual(service.state['phase'],'completed')

    def test_failed_transaction_preserves_real_reboot_reason_if_critical_packages_changed(self):
        service=settings.SettingsService()
        with mock.patch.object(service,'critical_versions',side_effect=[{'linux':'1'},{'linux':'2'}]), \
             mock.patch.object(settings.subprocess,'Popen',side_effect=OSError('hook failed')):
            service._run([('system',[['fake']])])
        self.assertTrue(service.state['reboot_required'])
        self.assertEqual(service.state['phase'],'failed')
        self.assertIn('linux',service.state['reboot_reason'])

    def test_snapshot_protection_skips_unsupported_or_opted_out_systems(self):
        with mock.patch.object(recovery,'root_info',return_value={'fstype':'ext4'}), \
             mock.patch.object(recovery,'preserve_boot') as preserve:
            data=recovery.execute(argparse.Namespace(action='pre-update',value='',confirm=False))
            self.assertFalse(data['snapshot_created']);preserve.assert_not_called()
        with mock.patch.object(recovery,'root_info',return_value={'fstype':'btrfs','fsroot':'/@','uuid':'x','separate_home':True}), \
             mock.patch.object(recovery,'STATE',self.folder), mock.patch.object(recovery,'preserve_boot') as preserve:
            self.assertEqual(recovery.execute(argparse.Namespace(action='pre-update',value='',confirm=False))['reason'],'protection-not-enabled')
            preserve.assert_not_called()

    def test_pre_update_snapshot_is_verified_and_prunes_only_its_own_retention(self):
        (self.folder/'automatic').touch()
        rows=[{'name':str(i),'description':'Nodalix pre-update'} for i in range(5)]
        rows.append({'name':'manual','description':'Nodalix manual'})
        with mock.patch.object(recovery,'root_info',return_value={'fstype':'btrfs','fsroot':'/@','uuid':'x','separate_home':True}), \
             mock.patch.object(recovery,'STATE',self.folder),mock.patch.object(recovery,'CONFIG',self.folder/'config'), \
             mock.patch.object(recovery,'repository',return_value=contextlib.nullcontext(self.folder)), \
             mock.patch.object(recovery,'snapshots',return_value=[]),mock.patch.object(recovery,'enough_space'), \
             mock.patch.object(recovery,'preserve_boot'),mock.patch.object(recovery,'status',return_value={'snapshots':rows}), \
             mock.patch.object(recovery,'run') as run:
            recovery.execute(argparse.Namespace(action='pre-update',value='',confirm=False))
            self.assertIn(mock.call('timeshift','--create','--btrfs','--comments','Nodalix pre-update','--tags','O','--scripted'),run.call_args_list)
            deleted=[call.args[3] for call in run.call_args_list if call.args[1]=='--delete']
            self.assertEqual(deleted,['3','4'])


class TilingTests(unittest.TestCase):
    def test_real_layout_insertion_gives_halves_quarters_and_caps_default_at_four(self):
        source=ROOT/'gnome/extensions/nodalix-auto-tiling@getnodalia.com/extension.js'
        layout=source.with_name('layout.js')
        script=r'''
const fs=require('fs'),assert=require('assert');
const layout=fs.readFileSync(process.argv[1],'utf8').replace(/^export /gm,'');
const Layout=new Function(layout+'; return {ids,insert,rectangles,minimum};')();
const text=fs.readFileSync(process.argv[2],'utf8');
const method=text.slice(text.indexOf('_insertWindow(group, id) {')+'_insertWindow(group, id) {'.length,text.indexOf('\n    _retileGroup'));
const fn=new Function('Layout','global','return function(group,id){'+method.slice(0,method.lastIndexOf('}'))+'}')(Layout,{display:{get_focus_window:()=>null}});
const group={tree:null,area:{x:0,y:0,width:1000,height:1000},gap:0,sizes:new Map()};
for(let id=1;id<=4;id++) {
  fn.call({_fits:()=>true},group,id);
  const rows=[...Layout.rectangles(group.tree,group.area,0).leaves.values()];
  assert.equal(rows.length,id);
  assert.equal(rows.reduce((n,r)=>n+r.width*r.height,0),1000000);
  if(id===4) assert(rows.every(r=>r.width===500&&r.height===500));
}
'''
        subprocess.run(['node','-e',script,str(layout),str(source)],check=True,capture_output=True)
        import xml.etree.ElementTree as ET
        xml=ET.parse(next(source.parent.glob('schemas/*.xml')))
        self.assertEqual(xml.find(".//key[@name='max-windows']/default").text,'4')


@unittest.skipUnless(Path('/usr/lib/systemd/system-generators/zram-generator').is_file(),'Requires upstream zram-generator')
class ZramTests(unittest.TestCase):
    def test_real_generator_sizes_low_and_high_memory_and_preserves_admin_override(self):
        for memory,expected in ((2048,1024),(16384,8192),(32768,8192)):
            with self.subTest(memory=memory),tempfile.TemporaryDirectory() as folder:
                root=Path(folder)
                for name in ('usr/lib/systemd','etc/systemd','proc','sys/module/zram','run','out'):(root/name).mkdir(parents=True,exist_ok=True)
                shutil.copy2(ROOT/'integrations/assets/zram-generator.conf',root/'usr/lib/systemd/zram-generator.conf')
                (root/'proc/meminfo').write_text(f'MemTotal: {memory*1024} kB\n')
                (root/'proc/cmdline').write_text('')
                cmd=['/usr/lib/systemd/system-generators/zram-generator',str(root/'out')]
                env={**os.environ,'ZRAM_GENERATOR_ROOT':folder}
                result=subprocess.run(cmd,env=env,capture_output=True,text=True,check=True)
                self.assertIn(f'with {expected}MB',result.stdout)
                shutil.rmtree(root/'out');(root/'out').mkdir()
                (root/'etc/systemd/zram-generator.conf').write_text('[zram0]\nzram-size=512\n')
                result=subprocess.run(cmd,env=env,capture_output=True,text=True,check=True)
                self.assertIn('with 512MB',result.stdout)


if __name__=='__main__':unittest.main()
