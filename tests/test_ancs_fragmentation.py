"""Regression tests for ANCS Data Source fragmentation and metadata."""
from __future__ import annotations

import ast
import logging
from pathlib import Path
import struct
import sys
import unittest
from unittest.mock import Mock

PHONE_SRC = Path(__file__).resolve().parents[1] / "phone-link" / "src"
sys.path.insert(0, str(PHONE_SRC))

from iphonebridge.ancs.constants import (
    CATEGORY_NAMES,
    CategoryID,
    CommandID,
    EventFlag,
    EventID,
    NotificationAttributeID,
)
from iphonebridge.ancs.events import AncsEvent
from iphonebridge.ancs.parsers import (
    AppAttributes,
    Notification,
    NotificationAttributes,
    app_attributes_response_length,
    build_get_app_attributes,
    build_get_notification_attributes,
    notification_attributes_response_length,
)


def _attr(attr_id: int, value: str) -> bytes:
    raw = value.encode("utf-8")
    return bytes([attr_id]) + struct.pack("<H", len(raw)) + raw


def _notification_frame(
    uid: int,
    *,
    app_id: str = "org.example.chat",
    title: str = "Hello",
    subtitle: str = "",
    message: str = "A message long enough to cross a small BLE MTU",
    positive: str | None = None,
    negative: str | None = None,
) -> bytes:
    payload = (
        bytes([CommandID.GetNotificationAttributes])
        + struct.pack("<I", uid)
        + _attr(NotificationAttributeID.AppIdentifier, app_id)
        + _attr(NotificationAttributeID.Title, title)
        + _attr(NotificationAttributeID.Subtitle, subtitle)
        + _attr(NotificationAttributeID.Message, message)
    )
    if positive is not None:
        payload += _attr(NotificationAttributeID.PositiveActionLabel, positive)
    if negative is not None:
        payload += _attr(NotificationAttributeID.NegativeActionLabel, negative)
    return payload


def _load_ancs_client():
    path = PHONE_SRC / "iphonebridge" / "ancs" / "client.py"
    tree = ast.parse(path.read_text())
    klass = next(
        node for node in tree.body
        if isinstance(node, ast.ClassDef) and node.name == "AncsClient"
    )
    module = ast.Module(
        body=[
            ast.ImportFrom(
                module="__future__",
                names=[ast.alias(name="annotations")],
                level=0,
            ),
            klass,
        ],
        type_ignores=[],
    )
    env = {
        "log": logging.getLogger("ancs-test"),
        "CommandID": CommandID,
        "EventID": EventID,
        "NotificationAttributeID": NotificationAttributeID,
        "CATEGORY_NAMES": CATEGORY_NAMES,
        "Notification": Notification,
        "NotificationAttributes": NotificationAttributes,
        "AppAttributes": AppAttributes,
        "AncsEvent": AncsEvent,
        "app_attributes_response_length": app_attributes_response_length,
        "notification_attributes_response_length": notification_attributes_response_length,
        "build_get_app_attributes": build_get_app_attributes,
        "build_get_notification_attributes": build_get_notification_attributes,
        "system_bus": Mock(),
    }
    exec(compile(ast.fix_missing_locations(module), str(path), "exec"), env)
    return env["AncsClient"]


class AncsFragmentationTests(unittest.TestCase):
    def test_notification_frame_waits_for_all_fragments(self):
        uid = 0x12345678
        frame = _notification_frame(uid)
        expected = (
            NotificationAttributeID.AppIdentifier,
            NotificationAttributeID.Title,
            NotificationAttributeID.Subtitle,
            NotificationAttributeID.Message,
        )

        for cut in range(len(frame)):
            with self.subTest(cut=cut):
                self.assertIsNone(
                    notification_attributes_response_length(frame[:cut], expected)
                )

        self.assertEqual(
            notification_attributes_response_length(frame, expected),
            len(frame),
        )
        attrs = NotificationAttributes.parse(frame[1:])
        self.assertEqual(attrs.id, uid)
        self.assertEqual(attrs.app_id, "org.example.chat")
        self.assertEqual(attrs.title, "Hello")

    def test_optional_action_labels_are_part_of_frame(self):
        uid = 7
        frame = _notification_frame(
            uid,
            positive="Reply",
            negative="Dismiss",
        )
        expected = (
            NotificationAttributeID.AppIdentifier,
            NotificationAttributeID.Title,
            NotificationAttributeID.Subtitle,
            NotificationAttributeID.Message,
            NotificationAttributeID.PositiveActionLabel,
            NotificationAttributeID.NegativeActionLabel,
        )

        self.assertEqual(
            notification_attributes_response_length(frame, expected),
            len(frame),
        )
        attrs = NotificationAttributes.parse(frame[1:])
        self.assertEqual(attrs.positive_action, "Reply")
        self.assertEqual(attrs.negative_action, "Dismiss")

    def test_app_attribute_frame_waits_for_complete_display_name(self):
        app_id = "org.example.chat"
        name = "Example Chat"
        frame = (
            bytes([CommandID.GetAppAttributes])
            + app_id.encode("utf-8")
            + b"\0"
            + _attr(0, name)
        )

        for cut in range(len(frame)):
            with self.subTest(cut=cut):
                self.assertIsNone(app_attributes_response_length(frame[:cut]))

        self.assertEqual(app_attributes_response_length(frame), len(frame))
        attrs = AppAttributes.parse(frame[1:])
        self.assertEqual(attrs.app_id, app_id)
        self.assertEqual(attrs.app_name, name)

    def test_control_point_requests_are_serialized(self):
        writes = []

        class FakeDbusException(Exception):
            def get_dbus_name(self):
                return "org.bluez.Error.Failed"

        class FakeCharacteristic:
            def WriteValue(self, value, _options):
                writes.append(bytes(value))

        class FakeBus:
            def get_object(self, _service, _path):
                return FakeCharacteristic()

        class FakeDbus:
            Byte = staticmethod(int)

            class exceptions:
                DBusException = FakeDbusException

            @staticmethod
            def Interface(obj, _iface):
                return obj

        AncsClient = _load_ancs_client()
        client = AncsClient("/org/bluez/hci0/dev_TEST", lambda _event: None)
        client._cp_path = "/org/bluez/hci0/dev_TEST/service/char"

        globals_ = client._pump_control_point.__globals__
        globals_["dbus"] = FakeDbus
        globals_["system_bus"] = FakeBus()

        first = Notification(
            id=1,
            type=EventID.NotificationAdded,
            flags=0,
            category=CategoryID.Social,
            category_count=1,
        )
        second = Notification(
            id=2,
            type=EventID.NotificationAdded,
            flags=0,
            category=CategoryID.Email,
            category_count=1,
        )

        client._request_attrs(first)
        client._request_attrs(second)

        self.assertEqual(len(writes), 1)
        self.assertEqual(client._cp_in_flight[1], 1)
        self.assertEqual(len(client._cp_queue), 1)
        self.assertEqual(client._cp_queue[0][1], 2)

        client._complete_control_point(
            CommandID.GetNotificationAttributes,
            1,
        )

        self.assertEqual(len(writes), 2)
        self.assertEqual(client._cp_in_flight[1], 2)
        self.assertEqual(client._cp_queue, [])

    def test_client_reassembles_fragments_and_preserves_silent_category(self):
        uid = 99
        frame = _notification_frame(uid)
        emitted = []
        AncsClient = _load_ancs_client()
        client = AncsClient("/org/bluez/hci0/dev_TEST", emitted.append)

        client._app_name_cache["org.example.chat"] = "Example Chat"
        client._notification_meta[uid] = Notification(
            id=uid,
            type=EventID.NotificationAdded,
            flags=EventFlag.Silent,
            category=CategoryID.Social,
            category_count=1,
        )

        split = max(6, len(frame) // 3)
        client._on_ds_changed(
            "org.bluez.GattCharacteristic1",
            {"Value": frame[:split]},
            [],
        )
        self.assertEqual(emitted, [])

        client._on_ds_changed(
            "org.bluez.GattCharacteristic1",
            {"Value": frame[split:]},
            [],
        )
        self.assertEqual(len(emitted), 1)
        self.assertEqual(emitted[0].category, "Social")
        self.assertTrue(emitted[0].is_silent)
        self.assertEqual(emitted[0].body, "A message long enough to cross a small BLE MTU")
        self.assertEqual(client._ds_buffer, bytearray())
        self.assertNotIn(uid, client._notification_meta)


if __name__ == "__main__":
    unittest.main()
