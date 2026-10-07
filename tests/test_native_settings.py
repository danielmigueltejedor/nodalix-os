import importlib.util
import io
import json
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('native_settings', ROOT/'gnome/settings/service.py')
settings = importlib.util.module_from_spec(spec)
spec.loader.exec_module(settings)

class NativeSettingsTests(unittest.TestCase):
    def test_invalid_operation_never_launches_worker(self):
        service = settings.SettingsService()
        with patch.object(settings.threading, 'Thread') as worker:
            for target in ['system;touch /tmp/a', '../all', '', 'shell']:
                with self.assertRaises(ValueError): service.start(target)
            worker.assert_not_called()
        self.assertFalse(json.loads(service.snapshot())['busy'])

    def test_all_skips_uninstalled_sources_and_rejects_overlap(self):
        service = settings.SettingsService()
        with (patch.object(service,'available',side_effect=lambda k:k in ('system','apps')),
              patch.object(settings,'program',return_value='/usr/bin/nodalix-apps'),
              patch.object(settings.threading,'Thread') as worker):
            service.start('all')
            plans = worker.call_args.kwargs['args'][0]
            self.assertEqual([k for k,_ in plans],['system','apps'])
            self.assertEqual(plans[1][1],[['/usr/bin/nodalix-apps','update','--all']])
            with self.assertRaises(ValueError): service.check()
            with self.assertRaises(ValueError): service.start('system')
            self.assertEqual(worker.call_count,1)

    def test_partial_failure_is_reported_after_other_sources_complete(self):
        service = settings.SettingsService()
        service.state['busy'] = True
        class Process:
            def __init__(self, command, **kwargs):
                self.command = command
                self.stdout = io.StringIO('resultado\n')
            def __enter__(self): return self
            def __exit__(self,*args): pass
            def wait(self): return 1 if self.command == ['failure'] else 0
        with patch.object(settings.subprocess,'Popen',side_effect=Process):
            service._run([('apps',[['failure']]),('system',[['success']])])
        state=json.loads(service.snapshot())
        self.assertFalse(state['busy'])
        self.assertIn('No se completó',state['message'])
        self.assertIn('Aplicaciones Nodalix',state['message'])
        self.assertTrue(state['reboot_required'])
        self.assertIn('Programas y kernel',state['log'])

    def test_unknown_video_cannot_change_settings_or_enable_renderer(self):
        service=settings.SettingsService()
        with patch.object(settings,'catalog',return_value=[{'path':'/known.mp4'}]), patch.object(settings.Gio.Settings,'new') as prefs:
            with self.assertRaises(ValueError): service.wallpaper('/etc/passwd')
            prefs.assert_not_called()

    def test_privileged_helper_rejects_arbitrary_command(self):
        result=subprocess.run(['python3',str(ROOT/'gnome/settings/updates-privileged'),'system;echo test'],capture_output=True,text=True)
        self.assertNotEqual(result.returncode,0)
        self.assertIn('Operación no admitida',result.stderr)

    def test_privileged_helper_needs_authentication(self):
        import os
        if os.geteuid()==0: self.skipTest('Runs as root')
        result=subprocess.run(['python3',str(ROOT/'gnome/settings/updates-privileged'),'system'],capture_output=True,text=True)
        self.assertNotEqual(result.returncode,0)
        self.assertIn('autenticación',result.stderr)

class LocalSendSettingsTests(unittest.TestCase):
    def test_only_existing_shared_options_can_be_written(self):
        with patch.object(settings,'localsend_settings') as prefs:
            for key,value in [('fingerprint','"NEW"'),('certificate','"new.pem"'),('auto-disable-minutes','true'),('auto-disable-minutes','0'),('auto-accept','"yes"'),('alias','"a\\n"'),('download-folder','"relative"')]:
                with self.subTest(key=key,value=value), self.assertRaises(ValueError):
                    settings.set_localsend_option(key,value)
            prefs.assert_not_called()

    def test_option_write_keeps_identity_and_other_preferences(self):
        with patch.object(settings,'localsend_settings') as factory, patch.object(settings.Gio.Settings,'sync'):
            settings.set_localsend_option('start-on-login','true')
            factory.return_value.set_boolean.assert_called_once_with('start-on-login',True)
            factory.return_value.set_string.assert_not_called()
            factory.return_value.set_strv.assert_not_called()

    def test_favorites_preserve_existing_order_and_do_not_duplicate(self):
        with patch.object(settings,'localsend_settings') as factory, patch.object(settings.Gio.Settings,'sync'):
            prefs=factory.return_value; prefs.get_strv.return_value=['PHONE','TABLET']
            settings.set_localsend_favorite('PHONE',True)
            prefs.set_strv.assert_called_with('favorite-fingerprints',['PHONE','TABLET'])
            settings.set_localsend_favorite('PC',True)
            prefs.set_strv.assert_called_with('favorite-fingerprints',['PHONE','TABLET','PC'])
            settings.set_localsend_favorite('PHONE',False)
            prefs.set_strv.assert_called_with('favorite-fingerprints',['TABLET'])
            prefs.set_string.assert_not_called()

    def test_runtime_snapshot_uses_shared_bridge_without_exposing_private_paths(self):
        from unittest.mock import MagicMock
        prefs=MagicMock();prefs.get_value.side_effect=lambda key: settings.GLib.Variant('s','Test') if settings.LOCAL_OPTIONS[key] is str else settings.GLib.Variant('b',False) if settings.LOCAL_OPTIONS[key] is bool else settings.GLib.Variant('i',10)
        prefs.get_strv.return_value=['PHONE']
        payload={'enabled':True,'devices':[{'fingerprint':'PHONE','favorite':True}], 'certificate':'private.pem','key':'private.key','identity':{'fingerprint':'SELF'}}
        with patch.object(settings,'localsend_settings',return_value=prefs), patch.object(settings,'localsend_call',return_value=settings.GLib.Variant('(s)',(json.dumps(payload),))) as bridge:
            state=json.loads(settings.localsend_snapshot())
            bridge.assert_called_once_with('GetStatus')
            self.assertTrue(state['ready']);self.assertTrue(state['devices'][0]['favorite'])
            self.assertNotIn('certificate',state);self.assertNotIn('key',state)

class LocalSendBridgeTests(unittest.TestCase):
    def test_bridge_enable_settings_and_cleanup_on_isolated_bus(self):
        import os, shutil, tempfile
        if not shutil.which('gjs') or not shutil.which('dbus-run-session'):
            self.skipTest('GJS and D-Bus are required')
        with tempfile.TemporaryDirectory(prefix='nx-bridge-') as directory:
            runtime=Path(directory)/'runtime';runtime.mkdir(mode=0o700)
            env={**os.environ,'XDG_RUNTIME_DIR':str(runtime),'GVFS_DISABLE_FUSE':'1'}
            result=subprocess.run(['dbus-run-session','--','gjs','-m',str(ROOT/'tests/localsend_bridge.mjs')],capture_output=True,text=True,env=env,timeout=10)
            self.assertEqual(result.returncode,0,result.stderr)
            self.assertIn('PASS:',result.stdout)

if __name__=='__main__': unittest.main()
