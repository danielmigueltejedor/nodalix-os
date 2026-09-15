import runpy
import unittest
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[1]
MODULE = runpy.run_path(str(ROOT / "updater" / "nodalix-hardware-profile"))


class HardwareProfileTests(unittest.TestCase):
    def detect(self, march="", levels=(), product="Desktop"):
        with mock.patch.dict(MODULE["detect"].__globals__, {
            "cpu_name": lambda: "Test CPU",
            "native_arch": lambda: march,
            "supports_hwcaps": lambda level: level in levels,
            "product_name": lambda: product,
        }):
            return MODULE["detect"]()

    def test_zen4_uses_znver4_repository(self):
        profile = self.detect(march="znver4", levels={"x86-64-v4", "x86-64-v3"})
        self.assertEqual(profile["repository"], "cachyos-znver4")
        self.assertEqual(profile["kernel"], "linux-cachyos")
        self.assertEqual(profile["fallback_kernel"], "linux")
        self.assertTrue(profile["arch_fallback_kept"])

    def test_v4_and_v3_fallbacks(self):
        self.assertEqual(self.detect(levels={"x86-64-v4", "x86-64-v3"})["repository"], "cachyos-v4")
        self.assertEqual(self.detect(levels={"x86-64-v3"})["repository"], "cachyos-v3")
        self.assertEqual(self.detect()["repository"], "cachyos")

    def test_handheld_selects_deckify_kernel(self):
        profile = self.detect(levels={"x86-64-v3"}, product="ROG Ally")
        self.assertEqual(profile["kernel"], "linux-cachyos-deckify")
        self.assertEqual(profile["headers"], "linux-cachyos-deckify-headers")

    def test_apply_resyncs_native_packages_without_a_shell_pipeline(self):
        source = (ROOT / "updater" / "nodalix-hardware-profile").read_text(encoding="utf-8")
        self.assertIn('["pacman", "-Qqn"]', source)
        self.assertIn('["pacman", "-S", "--needed", "--noconfirm", "-"]', source)
        self.assertIn('input="\\n".join(native_packages) + "\\n"', source)
        self.assertNotIn("shell=True", source)

    def test_boot_profile_sets_loader_file_and_persistent_efi_default(self):
        source = (ROOT / "updater" / "nodalix-hardware-profile").read_text(encoding="utf-8")
        self.assertIn('"default nodalix-cachyos.conf\\n"', source)
        self.assertIn('["bootctl", "set-default", target.name]', source)

    def test_polished_boot_options_replace_noisy_console_settings(self):
        options = MODULE["polished_boot_options"](
            "root=UUID=test rw quiet loglevel=3 systemd.show_status=auto rd.udev.log_level=3"
        )
        self.assertIn("root=UUID=test", options)
        self.assertIn("quiet", options)
        self.assertIn("loglevel=2", options)
        self.assertIn("systemd.show_status=false", options)
        self.assertIn("rd.systemd.show_status=false", options)
        self.assertIn("rd.udev.log_level=2", options)
        self.assertIn("udev.log_level=2", options)
        self.assertNotIn("loglevel=3", options)
        self.assertNotIn("systemd.show_status=auto", options)

    def test_polished_boot_options_are_idempotent(self):
        first = MODULE["polished_boot_options"]("root=/dev/test rw")
        self.assertEqual(MODULE["polished_boot_options"](first), first)


if __name__ == "__main__":
    unittest.main()
