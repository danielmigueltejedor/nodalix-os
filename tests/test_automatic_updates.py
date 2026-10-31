"""Automation preferences never execute privileged operations in these tests."""
import importlib.util
from importlib.machinery import SourceFileLoader
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock
import xml.etree.ElementTree as ET
from test_native_settings import settings

ROOT=Path(__file__).resolve().parents[1]
auto=settings.automatic_updates


class FakeSystemd:
    def __init__(self):self.states={};self.calls=[];self.fail=False;self.on_enable=None
    def __call__(self,args,**kwargs):
        self.calls.append(args)
        if '--user' in args:return subprocess.CompletedProcess(args,1,stdout='disabled\n')
        if args[1]=='show':
            rows=[]
            for name in args[4:]:
                enabled,active=self.states.get(name,(False,False))
                rows.append(f'Id={name}\nLoadState=loaded\nUnitFileState={"enabled" if enabled else "disabled"}\nActiveState={"active" if active else "inactive"}')
            return subprocess.CompletedProcess(args,0,stdout='\n\n'.join(rows))
        operation=args[1]
        if operation=='enable' and self.on_enable:self.on_enable()
        for name in args[2:]:
            if name.startswith('--'):continue
            enabled,active=self.states.get(name,(False,False))
            self.states[name]=(operation=='enable' if operation in ('enable','disable') else enabled,
                operation in ('start','enable') if '--now' in args or operation in ('start','stop') else active)
            if self.fail and operation=='enable' and kwargs.get('check'):
                self.fail=False
                raise subprocess.CalledProcessError(1,args,stderr='failure')
        return subprocess.CompletedProcess(args,0,stdout='')


class AutomaticUpdatesTests(unittest.TestCase):
    def setUp(self):
        directory=tempfile.TemporaryDirectory();self.addCleanup(directory.cleanup)
        self.path=Path(directory.name)/'preferences.json';self.systemd=FakeSystemd()

    def test_explicit_consent_is_persistent_before_timer_activation(self):
        self.systemd.on_enable=lambda:self.assertTrue(json.loads(self.path.read_text())['system']['enabled'])
        auto.configure('system',True,1000,self.path,self.systemd)
        data=auto.snapshot(1000,self.path,self.systemd)
        self.assertTrue(data['system']['enabled']);self.assertTrue(data['system']['active'])
        self.assertEqual(json.loads(self.path.read_text())['system']['owner_uid'],1000)
        self.assertEqual(self.path.stat().st_mode&0o777,0o644)
        self.assertNotIn('password',self.path.read_text())

    def test_disable_uses_existing_owner_and_only_stops_timers(self):
        auto.configure('system',True,1000,self.path,self.systemd)
        self.systemd.calls=[]
        auto.configure('system',False,1000,self.path,self.systemd)
        self.assertFalse(auto.snapshot(1000,self.path,self.systemd)['system']['enabled'])
        self.assertFalse(any(any(arg.endswith('.service') for arg in call) for call in self.systemd.calls))

    def test_other_user_cannot_disable_a_grant(self):
        auto.configure('system',True,1000,self.path,self.systemd)
        with self.assertRaises(PermissionError):auto.configure('system',False,1001,self.path,self.systemd)
        self.assertTrue(auto.snapshot(1000,self.path,self.systemd)['system']['enabled'])

    def test_failed_flatpak_activation_rolls_back_both_timers_and_consent(self):
        self.systemd.fail=True
        with self.assertRaises(subprocess.CalledProcessError):auto.configure('flatpak',True,1000,self.path,self.systemd)
        self.assertEqual(json.loads(self.path.read_text()),{})
        self.assertFalse(any(enabled or active for enabled,active in self.systemd.states.values()))

    def test_timer_state_is_authoritative_and_partial_setup_is_visible(self):
        self.path.write_text(json.dumps({'flatpak':{'enabled':False,'owner_uid':1000}}))
        self.systemd.states['nodalix-update-flatpak.timer']=(True,True)
        state=auto.snapshot(1000,self.path,self.systemd)['flatpak']
        self.assertTrue(state['partial']);self.assertFalse(state['enabled'])

    def test_only_fixed_timer_names_are_accepted(self):
        for bad in ('system;reboot','../system','all','apps',''):
            with self.assertRaises(ValueError):auto.configure(bad,True,1000,self.path,self.systemd)
        self.assertEqual(self.systemd.calls,[])

    def test_polkit_requires_authentication_only_for_enable(self):
        tree=ET.parse(ROOT/'gnome/settings/com.nodalix.settings.policy')
        actions={row.attrib['id']:row for row in tree.findall('action')}
        enabled=actions['com.nodalix.settings.automatic.enable']
        disabled=actions['com.nodalix.settings.automatic.disable']
        self.assertEqual(enabled.findtext('defaults/allow_active'),'auth_admin_keep')
        self.assertEqual(disabled.findtext('defaults/allow_active'),'yes')
        for action in (enabled,disabled):
            self.assertEqual(action.findtext('defaults/allow_inactive'),'no')
            self.assertEqual(action.findtext('defaults/allow_any'),'no')
        self.assertNotEqual(enabled.find('annotate').text,disabled.find('annotate').text)

    def test_scheduled_transactions_do_not_invoke_pkexec_or_reboot(self):
        for kind in auto.KINDS:
            service=(ROOT/f'updater/nodalix-update-{kind}.service').read_text()
            self.assertNotIn('pkexec',service)
            self.assertIn('update-transaction '+kind+' --automatic',service)
        transaction=(ROOT/'updater/nodalix-update-transaction').read_text()
        self.assertIn('--no-reboot-check',transaction)
        self.assertNotIn("'reboot'",transaction)
        user=(ROOT/'updater/nodalix-update-flatpak-user@.service').read_text()
        self.assertIn('User=%i',user);self.assertIn('--user --noninteractive',user)

    def test_scheduled_critical_changes_recommend_reboot_without_restarting(self):
        loader=SourceFileLoader('update_transaction',str(ROOT/'updater/nodalix-update-transaction'))
        spec=importlib.util.spec_from_loader(loader.name,loader)
        transaction=importlib.util.module_from_spec(spec);loader.exec_module(transaction)
        marker=self.path.parent/'reboot-recommended'
        transaction.recommend_reboot({'linux':'1'},{'linux':'1'},marker)
        self.assertFalse(marker.exists())
        transaction.recommend_reboot({'linux':'1'},{'linux':'2'},marker)
        self.assertEqual(json.loads(marker.read_text()),{'packages':['linux']})
        with mock.patch.object(transaction.subprocess,'run',side_effect=OSError('unavailable')):
            self.assertIsNone(transaction.critical_versions())

    def test_cancelled_authorization_returns_real_switch_state(self):
        service=settings.SettingsService();invocation=mock.Mock()
        thread=mock.Mock()
        with mock.patch.object(settings.threading,'Thread',return_value=thread) as factory:
            service.set_automatic('system',True,invocation)
        worker=factory.call_args.kwargs['target']
        with mock.patch.object(settings.subprocess,'run',return_value=subprocess.CompletedProcess([],126,stderr='Cancelled')), \
             mock.patch.object(auto,'snapshot',return_value={'system':{'enabled':False}}), \
             mock.patch.object(settings.GLib,'idle_add',side_effect=lambda cb:cb()):worker()
        self.assertFalse(service.state['automatic']['system']['enabled'])
        self.assertFalse(service.state['automatic_busy'])
        invocation.return_dbus_error.assert_called_once()


if __name__=='__main__':unittest.main()
