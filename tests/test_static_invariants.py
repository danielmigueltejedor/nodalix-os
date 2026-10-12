from __future__ import annotations

import unittest
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SHELL = ROOT / "integrations"


class StaticInvariantTests(unittest.TestCase):





















    def test_legacy_migration_checks_pacman_ownership(self) -> None:
        migration = (ROOT / "updater" / "migrations" / "to_0_2_0.py").read_text(encoding="utf-8")
        registry = (ROOT / "updater" / "migrations" / "registry.py").read_text(encoding="utf-8")
        self.assertIn("has_unowned_legacy_files", migration)
        self.assertIn("pacman", registry)
        self.assertIn("-Qo", registry)
        self.assertIn("/var/lib/nodalix-updater", ROOT.joinpath("updater/nodalix-updater").read_text(encoding="utf-8"))


    def test_calendar_bridge_is_packaged_and_runs_after_vdirsyncer(self) -> None:
        pkgbuild = (ROOT / "packaging/nodalix-integrations/PKGBUILD").read_text(encoding="utf-8")
        service = (SHELL / "systemd/nodalix-icloud-sync.service").read_text(encoding="utf-8")
        bridge = SHELL / "scripts/nodalix-calendar-bridge.py"
        text = bridge.read_text(encoding="utf-8")

        self.assertTrue(bridge.is_file())
        self.assertIn("calendar-bridge", pkgbuild)
        self.assertIn("python-gobject", pkgbuild)
        self.assertIn("evolution-data-server", pkgbuild)

        export = "ExecStart=/usr/bin/nodalix-calendar-bridge --export"
        sync = "ExecStart=/usr/bin/vdirsyncer sync"
        import_ = "ExecStart=/usr/bin/nodalix-calendar-bridge --import"

        self.assertIn(export, service)
        self.assertIn(sync, service)
        self.assertIn(import_, service)
        self.assertLess(service.index(export), service.index(sync))
        self.assertLess(service.index(sync), service.index(import_))

        self.assertIn("nodalix-calendar", text)
        self.assertIn("EDataServer.Source.new_with_uid", text)
        self.assertIn("get_object_list_as_comps_sync", text)
        self.assertIn("ECal.ObjModType.THIS", text)
        self.assertIn("X-NODALIX-SOURCE-FILE:", text)




    def test_phone_link_reserves_bluez_hfp_for_ofono(self) -> None:
        dropin = (
            ROOT
            / "phone-link/systemd/50-nodalix-phone-link-bluez-hfp.conf"
        ).read_text(encoding="utf-8")
        pkgbuild = (
            ROOT / "packaging/nodalix-phone-link/PKGBUILD"
        ).read_text(encoding="utf-8")
        install = (
            ROOT
            / "packaging/nodalix-phone-link/nodalix-phone-link.install"
        ).read_text(encoding="utf-8")
        wireplumber = (
            ROOT / "phone-link/wireplumber/51-nodalix-phone-link.conf"
        ).read_text(encoding="utf-8")

        self.assertIn(
            "ExecStart=/usr/lib/bluetooth/bluetoothd --noplugin=hfp",
            dropin,
        )
        self.assertIn(
            "bluetooth.service.d/50-nodalix-phone-link-bluez-hfp.conf",
            pkgbuild,
        )
        self.assertIn("--noplugin=hfp", install)
        self.assertIn(
            '"bluez5.hfphsp-backend" = "ofono"',
            wireplumber,
        )
        self.assertIn('"hfp_hf"', wireplumber)
        self.assertIn('"a2dp_source"', wireplumber)
        self.assertNotIn('"a2dp_sink"', wireplumber)



    def test_phone_contacts_are_mirrored_to_gnome_contacts(self) -> None:
        bridge = (
            ROOT / "phone-link/src/iphonebridge/eds_contacts.py"
        ).read_text(encoding="utf-8")
        contacts = (
            ROOT / "phone-link/src/iphonebridge/contacts.py"
        ).read_text(encoding="utf-8")
        pkgbuild = (
            ROOT / "packaging/nodalix-phone-link/PKGBUILD"
        ).read_text(encoding="utf-8")

        self.assertIn('SOURCE_UID = "nodalix-iphone-contacts"', bridge)
        self.assertIn('SOURCE_NAME = "iPhone (Nodalix)"', bridge)
        self.assertIn("SOURCE_EXTENSION_ADDRESS_BOOK", bridge)
        self.assertIn("EBook.BookClient.connect_sync", bridge)
        self.assertIn("new_from_vcard_with_uid", bridge)
        self.assertIn("sync_gnome_contacts(blob)", contacts)
        self.assertIn("'evolution-data-server'", pkgbuild)
        self.assertIn("'ofono'", pkgbuild)
        self.assertIn("pkgver=0.2.5", pkgbuild)


if __name__ == "__main__":
    unittest.main()
