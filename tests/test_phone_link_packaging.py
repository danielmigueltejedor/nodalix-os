from __future__ import annotations

import json
import py_compile
import re
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PHONE_LINK = ROOT / "phone-link"


class PhoneLinkPackagingTests(unittest.TestCase):
    def test_python_sources_compile(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            for source in (PHONE_LINK / "src/iphonebridge").rglob("*.py"):
                target = Path(directory) / f"{source.stem}.pyc"
                py_compile.compile(str(source), cfile=str(target), doraise=True)

    def test_release_contains_phone_link_component(self) -> None:
        definition = json.loads((ROOT / "release/components.json").read_text(encoding="utf-8"))
        component = next(item for item in definition["components"] if item["id"] == "phone-link")
        self.assertEqual(component["package"], "nodalix-phone-link")
        self.assertTrue(component["required"])
        self.assertEqual(component["restart"], "system")

    def test_package_has_desktop_icon_and_services(self) -> None:
        desktop = (PHONE_LINK / "data/com.nodalix.PhoneLink.desktop").read_text(encoding="utf-8")
        self.assertIn("Name=Enlace móvil", desktop)
        self.assertIn("Icon=nodalix-phone-link", desktop)
        self.assertTrue((PHONE_LINK / "data/icons/nodalix-phone-link.svg").is_file())
        self.assertTrue((PHONE_LINK / "systemd/nodalix-phone-link.service").is_file())
        self.assertTrue((PHONE_LINK / "systemd/nodalix-phone-link-bluetooth.service").is_file())

    def test_no_personal_pairing_configuration_is_bundled(self) -> None:
        bundled = [path for path in PHONE_LINK.rglob("*") if path.is_file()]
        self.assertFalse(any(path.name == "local.env" for path in bundled))
        # The CLI source prints a template assignment as pairing guidance, so
        # scan distributable configuration rather than executable source.
        sensitive = re.compile(r"IPHONEBRIDGE_MAC\s*=", re.IGNORECASE)
        for path in bundled:
            if "src" in path.relative_to(PHONE_LINK).parts:
                continue
            if path.suffix.lower() in {".svg", ".png", ".jpg", ".jpeg"}:
                continue
            self.assertIsNone(sensitive.search(path.read_text(encoding="utf-8")), str(path))


if __name__ == "__main__":
    unittest.main()
