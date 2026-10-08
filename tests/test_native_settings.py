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

class WallpaperActivationTests(unittest.TestCase):
    def test_request_keeps_other_extensions_and_cancels_pending_video(self):
        from unittest.mock import MagicMock
        prefs=MagicMock();values={'enabled-extensions':['OTHER'], 'disabled-extensions':['DISABLED',settings.HANABI_UUID]}
        prefs.get_strv.side_effect=lambda key:list(values[key])
        prefs.set_strv.side_effect=lambda key,value:values.__setitem__(key,list(value))
        with patch.object(settings.Gio.Settings,'new',return_value=prefs),patch.object(settings.Gio.Settings,'sync'):
            settings.request_wallpaper(True)
            self.assertEqual(values['enabled-extensions'],['OTHER',settings.HANABI_UUID])
            self.assertEqual(values['disabled-extensions'],['DISABLED'])
            settings.request_wallpaper(True)
            self.assertEqual(values['enabled-extensions'].count(settings.HANABI_UUID),1)
            settings.request_wallpaper(False)
            self.assertEqual(values['enabled-extensions'],['OTHER'])

    def test_new_extension_is_queued_without_claiming_playback(self):
        from unittest.mock import MagicMock
        prefs=MagicMock();prefs.get_string.return_value='/known.mp4'
        shell=MagicMock();shell.get_strv.return_value=[settings.HANABI_UUID]
        with patch.object(settings,'wallpaper_preferences',return_value=prefs),patch.object(settings.Gio.Settings,'new',return_value=shell),patch.object(settings,'extension_info',return_value={}):
            state=json.loads(settings.wallpaper_snapshot())
            self.assertTrue(state['requested']);self.assertFalse(state['active']);self.assertFalse(state['playing'])
            self.assertIn('Cierra sesión',state['message'])

    def test_enabled_extension_requires_a_renderer_before_reporting_active(self):
        from unittest.mock import MagicMock
        prefs=MagicMock();prefs.get_string.return_value='/known.mp4'
        shell=MagicMock();shell.get_strv.return_value=[settings.HANABI_UUID]
        bus=MagicMock();bus.call_sync.return_value=settings.GLib.Variant('(b)',(False,))
        with patch.object(settings,'wallpaper_preferences',return_value=prefs),patch.object(settings.Gio.Settings,'new',return_value=shell),patch.object(settings,'extension_info',return_value={'state':1}),patch.object(settings.Gio,'bus_get_sync',return_value=bus):
            state=json.loads(settings.wallpaper_snapshot())
            self.assertFalse(state['active']);self.assertFalse(state['playing'])

    def test_desktop_uses_full_resolution_still_instead_of_gallery_thumbnail(self):
        from unittest.mock import MagicMock
        import tempfile
        with tempfile.TemporaryDirectory() as folder:
            thumbnail=Path(folder)/'thumbnail.jpg';thumbnail.write_bytes(b'small')
            still=Path(folder)/'still.jpg';still.write_bytes(b'large')
            video=MagicMock();background=MagicMock();service=settings.SettingsService()
            with patch.object(settings,'poster_for',return_value=still),patch.object(settings,'sync_login'),patch.object(settings.threading.Thread,'start'),patch.object(settings,'catalog',return_value=[{'path':'/known.mp4','preview':str(thumbnail),'still':str(still)}]),patch.object(settings,'wallpaper_preferences',return_value=video),patch.object(settings,'request_wallpaper'),patch.object(settings,'extension_info',return_value={}),patch.object(settings.Gio.Settings,'new',return_value=background):
                service.wallpaper('/known.mp4')
                background.set_string.assert_any_call('picture-uri',still.as_uri())
                background.set_string.assert_any_call('picture-uri-dark',still.as_uri())
                video.set_int.assert_any_call('startup-delay',0)
                self.assertNotIn(('picture-uri',thumbnail.as_uri()),[tuple(c.args) for c in background.set_string.call_args_list])

    def test_selection_persists_next_login_and_applies_the_matching_poster(self):
        from unittest.mock import MagicMock
        import tempfile
        with tempfile.TemporaryDirectory() as folder:
            poster=Path(folder)/'poster.jpg';poster.write_bytes(b'poster')
            items=[{'path':'/known.mp4','preview':str(poster)}]
            video=MagicMock();background=MagicMock();service=settings.SettingsService()
            with patch.object(settings,'poster_for',return_value=poster),patch.object(settings,'sync_login'),patch.object(settings.threading.Thread,'start'),patch.object(settings,'catalog',return_value=items),patch.object(settings,'wallpaper_preferences',return_value=video),patch.object(settings,'request_wallpaper') as requested,patch.object(settings,'extension_info',return_value={}),patch.object(settings.Gio.Settings,'new',return_value=background),patch.object(service,'extension') as enable:
                service.wallpaper('/known.mp4')
                requested.assert_called_once_with(True);enable.assert_not_called()
                video.set_string.assert_called_once_with('video-path','/known.mp4')
                background.set_string.assert_any_call('picture-uri',poster.as_uri())
                background.set_string.assert_any_call('picture-uri-dark',poster.as_uri())

if __name__=='__main__': unittest.main()

class RestartTests(unittest.TestCase):
    def test_restart_uses_logind_and_completes_one_reply(self):
        from unittest.mock import MagicMock
        service=settings.SettingsService();invocation=MagicMock();bus=MagicMock()
        with patch.object(settings.Gio,'bus_get_sync',return_value=bus):
            service.method_call(None,None,None,None,'Restart',None,invocation)
        bus.call_sync.assert_called_once()
        self.assertEqual(bus.call_sync.call_args.args[3],'Reboot')
        invocation.return_value.assert_called_once_with(None)
    def test_restart_rejected_while_installing(self):
        from unittest.mock import MagicMock
        service=settings.SettingsService();service.state['busy']=True;invocation=MagicMock()
        with patch.object(settings.Gio,'bus_get_sync') as bus:
            service.method_call(None,None,None,None,'Restart',None,invocation)
        bus.assert_not_called();invocation.return_dbus_error.assert_called_once()
