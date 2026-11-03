"""Regression checks for Nodalix's pinned Xbox Bluetooth DKMS package."""

import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
PKG = ROOT / "packaging" / "nodalix-xpadneo-dkms"
PATCH = PKG / "preserve-ble-identity.patch"
CONF = PKG / "99-nodalix-xpadneo-real-id.conf"


class XboxDriverPackageTests(unittest.TestCase):
    def test_patch_is_strict_reversible_and_scoped(self):
        # Use the exact two contexts from the pinned upstream v0.10.5.
        original = (
            'MODULE_VERSION(XPADNEO_VERSION);\n'
            'MODULE_SOFTDEP("pre: uhid");\n\n'
            'static DEFINE_IDA(xpadneo_core_device_id_allocator);\n\n'
            '#ifndef USB_VENDOR_ID_MICROSOFT\n'
            '#define USB_VENDOR_ID_MICROSOFT 0x045e\n'
            '#endif\n\n'
            'static int core_probe(struct hid_device *hdev, const struct hid_device_id *id) {\n'
            '\txdata->original_vendor = hdev->vendor;\n'
            '\txdata->original_product = hdev->product;\n'
            '\txdata->original_version = hdev->version;\n'
            '\thdev->vendor = USB_VENDOR_ID_MICROSOFT;\n'
            '\thdev->product = 0x028E;\n'
            '\thdev->version = 0x00001130;\n\n'
            '\tif ((hdev->vendor != xdata->original_vendor) || (hdev->product != xdata->original_product))\n'
            '\t\thid_info(hdev,\n'
            '}\n'
        )
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / "hid-xpadneo/src/xpadneo/core.c"
            source.parent.mkdir(parents=True)
            source.write_text(original)
            command = ["patch", "--fuzz=0", "-p1", "-i", str(PATCH)]
            subprocess.run(command, cwd=tmp, check=True, capture_output=True, text=True)
            patched = source.read_text()
            self.assertIn("param_disable_pid_spoofing", patched)
            self.assertIn("hdev->product == 0x0B20", patched)
            self.assertIn("hdev->product = 0x028E", patched)
            self.assertIn("hdev->version = 0x00001130", patched)
            subprocess.run(["patch", "--fuzz=0", "-R", "-p1", "-i", str(PATCH)],
                           cwd=tmp, check=True, capture_output=True, text=True)
            self.assertEqual(original, source.read_text())

    def test_patch_rejects_unexpected_upstream_code(self):
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / "hid-xpadneo/src/xpadneo/core.c"
            source.parent.mkdir(parents=True)
            source.write_text("upstream changed the core driver\n")
            command = ["patch", "--fuzz=0", "-p1", "-i", str(PATCH)]
            result = subprocess.run(command, cwd=tmp, capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)

    def test_package_is_pinned_and_hashes_local_sources(self):
        spec = (PKG / "PKGBUILD").read_text()
        self.assertIn("pkgname=nodalix-xpadneo-dkms", spec)
        self.assertIn("conflicts=('xpadneo-dkms')", spec)
        self.assertIn('provides=("xpadneo-dkms=${pkgver}")', spec)
        self.assertIn("e8f103a77f777c2c54536dd947969cf228699c13", spec)
        self.assertIn("patch --fuzz=0", spec)
        self.assertIn("usr/lib/modprobe.d/99-nodalix-xpadneo-real-id.conf", spec)
        self.assertIn("usr/src/hid-xpadneo-v", spec)
        for path in (PATCH, CONF):
            self.assertIn(hashlib.sha256(path.read_bytes()).hexdigest(), spec)
        result = subprocess.run(["bash", "-n", str(PKG / "PKGBUILD")],
                                capture_output=True, text=True)
        self.assertEqual(0, result.returncode, result.stderr)

    def test_configuration_is_vendor_owned_and_opt_in_to_known_ble_pid(self):
        conf = CONF.read_text()
        self.assertIn("options hid_xpadneo disable_pid_spoofing=1", conf)
        self.assertEqual(1, conf.count("options hid_xpadneo"))
        patch = PATCH.read_text()
        self.assertIn("hdev->vendor == USB_VENDOR_ID_MICROSOFT", patch)
        self.assertIn("hdev->product == 0x0B20", patch)

    def test_release_ship_and_builder(self):
        definition = json.loads((ROOT / "release/components.json").read_text())
        entries = [c for c in definition["components"] if c["id"] == "xbox-driver"]
        self.assertEqual(1, len(entries))
        self.assertEqual("nodalix-xpadneo-dkms", entries[0]["package"])
        self.assertTrue(entries[0]["required"])
        self.assertEqual("system", entries[0]["restart"])
        builder = (ROOT / "tools/build-packages.sh").read_text()
        self.assertIn('makepkg_one "$xpadneo_dir"', builder)
        self.assertIn("preserve-ble-identity.patch", builder)


if __name__ == "__main__":
    unittest.main()
