"""0.2.4 ownership transitions, bridge integrity, boot boundary and backups."""
import hashlib
import importlib.machinery
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock
from test_updater import UPDATER, manifest, component
ROOT = Path(__file__).resolve().parents[1]

def load(name, path):
    spec=importlib.util.spec_from_loader(name,importlib.machinery.SourceFileLoader(name,str(ROOT/path)))
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module
SYSTEM=load('system_migration','gnome/session/nodalix-gnome-migrate')
BRIDGE=load('migration_bridge','updater/nodalix-migrate-gnome')

class MigrationTests(unittest.TestCase):
    def test_system_backups_sessions_and_idempotence(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)
            (root/'etc/systemd/system').mkdir(parents=True)
            (root/'etc/systemd/system/display-manager.service').symlink_to('/usr/lib/systemd/system/greetd.service')
            (root/'etc/greetd').mkdir();(root/'etc/greetd/config.toml').write_text('personal configuration')
            (root/'etc/passwd').write_text('fresh:x:1000:1000::/home/fresh:/bin/bash\nroot:x:0:0::/root:/bin/bash\n')
            accounts=root/'var/lib/AccountsService/users';accounts.mkdir(parents=True)
            (accounts/'daniel').write_text('[User]\nSession=hyprland\nLanguage=es_ES.UTF-8\nIcon=/home/daniel/avatar.png\n')
            (accounts/'custom').write_text('[User]\nSession=plasma\n')
            runner=mock.Mock()
            SYSTEM.migrate(root,runner)
            backup=root/'var/lib/nodalix-updater/migrations/gnome-0.3.0/files'
            self.assertEqual((backup/'etc/greetd/config.toml').read_text(),'personal configuration')
            self.assertEqual((backup/'etc/systemd/system/display-manager.service').readlink(),Path('/usr/lib/systemd/system/greetd.service'))
            self.assertIn('Session = nodalix',(accounts/'daniel').read_text())
            self.assertIn('Language = es_ES.UTF-8',(accounts/'daniel').read_text())
            self.assertIn('Icon = /home/daniel/avatar.png',(accounts/'daniel').read_text())
            self.assertIn('Session = nodalix',(accounts/'fresh').read_text())
            self.assertEqual((accounts/'custom').read_text(),'[User]\nSession=plasma\n')
            calls=runner.call_args_list
            self.assertTrue(any('gdm.service' in c.args[0] for c in calls))
            self.assertFalse(any(any(x in c.args[0] for x in ('--now','restart','reboot')) for c in calls))
            SYSTEM.migrate(root,runner)
            self.assertEqual(calls,runner.call_args_list)
            self.assertEqual((backup/'var/lib/AccountsService/users/daniel').read_text(),'[User]\nSession=hyprland\nLanguage=es_ES.UTF-8\nIcon=/home/daniel/avatar.png\n')
    def test_failure_not_marked_complete(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)
            runner=mock.Mock(side_effect=subprocess.CalledProcessError(1,['systemctl']))
            with self.assertRaises(subprocess.CalledProcessError):SYSTEM.migrate(root,runner)
            self.assertFalse((root/'var/lib/nodalix-updater/migrations/gnome-0.3.0/completed.json').exists())
    def test_reboot_pending_survives_status_reload_and_clears_at_boot(self):
        with tempfile.TemporaryDirectory() as temp, mock.patch.object(UPDATER,'STATUS_PATH',Path(temp)/'status'), mock.patch.object(UPDATER,'REBOOT_REQUIRED_PATH',Path(temp)/'run/reboot'):
            UPDATER.require_reboot('0.3.0')
            UPDATER.set_status('completed',reboot_mandatory=True,reboot_required=True)
            self.assertTrue(UPDATER.load_status()['reboot_mandatory'])
            with mock.patch.object(UPDATER.os,'geteuid',return_value=0), mock.patch.object(UPDATER,'current_version',return_value='0.3.0'), mock.patch.object(UPDATER,'release_info') as network:
                with self.assertRaises(UPDATER.BusyError):UPDATER.do_update({})
                network.assert_not_called()
            UPDATER.REBOOT_REQUIRED_PATH.unlink()
            self.assertFalse(UPDATER.load_status()['reboot_mandatory'])
            self.assertFalse(UPDATER.load_status()['reboot_required'])
    def test_split_and_declared_replacements_only(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);(root/'usr/bin').mkdir(parents=True)
            (root/'usr/bin/shared').touch()
            reader=lambda p:(p,['usr/bin/shared'] if p=='new-integrations' else [])
            kwargs=dict(root=root,payload_reader=reader,owner_lookup=lambda p:'old-shell')
            self.assertTrue(UPDATER.package_conflicts(['new-integrations'],**kwargs))
            self.assertEqual(UPDATER.package_conflicts(['new-integrations'],replacements={'old-shell'},**kwargs),[])
            self.assertEqual(UPDATER.package_conflicts(['new-integrations','old-shell'],**kwargs),[])
            with self.assertRaises(UPDATER.PackagePreflightError):
                UPDATER.package_conflicts(['a','b'],root=root,payload_reader=lambda p:(p,['usr/bin/shared']))
    def test_dependencies_use_candidate_providers_and_full_arch_upgrade(self):
        rows={'gnome':{'pkgname':['nodalix-gnome'],'depend':['gnome-shell','gnome-control-center','nodalix-integrations>=0.3.0']},'cc':{'pkgname':['nodalix-control-center'],'provides':['gnome-control-center=51.0']},'integrations':{'pkgname':['nodalix-integrations']}}
        with mock.patch.object(UPDATER,'package_metadata',side_effect=lambda p:rows[p]),mock.patch.object(UPDATER,'ensure_pacman_available'),mock.patch.object(UPDATER,'set_status'),mock.patch.object(UPDATER,'current_version',return_value='0.2.4'),mock.patch.object(UPDATER,'atomic_text'),mock.patch.object(UPDATER.subprocess,'run',return_value=subprocess.CompletedProcess([],0,stdout='ok')) as run:
            UPDATER.prepare_gnome_dependencies(list(rows),'0.3.0')
            self.assertEqual(run.call_args.args[0],['pacman','-Syu','--needed','--noconfirm','gnome-shell'])
    def test_candidate_uses_full_manifest_validation(self):
        with tempfile.TemporaryDirectory() as temp:
            path=Path(temp);data=manifest(version='0.3.0');data['components'][0]['asset']='../../bad.pkg.tar.zst'
            (path/'nodalix-manifest.json').write_text(json.dumps(data))
            with self.assertRaises(UPDATER.ManifestError):UPDATER.candidate_info(path,{})

class BridgeTests(unittest.TestCase):
    def fixture(self,path):
        asset='nodalix-updater-0.3.0-1-any.pkg.tar.zst';(path/asset).write_bytes(b'bridge')
        return {'schema_version':1,'version':'0.3.0','components':[{'package':'nodalix-updater','version':'0.3.0','asset':asset,'sha256':hashlib.sha256(b'bridge').hexdigest()}]}
    def test_bridge_installs_only_updater_before_full_update(self):
        with tempfile.TemporaryDirectory() as temp:
            path=Path(temp);data=self.fixture(path)
            runner=mock.Mock(return_value=subprocess.CompletedProcess([],0,stdout='pkgname = nodalix-updater\npkgver = 0.3.0-1\n'))
            BRIDGE.install_bridge(data,path,runner,offline=True)
            self.assertEqual(runner.call_args_list[1].args[0],['pacman','-U','--needed','--noconfirm',str(path/data['components'][0]['asset'])])
            self.assertEqual(runner.call_args_list[2].args[0],['/usr/bin/nodalix-updater','update','--json','--assets',str(path)])
    def test_corrupt_bridge_never_installs(self):
        with tempfile.TemporaryDirectory() as temp:
            path=Path(temp);data=self.fixture(path);(path/data['components'][0]['asset']).write_bytes(b'corrupt')
            runner=mock.Mock()
            with self.assertRaises(ValueError):BRIDGE.install_bridge(data,path,runner)
            runner.assert_not_called()
    def test_wrong_identity_never_installs(self):
        with tempfile.TemporaryDirectory() as temp:
            path=Path(temp);data=self.fixture(path)
            runner=mock.Mock(return_value=subprocess.CompletedProcess([],0,stdout='pkgname = other\npkgver = 0.3.0-1\n'))
            with self.assertRaises(ValueError):BRIDGE.install_bridge(data,path,runner)
            self.assertEqual(runner.call_count,1)

class CLIErrorTests(unittest.TestCase):
    def test_pending_reboot_is_json_error_not_traceback(self):
        import contextlib,io,sys
        with tempfile.TemporaryDirectory() as temp,mock.patch.object(UPDATER,'STATE_DIR',Path(temp)),mock.patch.object(UPDATER,'STATUS_PATH',Path(temp)/'status'),mock.patch.object(UPDATER,'current_version',return_value='0.3.0'),mock.patch.object(UPDATER,'load_config',return_value={}),mock.patch.object(UPDATER,'do_update',side_effect=UPDATER.BusyError('Reinicio obligatorio pendiente')),mock.patch.object(sys,'argv',['nodalix-updater','update','--json']):
            output=io.StringIO()
            with contextlib.redirect_stdout(output):code=UPDATER.main()
            self.assertEqual(code,2)
            self.assertIn('Reinicio obligatorio',json.loads(output.getvalue())['error'])
            self.assertEqual(json.loads((Path(temp)/'status').read_text())['status'],'update_busy')

class BridgePolicyTests(unittest.TestCase):
    def test_signature_refusal_prevents_bootstrap_install(self):
        import types,sys
        with tempfile.TemporaryDirectory() as temp:
            path=Path(temp);data=BridgeTests().fixture(path)
            (path/'nodalix-manifest.json').write_text(json.dumps(data))
            client=types.SimpleNamespace(validate_manifest=mock.Mock(),current_version=lambda:'0.2.4',compare_versions=UPDATER.compare_versions,load_config=lambda:{'require_signature':True},verify_manifest_signature=mock.Mock(side_effect=UPDATER.IntegrityError('manifest is not signed')))
            with mock.patch.object(BRIDGE.importlib.util,'spec_from_loader',return_value=types.SimpleNamespace(loader=mock.Mock())),mock.patch.object(BRIDGE.importlib.util,'module_from_spec',return_value=client),mock.patch.object(BRIDGE.os,'geteuid',return_value=0),mock.patch.object(BRIDGE,'install_bridge') as install,mock.patch.object(sys,'argv',['bridge','--assets',str(path)]):
                with self.assertRaises(UPDATER.IntegrityError):BRIDGE.main()
            install.assert_not_called();client.validate_manifest.assert_called_once_with(data)
    def test_bridge_refuses_downgrading_newer_installation(self):
        import types
        client=types.SimpleNamespace(validate_manifest=mock.Mock(),current_version=lambda:'0.4.0',compare_versions=UPDATER.compare_versions)
        with mock.patch.object(BRIDGE.importlib.util,'spec_from_loader',return_value=types.SimpleNamespace(loader=mock.Mock())),mock.patch.object(BRIDGE.importlib.util,'module_from_spec',return_value=client):
            with self.assertRaisesRegex(ValueError,'0.2.4'):BRIDGE.verify_bridge_policy(b'{}',{})

class UserMigrationTests(unittest.TestCase):
    def test_regular_user_overrides_backed_up_before_mask_and_migration_runs_once(self):
        import os
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);commands=root/'bin';commands.mkdir()
            home=root/'home';units=home/'.config/systemd/user';units.mkdir(parents=True)
            unit=units/'nodalix-shell.service';unit.write_text('[Service]\nExecStart=personal-shell\n')
            for name in ('gsettings','nodalix-app-icons'):
                p=commands/name;p.write_text('#!/bin/sh\nexit 0\n');p.chmod(0o755)
            p=commands/'systemctl';p.write_text('''#!/usr/bin/python3
import os,sys
from pathlib import Path
with Path(os.environ['COMMAND_LOG']).open('a') as f:f.write(' '.join(sys.argv[1:])+'\\n')
if 'is-enabled' in sys.argv:print('enabled')
if 'mask' in sys.argv:
    assert '--force' in sys.argv
    p=Path(os.environ['HOME'])/'.config/systemd/user/nodalix-shell.service'
    assert list((Path(os.environ['XDG_STATE_HOME'])/'nodalix/migrations').glob('gnome-first-*/user/nodalix-shell.service'))
    p.unlink();p.symlink_to('/dev/null')
''');p.chmod(0o755)
            log=root/'commands.log'
            env={**os.environ,'PATH':str(commands)+':'+os.environ['PATH'],'HOME':str(home),'XDG_CONFIG_HOME':str(home/'.config'),'XDG_STATE_HOME':str(root/'state'),'XDG_CURRENT_DESKTOP':'GNOME','GSETTINGS_BACKEND':'memory','COMMAND_LOG':str(log)}
            helper=ROOT/'gnome/session/nodalix-gnome-user-migrate'
            subprocess.run(['sh',str(helper)],env=env,check=True,capture_output=True)
            self.assertEqual(unit.readlink(),Path('/dev/null'))
            saved=list((root/'state/nodalix/migrations').glob('gnome-first-*/user/nodalix-shell.service'))
            self.assertEqual(saved[0].read_text(),'[Service]\nExecStart=personal-shell\n')
            before=log.read_text()
            subprocess.run(['sh',str(helper)],env=env,check=True,capture_output=True)
            self.assertEqual(log.read_text(),before)
