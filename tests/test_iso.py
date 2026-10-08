import json
import re
import runpy
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
policy = runpy.run_path(str(ROOT / 'iso/installer/finish-hardware.py'))['hardware_policy']


class IsoPolicyTests(unittest.TestCase):
    def test_vm_never_inherits_hosts_optimized_kernel(self):
        profile = {'tier': 'znver4', 'kernel': 'linux-cachyos', 'reason': 'Zen 4'}
        result = policy(profile, True)
        self.assertEqual(result['kernel'], 'linux')
        self.assertFalse(result['optimized'])

    def test_legacy_cpu_uses_compatible_kernel(self):
        profile = {'tier': 'x86-64', 'kernel': 'linux-cachyos', 'reason': 'generic'}
        self.assertEqual(policy(profile, False)['kernel'], 'linux')

    def test_supported_physical_handheld_retains_its_kernel(self):
        profile = {'tier': 'x86-64-v3', 'kernel': 'linux-cachyos-deckify', 'reason': 'handheld'}
        self.assertEqual(policy(profile, False)['kernel'], 'linux-cachyos-deckify')

    def test_release_definition_matches_actual_package_recipes(self):
        import sys
        sys.path.insert(0, str(ROOT / 'tools'))
        from versioning import to_pkgver
        definition = json.loads((ROOT / 'release/components.json').read_text())
        for component in definition['components']:
            text = (ROOT / 'packaging' / component['package'] / 'PKGBUILD').read_text()
            actual = re.search(r'^pkgver=(.+)$', text, re.M)[1].strip('\"\'')
            self.assertEqual(actual, to_pkgver(component['version']), component['package'])

    def test_new_user_provision_without_legacy_skeleton(self):
        import sys,types,tempfile
        from unittest import mock
        module=types.ModuleType('archinstall.default_profiles.profile')
        module.DisplayServerType=types.SimpleNamespace(Wayland='wayland')
        module.ProfileType=types.SimpleNamespace(DesktopEnv='desktop')
        module.Profile=object
        modules={name:types.ModuleType(name) for name in ('archinstall','archinstall.default_profiles')}
        modules['archinstall.default_profiles.profile']=module
        with mock.patch.dict(sys.modules,modules):
            profile=runpy.run_path(str(ROOT/'iso/installer/nodalix_profile.py'))
        cls=profile['NodalixProfile']
        with tempfile.TemporaryDirectory() as temp,mock.patch('subprocess.run') as command:
            target=Path(temp)
            cls.provision(object.__new__(cls),types.SimpleNamespace(target=target),[types.SimpleNamespace(username='fresh')])
            self.assertTrue((target/'home/fresh/.config').is_dir())
            self.assertTrue((target/'home/fresh/.local').is_dir())
            command.assert_called_once()
