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

if __name__=='__main__': unittest.main()
