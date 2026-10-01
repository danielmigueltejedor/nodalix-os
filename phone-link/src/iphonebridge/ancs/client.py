"""ANCS GATT client — subscribes to the three ANCS characteristics on a
paired iPhone, decodes Notification Source events, writes
GetNotificationAttributes commands to Control Point, parses Data Source
responses, and emits AncsEvents.

Design notes (from bmh129/ancs4linux + iphonebridge's Phase 0):

- We don't trust Device1.ServicesResolved as a readiness signal because
  BlueZ flips it true after BR/EDR SDP, before BLE GATT enumerates. Instead,
  we listen for ObjectManager.InterfacesAdded and wait for all three ANCS
  characteristic UUIDs to show up under the target iPhone's device path.

- Notification Source and Data Source prefer BlueZ AcquireNotify so every
  GATT packet is consumed directly from its SOCK_SEQPACKET fd. StartNotify +
  PropertiesChanged remains a fallback for BlueZ/device combinations where
  AcquireNotify is unavailable. Control Point is write-only.

- For each NotificationAdded/Modified event we get on NS, we synthesize a
  GetNotificationAttributes packet asking for AppIdentifier+Title+Subtitle+
  Message (plus positive/negative action labels if the iPhone declared
  them) and write it to CP. The response comes back on DS.

- App display names are looked up lazily via GetAppAttributes and cached.
"""
from __future__ import annotations

import logging
import os
from collections.abc import Callable

import dbus
import dbus.exceptions
from gi.repository import GLib

from iphonebridge.ancs.constants import (
    ANCS_CHAR_UUIDS,
    CATEGORY_NAMES,
    CONTROL_POINT_CHAR,
    DATA_SOURCE_CHAR,
    NOTIFICATION_SOURCE_CHAR,
    CommandID,
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
from iphonebridge.bus import system_bus

log = logging.getLogger(__name__)


def _char_path_to_device_path(char_path: str) -> str:
    """Drop the last two segments: …/hciN/dev_XX/serviceYYYY/charZZZZ → …/dev_XX."""
    return "/".join(char_path.rsplit("/", 2)[:-2])


class AncsClient:
    """Tracks ANCS characteristics under one target device and subscribes
    when all three are present.

    Idempotent: calling start() multiple times is harmless; if chars are
    already present at start time, we hook them immediately.
    """

    def __init__(
        self,
        device_path: str,
        on_event: Callable[[AncsEvent], None],
    ) -> None:
        self.device_path = device_path
        self.on_event = on_event

        # Char path slots — set as InterfacesAdded fires
        self._ns_path: str | None = None
        self._ds_path: str | None = None
        self._cp_path: str | None = None
        self._notify_started = False

        # In-flight Data Source state. ANCS responses can span several GATT
        # notifications, so keep the stream and the original Notification Source
        # metadata until the corresponding attributes have been fully reassembled.
        self._ds_buffer = bytearray()
        self._notification_meta: dict[int, Notification] = {}

        # ANCS uses one shared Data Source stream for Control Point responses.
        # Keep exactly one request in flight so fragmented responses can never
        # overlap and become ambiguous on the stream.
        self._cp_queue: list[tuple[int, int | str, bytes]] = []
        self._cp_in_flight: tuple[int, int | str, bytes] | None = None
        self._notification_requests_pending: set[int] = set()

        # App display names are stable enough to cache across ANCS reconnects.
        self._app_name_cache: dict[str, str] = {}
        self._pending_app_lookups: dict[str, list[NotificationAttributes]] = {}
        self._app_lookup_in_flight: set[str] = set()

        # Permanent ObjectManager subscriptions + per-GATT-session subscriptions.
        self._manager_matches: list = []
        self._char_matches: list = []
        self._notify_fds: dict[str, tuple[int, int]] = {}
        self._start_notify_paths: set[str] = set()

    # ---- lifecycle ------------------------------------------------------

    def start(self) -> None:
        log.info("ANCS client starting; watching %s", self.device_path)
        om = dbus.Interface(
            system_bus.get_object("org.bluez", "/"),
            "org.freedesktop.DBus.ObjectManager",
        )
        self._manager_matches.append(
            om.connect_to_signal("InterfacesAdded", self._on_iface_added)
        )
        self._manager_matches.append(
            om.connect_to_signal("InterfacesRemoved", self._on_iface_removed)
        )
        # Sweep current state — ANCS chars may already exist if we're
        # restarting against a live pair.
        managed = om.GetManagedObjects()
        for path, ifaces in managed.items():
            self._on_iface_added(path, ifaces)

    def stop(self) -> None:
        log.info("ANCS client stopping")
        for m in self._manager_matches:
            try:
                m.remove()
            except Exception:
                pass
        self._manager_matches = []
        self._reset_subscription(stop_notify=True)
        self._ns_path = self._ds_path = self._cp_path = None
        self._notify_started = False

    @property
    def active(self) -> bool:
        return self._notify_started

    def _reset_subscription(self, *, stop_notify: bool = False) -> None:
        """Drop per-GATT-session transports so reconnects cannot duplicate events."""
        for match in self._char_matches:
            try:
                match.remove()
            except Exception:
                pass
        self._char_matches = []

        for path, (fd, watch_id) in list(self._notify_fds.items()):
            try:
                GLib.source_remove(watch_id)
            except Exception:
                pass
            try:
                os.close(fd)
            except OSError:
                pass
            self._notify_fds.pop(path, None)

        if stop_notify:
            for path in list(self._start_notify_paths):
                try:
                    dbus.Interface(
                        system_bus.get_object("org.bluez", path),
                        "org.bluez.GattCharacteristic1",
                    ).StopNotify()
                except dbus.exceptions.DBusException:
                    pass
        self._start_notify_paths.clear()
        self._notify_started = False
        self._ds_buffer.clear()
        self._notification_meta.clear()
        self._cp_queue.clear()
        self._cp_in_flight = None
        self._notification_requests_pending.clear()
        self._pending_app_lookups.clear()
        self._app_lookup_in_flight.clear()

    # ---- ObjectManager event handlers -----------------------------------

    def _on_iface_added(self, path, ifaces):
        path_s = str(path)
        char = ifaces.get("org.bluez.GattCharacteristic1")
        if char is None:
            return
        uuid = str(char.get("UUID", "")).lower()
        if uuid not in ANCS_CHAR_UUIDS:
            return
        if _char_path_to_device_path(path_s) != self.device_path:
            return
        if uuid == NOTIFICATION_SOURCE_CHAR:
            self._ns_path = path_s
            log.info("ANCS Notification Source found: %s", path_s)
        elif uuid == DATA_SOURCE_CHAR:
            self._ds_path = path_s
            log.info("ANCS Data Source found:         %s", path_s)
        elif uuid == CONTROL_POINT_CHAR:
            self._cp_path = path_s
            log.info("ANCS Control Point found:       %s", path_s)
        self._try_subscribe()

    def _on_iface_removed(self, path, ifaces):
        path_s = str(path)
        lost_ancs = False
        for attr in ("_ns_path", "_ds_path", "_cp_path"):
            if getattr(self, attr) == path_s:
                setattr(self, attr, None)
                lost_ancs = True
                log.warning("ANCS char gone: %s", path_s)
        if lost_ancs:
            self._reset_subscription()

    def _subscribe_characteristic(
        self,
        path: str,
        dbus_callback,
        packet_callback,
        label: str,
    ) -> bool:
        char = dbus.Interface(
            system_bus.get_object("org.bluez", path),
            "org.bluez.GattCharacteristic1",
        )

        try:
            # BlueZ may need longer than dbus-python's ~25 s default
            # while the iPhone finishes LE/ATT setup.  A default-timeout
            # NoReply can still leave NotifyAcquired=true, after which a
            # StartNotify fallback is rejected with NotPermitted and the
            # usable fd reply is effectively lost.  Wait long enough for
            # the AcquireNotify reply itself instead.
            fd_obj, mtu = char.AcquireNotify(
                dbus.Dictionary({}, signature="sv"),
                timeout=60.0,
            )
            fd = fd_obj.take() if hasattr(fd_obj, "take") else int(fd_obj)
            os.set_blocking(fd, False)
            watch_id = GLib.io_add_watch(
                fd,
                GLib.PRIORITY_DEFAULT,
                GLib.IO_IN | GLib.IO_HUP | GLib.IO_ERR | GLib.IO_NVAL,
                self._on_notify_fd,
                path,
                packet_callback,
                int(mtu),
            )
            self._notify_fds[path] = (fd, watch_id)
            log.info(
                "ANCS %s using AcquireNotify fd=%d mtu=%d",
                label,
                fd,
                int(mtu),
            )
            return True
        except dbus.exceptions.DBusException as e:
            error_name = e.get_dbus_name()
            if error_name in {
                "org.freedesktop.DBus.Error.NoReply",
                "org.bluez.Error.NotPermitted",
            }:
                # Do not immediately call StartNotify here.  On real iPhones
                # BlueZ can complete AcquireNotify just after a client timeout,
                # or briefly retain the previous notify acquisition while the
                # LE/GATT session is being rebuilt.  In both cases StartNotify
                # is rejected and only adds churn.  Let the daemon retry the
                # direct acquisition once BlueZ settles.
                log.warning(
                    "ANCS %s AcquireNotify deferred (%s); not falling back "
                    "to StartNotify",
                    label,
                    error_name,
                )
                return False
            log.info(
                "ANCS %s AcquireNotify unavailable (%s); falling back to StartNotify",
                label,
                error_name,
            )
        except (OSError, ValueError, TypeError) as e:
            log.warning(
                "ANCS %s AcquireNotify setup failed (%s); falling back to StartNotify",
                label,
                e,
            )

        try:
            char.StartNotify()
        except dbus.exceptions.DBusException as e:
            log.warning(
                "ANCS %s StartNotify failed: %s",
                label,
                e.get_dbus_name(),
            )
            return False

        self._start_notify_paths.add(path)
        self._char_matches.append(
            system_bus.add_signal_receiver(
                dbus_callback,
                dbus_interface="org.freedesktop.DBus.Properties",
                signal_name="PropertiesChanged",
                path=path,
            )
        )
        log.info("ANCS %s using StartNotify fallback", label)
        return True

    def _on_notify_fd(
        self,
        fd: int,
        condition,
        path: str,
        packet_callback,
        mtu: int,
    ) -> bool:
        if condition & (GLib.IO_HUP | GLib.IO_ERR | GLib.IO_NVAL):
            log.info("ANCS notify fd closed for %s", path)
            self._notify_fds.pop(path, None)
            try:
                os.close(fd)
            except OSError:
                pass
            self._notify_started = False
            return False

        try:
            while True:
                packet = os.read(fd, max(65535, mtu))
                if not packet:
                    self._notify_fds.pop(path, None)
                    try:
                        os.close(fd)
                    except OSError:
                        pass
                    self._notify_started = False
                    return False
                log.debug(
                    "ANCS notify fd packet: path=%s len=%d",
                    path,
                    len(packet),
                )
                packet_callback(packet)
        except BlockingIOError:
            return True
        except OSError as e:
            log.warning("ANCS notify fd read failed for %s: %s", path, e)
            self._notify_fds.pop(path, None)
            try:
                os.close(fd)
            except OSError:
                pass
            self._notify_started = False
            return False

    def _try_subscribe(self) -> None:
        if self._notify_started:
            return
        if not (self._ns_path and self._ds_path and self._cp_path):
            return

        if not self._subscribe_characteristic(
            self._ns_path,
            self._on_ns_changed,
            self._handle_ns_value,
            "Notification Source",
        ):
            self._reset_subscription(stop_notify=True)
            return

        if not self._subscribe_characteristic(
            self._ds_path,
            self._on_ds_changed,
            self._handle_ds_value,
            "Data Source",
        ):
            self._reset_subscription(stop_notify=True)
            return

        self._notify_started = True
        log.info("ANCS subscription active for %s", self.device_path)

    # ---- Notification Source: new/modified/removed events --------------

    def _on_ns_changed(self, iface, changed, _invalidated):
        if iface != "org.bluez.GattCharacteristic1":
            return
        value = changed.get("Value")
        if value is None:
            return
        self._handle_ns_value(bytes(value))

    def _handle_ns_value(self, value: bytes) -> None:
        try:
            n = Notification.parse(value)
        except Exception as e:
            log.error("NS parse failed: %s", e)
            return
        # Skip pre-existing (notifications that already existed on the
        # iPhone at our connect time — too noisy on initial subscribe).
        if n.is_preexisting:
            log.debug("ANCS preexisting event uid=%d cat=%d — skipping",
                      n.id, n.category)
            return
        if n.type == EventID.NotificationRemoved:
            self._notification_meta.pop(n.id, None)
            self._notification_requests_pending.discard(n.id)
            self._cp_queue = [
                item for item in self._cp_queue
                if not (
                    item[0] == CommandID.GetNotificationAttributes
                    and item[1] == n.id
                )
            ]
            log.debug("ANCS removed uid=%d", n.id)
            return
        # Added or Modified → request full attrs. Keep the NS packet so the
        # eventual event retains its real category and Silent/action flags.
        self._request_attrs(n)

    def _queue_control_point(
        self,
        command: int,
        key: int | str,
        packet: bytes,
    ) -> None:
        self._cp_queue.append((int(command), key, packet))
        self._pump_control_point()

    def _pump_control_point(self) -> None:
        if self._cp_in_flight is not None or not self._cp_path or not self._cp_queue:
            return

        command, key, packet = self._cp_queue.pop(0)
        self._cp_in_flight = (command, key, packet)

        try:
            dbus.Interface(
                system_bus.get_object("org.bluez", self._cp_path),
                "org.bluez.GattCharacteristic1",
            ).WriteValue([dbus.Byte(b) for b in packet], {})
        except dbus.exceptions.DBusException as e:
            self._cp_in_flight = None
            if command == CommandID.GetNotificationAttributes:
                self._notification_requests_pending.discard(int(key))
                self._notification_meta.pop(int(key), None)
            elif command == CommandID.GetAppAttributes:
                self._app_lookup_in_flight.discard(str(key))
            log.warning("CP WriteValue failed: %s", e.get_dbus_name())
            self._pump_control_point()

    def _complete_control_point(self, command: int, key: int | str) -> None:
        current = self._cp_in_flight
        if current is None:
            log.debug(
                "ANCS response arrived with no Control Point request in flight: "
                "command=%s key=%r",
                command,
                key,
            )
            return

        current_command, current_key, _packet = current
        if current_command != int(command) or current_key != key:
            log.warning(
                "ANCS response does not match in-flight request: "
                "got command=%s key=%r, expected command=%s key=%r",
                command,
                key,
                current_command,
                current_key,
            )
            return

        if current_command == CommandID.GetNotificationAttributes:
            self._notification_requests_pending.discard(int(current_key))
        elif current_command == CommandID.GetAppAttributes:
            self._app_lookup_in_flight.discard(str(current_key))

        self._cp_in_flight = None
        self._pump_control_point()

    def _request_attrs(self, n: Notification) -> None:
        if not self._cp_path:
            return

        # A Modified event for the same UID can arrive while its first request
        # is still queued/in flight. Preserve the newest metadata, but do not
        # issue a second overlapping Control Point request for the same UID.
        self._notification_meta[n.id] = n
        if n.id in self._notification_requests_pending:
            return

        pkt = build_get_notification_attributes(
            n.id,
            want_positive=n.has_positive_action,
            want_negative=n.has_negative_action,
        )
        self._notification_requests_pending.add(n.id)
        self._queue_control_point(
            CommandID.GetNotificationAttributes,
            n.id,
            pkt,
        )

    # ---- Data Source: responses to our CP writes ------------------------

    def _on_ds_changed(self, iface, changed, _invalidated):
        if iface != "org.bluez.GattCharacteristic1":
            return
        value = changed.get("Value")
        if value is None:
            return
        self._handle_ds_value(bytes(value))

    def _handle_ds_value(self, value: bytes) -> None:
        # A Data Source response is a byte stream, not one response per GATT
        # notification. Apple explicitly permits fragmentation at the negotiated
        # MTU, so append every fragment and drain only complete ANCS frames.
        self._ds_buffer.extend(value)
        self._drain_ds_buffer()

    def _drain_ds_buffer(self) -> None:
        while self._ds_buffer:
            command = self._ds_buffer[0]

            try:
                if command == CommandID.GetNotificationAttributes:
                    if len(self._ds_buffer) < 5:
                        return

                    notification_id = int.from_bytes(
                        self._ds_buffer[1:5],
                        "little",
                    )
                    source = self._notification_meta.get(notification_id)
                    if source is None:
                        # Every request issued by this client records its NS
                        # metadata first. An unknown UID therefore belongs to a
                        # stale/foreign ANCS transaction and cannot be framed
                        # safely because we do not know which optional attrs
                        # were requested.
                        log.warning(
                            "dropping ANCS Data Source response for unknown uid=%d",
                            notification_id,
                        )
                        self._ds_buffer.clear()
                        return

                    expected = [
                        NotificationAttributeID.AppIdentifier,
                        NotificationAttributeID.Title,
                        NotificationAttributeID.Subtitle,
                        NotificationAttributeID.Message,
                    ]
                    if source.has_positive_action:
                        expected.append(NotificationAttributeID.PositiveActionLabel)
                    if source.has_negative_action:
                        expected.append(NotificationAttributeID.NegativeActionLabel)

                    frame_len = notification_attributes_response_length(
                        bytes(self._ds_buffer),
                        tuple(expected),
                    )
                    if frame_len is None:
                        return

                    frame = bytes(self._ds_buffer[:frame_len])
                    del self._ds_buffer[:frame_len]
                    attrs = NotificationAttributes.parse(frame[1:])
                    self._handle_notification_attrs(attrs)
                    self._complete_control_point(
                        CommandID.GetNotificationAttributes,
                        attrs.id,
                    )
                    continue

                if command == CommandID.GetAppAttributes:
                    frame_len = app_attributes_response_length(
                        bytes(self._ds_buffer)
                    )
                    if frame_len is None:
                        return

                    frame = bytes(self._ds_buffer[:frame_len])
                    del self._ds_buffer[:frame_len]
                    app_attrs = AppAttributes.parse(frame[1:])
                    self._handle_app_attrs(app_attrs)
                    self._complete_control_point(
                        CommandID.GetAppAttributes,
                        app_attrs.app_id,
                    )
                    continue

                raise ValueError(f"unknown Data Source command {command}")

            except ValueError as e:
                log.error("ANCS Data Source stream parse failed: %s", e)
                self._ds_buffer.clear()
                return

    def _handle_notification_attrs(self, attrs: NotificationAttributes) -> None:
        # If we don't have the app's display name yet, queue and ask the
        # iPhone for it. Otherwise emit immediately.
        if attrs.app_id in self._app_name_cache:
            self._emit(attrs, self._app_name_cache[attrs.app_id])
        else:
            self._pending_app_lookups.setdefault(attrs.app_id, []).append(attrs)
            self._request_app_name(attrs.app_id)

    def _request_app_name(self, app_id: str) -> None:
        if not self._cp_path or app_id in self._app_lookup_in_flight:
            return
        self._app_lookup_in_flight.add(app_id)
        self._queue_control_point(
            CommandID.GetAppAttributes,
            app_id,
            build_get_app_attributes(app_id),
        )

    def _handle_app_attrs(self, app_attrs: AppAttributes) -> None:
        self._app_lookup_in_flight.discard(app_attrs.app_id)
        self._app_name_cache[app_attrs.app_id] = app_attrs.app_name
        pending = self._pending_app_lookups.pop(app_attrs.app_id, [])
        for attrs in pending:
            self._emit(attrs, app_attrs.app_name)

    def _emit(self, attrs: NotificationAttributes, app_name: str) -> None:
        source = self._notification_meta.pop(attrs.id, None)
        event = AncsEvent(
            notification_id=attrs.id,
            device_path=self.device_path,
            app_id=attrs.app_id,
            app_name=app_name,
            title=attrs.title,
            subtitle=attrs.subtitle,
            body=attrs.message,
            category=CATEGORY_NAMES.get(
                source.category if source is not None else 0,
                "Other",
            ),
            is_silent=source.is_silent if source is not None else False,
            is_preexisting=(
                source.is_preexisting if source is not None else False
            ),
            positive_action=attrs.positive_action,
            negative_action=attrs.negative_action,
        )
        log.info(
            "ANCS event: app=%r title=%r body=%r",
            event.app_name or event.app_id,
            (event.title or "")[:40],
            (event.body or "")[:60],
        )
        try:
            self.on_event(event)
        except Exception:
            log.exception("on_event callback raised")
