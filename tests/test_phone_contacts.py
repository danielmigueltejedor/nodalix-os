from __future__ import annotations

from pathlib import Path
import sys
import unittest

PHONE_SRC = Path(__file__).resolve().parents[1] / "phone-link" / "src"
sys.path.insert(0, str(PHONE_SRC))

from iphonebridge.eds_contacts import split_vcards, stable_uid


class PhoneContactSyncTests(unittest.TestCase):
    def test_split_vcards_preserves_complete_cards(self) -> None:
        blob = (
            "BEGIN:VCARD\r\nVERSION:3.0\r\nFN:Alice\r\n"
            "TEL;TYPE=CELL:+34111111111\r\nEND:VCARD\r\n"
            "BEGIN:VCARD\nVERSION:3.0\nFN:Bob\n"
            "TEL:222222222\nEND:VCARD\n"
        )
        cards = split_vcards(blob)
        self.assertEqual(len(cards), 2)
        self.assertTrue(all(card.startswith("BEGIN:VCARD") for card in cards))
        self.assertTrue(all(card.endswith("END:VCARD\r\n") for card in cards))

    def test_stable_uid_is_stable_for_same_contact(self) -> None:
        a = (
            "BEGIN:VCARD\r\nVERSION:3.0\r\nFN:Alice Example\r\n"
            "TEL;TYPE=CELL:+34 611 111 111\r\nEND:VCARD\r\n"
        )
        b = (
            "BEGIN:VCARD\nVERSION:3.0\nFN:Alice Example\n"
            "TEL;TYPE=CELL:+34611111111\nEND:VCARD\n"
        )
        self.assertEqual(stable_uid(a), stable_uid(b))
        self.assertTrue(stable_uid(a).startswith("nodalix-iphone-"))


if __name__ == "__main__":
    unittest.main()
