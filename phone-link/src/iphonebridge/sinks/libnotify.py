"""Desktop notification sink via org.freedesktop.Notifications.

Body format: title = display sender (contact name or phone number),
             body  = SMS text (truncated at ~280 chars to avoid huge popups).

Persistence model — notifications stay visible until ONE of:
  • The user dismisses the popup (clicks/swipes)        → we mark-read on iPhone
  • The iPhone marks the message read (user opens it)  → we auto-close popup
That way an unread message is never "missed" on the desktop side.

Read-state sync:
  Linux dismiss → MAP Message1.Properties.Set(Read=true)  → iPhone marks read
  iPhone reads  → MAP PropertiesChanged(Read=true)         → we close popup

Empirical on iOS 26.5: both directions work. iPhone propagates Read=true
back over MAP within a few seconds of opening the Messages app.
"""
from __future__ import annotations

import logging
import os
from pathlib import Path

import dbus
import dbus.exceptions
from gi.repository import GLib

from iphonebridge.ancs.events import AncsEvent
from iphonebridge.bus import session_bus
from iphonebridge.events import SmsEvent

log = logging.getLogger(__name__)

_APP_NAME = "Enlace móvil"
_CALL_APP_NAME = "Enlace móvil"
_BODY_LIMIT = 280

# ANCS only provides an iOS bundle identifier, not artwork. Resolve common
# apps to freedesktop icon names supplied by Nodalix's icon theme. Unknown
# apps fall back by category instead of showing a missing-glyph square.
_APP_ICONS = {
    "io.robbie.homeassistant": "home-assistant",
    "com.facebook.facebook": "web-facebook",
    "com.facebook.messenger": "fbmessenger",
    "net.whatsapp.whatsapp": "whatsapp",
    "ph.telegra.telegram": "telegram",
    "org.whispersystems.signal": "signal-desktop",
    "com.hammerandchisel.discord": "discord",
    "com.tinyspeck.chatlyio": "slack",
    "com.burbn.instagram": "instagram",
    "com.google.ios.youtube": "youtube",
    "com.apple.mobilemail": "mail-app-symbolic",
    "com.apple.mobilecal": "calendar",
    "com.apple.mobilesms": "chat-message-new-symbolic",
    "com.apple.facetime": "call-start-symbolic",
}

_SYSTEM_ICONS = {
    "phone-symbolic": "/usr/share/icons/Adwaita/symbolic/devices/phone-symbolic.svg",
    "chat-message-new-symbolic": "/usr/share/icons/Adwaita/symbolic/actions/chat-message-new-symbolic.svg",
    "mail-unread-symbolic": "/usr/share/icons/Adwaita/symbolic/status/mail-unread-symbolic.svg",
    "call-incoming-symbolic": "/usr/share/icons/Adwaita/symbolic/status/call-incoming-symbolic.svg",
    "call-outgoing-symbolic": "/usr/share/icons/Adwaita/symbolic/status/call-outgoing-symbolic.svg",
    "call-start-symbolic": "/usr/share/icons/Adwaita/symbolic/actions/call-start-symbolic.svg",
    "calendar-app-symbolic": "/usr/share/icons/Adwaita/symbolic/mimetypes/x-office-calendar-symbolic.svg",
}


def _icon(icon_name: str) -> str:
    """Return a concrete icon file so Quickshell is theme-provider agnostic."""
    app_icon = (
        Path.home() / ".local/share/icons/WhiteSur/apps/scalable"
        / f"{icon_name}.svg"
    )
    if app_icon.is_file():
        return str(app_icon)
    system_icon = _SYSTEM_ICONS.get(icon_name, _SYSTEM_ICONS["phone-symbolic"])
    return system_icon if Path(system_icon).is_file() else "phone-symbolic"


def _event_icon(event: AncsEvent) -> str:
    app_id = (event.app_id or "").strip().lower()
    if app_id in _APP_ICONS:
        return _icon(_APP_ICONS[app_id])

    app = f"{event.app_name or ''} {app_id}".lower()
    name_markers = (
        ("home assistant", "home-assistant"),
        ("whatsapp", "whatsapp"),
        ("telegram", "telegram"),
        ("signal", "signal-desktop"),
        ("discord", "discord"),
        ("slack", "slack"),
        ("instagram", "instagram"),
        ("facebook", "web-facebook"),
        ("youtube", "youtube"),
    )
    for marker, icon in name_markers:
        if marker in app:
            return _icon(icon)

    category = (event.category or "").lower()
    if "call" in category:
        return _icon("call-incoming-symbolic")
    if category == "email":
        return _icon("mail-unread-symbolic")
    if category == "social":
        return _icon("chat-message-new-symbolic")
    if category == "schedule":
        return _icon("calendar-app-symbolic")
    return _icon("phone-symbolic")


def _call_text(english: str, spanish: str) -> str:
    """Use the desktop locale for the small amount of call UI we own."""
    language = (
        os.environ.get("LANGUAGE")
        or os.environ.get("LC_ALL")
        or os.environ.get("LC_MESSAGES")
        or os.environ.get("LANG")
        or ""
    ).lower()
    return spanish if language.startswith("es") else english

# NotificationClosed reason codes (org.freedesktop.Notifications spec):
#   1 = expired (timeout)
#   2 = dismissed by user
#   3 = CloseNotification() called programmatically (e.g. by us on iPhone-read)
#   4 = undefined / reserved
#
# We mark-read only on dismissed-by-user. Reason 3 = we're already closing
# because the iPhone marked it read (so we'd be in a write-self-write loop).
# Reason 1 = expired, but with timeout=0 this shouldn't happen for us.
_REASON_DISMISSED = 2


class LibnotifySink:
    name = "libnotify"

    def __init__(self, hfp=None) -> None:
        # Optional HfpManager — when present, incoming-call popups carry
        # Answer / Decline action buttons wired straight to it.
        self._hfp = hfp
        self._notif = dbus.Interface(
            session_bus.get_object(
                "org.freedesktop.Notifications",
                "/org/freedesktop/Notifications",
            ),
            "org.freedesktop.Notifications",
        )
        # notification_id (uint32 from Notify) -> Message1 DBus path
        self._pending: dict[int, str] = {}
        # notification_id -> SignalMatch for the per-Message1 PropertiesChanged sub
        self._msg_subs: dict[int, object] = {}
        # Incoming-call popups: call_path <-> notification_id
        self._call_notifs: dict[str, int] = {}
        self._notif_calls: dict[int, str] = {}
        # Keep the latest state while the notification daemon is temporarily
        # unavailable (for example while Quickshell is restarting).  The call
        # remains controlled by oFono; only its visual notification is retried.
        self._latest_call_events: dict[str, object] = {}
        self._call_retry_sources: dict[str, int] = {}

        # Listen for any of our notifications closing (dismissed, expired,
        # or programmatically closed).
        self._match = self._notif.connect_to_signal(
            "NotificationClosed", self._on_closed,
        )
        # Listen for action-button clicks (Answer / Decline on call popups).
        self._action_match = self._notif.connect_to_signal(
            "ActionInvoked", self._on_action,
        )
        log.info(
            "libnotify sink ready (persistent + bidirectional read-sync)")

    def handle(self, event: SmsEvent) -> None:
        # Don't pop a desktop notification for a message we ourselves sent.
        if event.kind == "sms_sent":
            return
        title = event.display_sender
        body = (event.body or "").strip()
        if len(body) > _BODY_LIMIT:
            body = body[:_BODY_LIMIT - 1] + "…"
        try:
            # expire_timeout=0 → notification stays visible indefinitely.
            # We close it ourselves when the iPhone marks the message read,
            # or rely on the user to dismiss it manually (which we then
            # propagate back as mark-read).
            nid = int(self._notif.Notify(
                _APP_NAME,
                dbus.UInt32(0),
                _icon("chat-message-new-symbolic"),
                title,
                body,
                dbus.Array([], signature="s"),
                dbus.Dictionary({"urgency": dbus.Byte(1)}, signature="sv"),
                dbus.Int32(0),  # 0 = never expire
            ))
        except dbus.exceptions.DBusException as e:
            log.error("libnotify Notify failed: %s", e.get_dbus_name())
            return

        if event.message_path:
            self._pending[nid] = event.message_path
            # Subscribe to PropertiesChanged on this specific Message1 path
            # so we get notified if iOS marks it read.
            self._msg_subs[nid] = session_bus.add_signal_receiver(
                lambda iface, changed, _inv, nid=nid:
                    self._on_msg_props(nid, iface, changed),
                dbus_interface="org.freedesktop.DBus.Properties",
                signal_name="PropertiesChanged",
                path=event.message_path,
            )

    # ---- ANCS events (per-app notifications) ----------------------------

    def handle_ancs(self, event: AncsEvent) -> None:
        # Keep the app name clean; the real app icon carries its identity.
        app = event.app_name or event.app_id or "Notification"
        title = app
        # Body: prefer Title field for headline, then Message
        body_parts = [p for p in (event.title, event.body) if p]
        body = " — ".join(body_parts) if body_parts else ""
        if len(body) > _BODY_LIMIT:
            body = body[:_BODY_LIMIT - 1] + "…"
        try:
            # ANCS notifications also persistent (timeout=0). User dismisses
            # or we close on demand. No mark-read sync for ANCS (the iPhone
            # doesn't expose a write-back path for app notification state).
            self._notif.Notify(
                _APP_NAME,
                dbus.UInt32(0),
                _event_icon(event),
                title,
                body,
                dbus.Array([], signature="s"),
                dbus.Dictionary({"urgency": dbus.Byte(1),
                                 "suppress-sound": dbus.Boolean(event.is_silent or event.is_preexisting),
                                 "x-nodalix-silent": dbus.Boolean(event.is_silent or event.is_preexisting)}, signature="sv"),
                dbus.Int32(0),
            )
        except dbus.exceptions.DBusException as e:
            log.error("libnotify Notify (ANCS) failed: %s", e.get_dbus_name())

    # ---- HFP call events -------------------------------------------------

    def handle_call(self, event) -> None:
        if event.kind == "call_ended":
            self._latest_call_events.pop(event.call_path, None)
            retry_source = self._call_retry_sources.pop(event.call_path, None)
            if retry_source is not None:
                GLib.source_remove(retry_source)
            self._close_call_notif(event.call_path)
            return
        # Keep one live notification for the complete call lifecycle.  Notify's
        # replaces_id updates it in-place as ringing → active instead of making
        # several rows in the notification centre.
        self._latest_call_events[event.call_path] = event
        self._show_call(event)

    @staticmethod
    def _notification_service_unavailable(error) -> bool:
        return error.get_dbus_name() in {
            "org.freedesktop.DBus.Error.ServiceUnknown",
            "org.freedesktop.DBus.Error.NameHasNoOwner",
            "org.freedesktop.DBus.Error.NoReply",
        }

    def _schedule_call_retry(self, call_path: str) -> None:
        """Retry the current call once Quickshell owns Notifications again."""
        if call_path in self._call_retry_sources:
            return
        self._call_retry_sources[call_path] = GLib.timeout_add(
            750, self._retry_call_notification, call_path,
        )

    def _retry_call_notification(self, call_path: str) -> bool:
        self._call_retry_sources.pop(call_path, None)
        event = self._latest_call_events.get(call_path)
        if event is not None:
            # Recreate the proxy so a replacement notification server is used.
            self._notif = dbus.Interface(
                session_bus.get_object(
                    "org.freedesktop.Notifications",
                    "/org/freedesktop/Notifications",
                ),
                "org.freedesktop.Notifications",
            )
            self._show_call(event, retrying=True)
        return GLib.SOURCE_REMOVE

    def _show_call(self, event, *, retrying: bool = False) -> None:
        previous_id = self._call_notifs.get(event.call_path, 0)
        title = event.display_peer

        if event.kind == "call_incoming":
            body = _call_text("Incoming call", "Llamada entrante")
            icon = _icon("call-incoming-symbolic")
            if self._hfp is not None:
                actions = dbus.Array([
                    "answer", _call_text("Answer", "Responder"),
                    "decline", _call_text("Decline", "Rechazar"),
                ], signature="s")
            else:
                actions = dbus.Array([], signature="s")
        elif event.kind == "call_outgoing":
            body = _call_text("Calling…", "Llamando…")
            icon = _icon("call-outgoing-symbolic")
            actions = dbus.Array(
                ["hangup", _call_text("Hang up", "Colgar")], signature="s")
        else:
            body = _call_text("Call in progress", "Llamada en curso")
            icon = _icon("call-start-symbolic")
            actions = dbus.Array(
                ["hangup", _call_text("Hang up", "Colgar")], signature="s")

        try:
            nid = int(self._notif.Notify(
                _CALL_APP_NAME,
                dbus.UInt32(previous_id),
                icon,
                title,
                body,
                actions,
                dbus.Dictionary({
                    "urgency": dbus.Byte(2),
                    "resident": dbus.Boolean(True),
                    "category": dbus.String("x-nodalix.phone-call"),
                }, signature="sv"),
                dbus.Int32(0),  # 0 = never expire (we close it ourselves)
            ))
        except dbus.exceptions.DBusException as e:
            if self._notification_service_unavailable(e):
                self._schedule_call_retry(event.call_path)
                if not retrying:
                    log.warning(
                        "notification service unavailable; call popup queued")
            else:
                log.error(
                    "libnotify Notify (call) failed: %s", e.get_dbus_name())
            return
        # A compliant server returns the replacement id, but tolerate one that
        # allocates a new id and remove the stale reverse mapping.
        if previous_id and previous_id != nid:
            self._notif_calls.pop(previous_id, None)
        self._call_notifs[event.call_path] = nid
        self._notif_calls[nid] = event.call_path

    def _close_call_notif(self, call_path: str) -> None:
        nid = self._call_notifs.pop(call_path, None)
        if nid is None:
            return
        self._notif_calls.pop(nid, None)
        try:
            self._notif.CloseNotification(dbus.UInt32(nid))
        except dbus.exceptions.DBusException:
            pass

    def _on_action(self, nid, action_key) -> None:
        try:
            nid_i = int(nid)
        except (TypeError, ValueError):
            return
        call_path = self._notif_calls.get(nid_i)
        if call_path is None or self._hfp is None:
            return
        action = str(action_key)
        try:
            if action == "answer":
                self._hfp.answer(call_path)
                log.info("answered call from notification: %s", call_path)
            elif action in ("decline", "hangup"):
                self._hfp.hangup(call_path)
                log.info("ended call from notification: %s", call_path)
        except Exception as e:
            log.warning("call action %r failed: %s", action, e)

    # ---- iPhone marks read → close our popup ----------------------------

    def _on_msg_props(self, nid: int, iface: str, changed) -> None:
        if iface != "org.bluez.obex.Message1":
            return
        # Look for Read going True. Some BlueZ versions send Status instead.
        read_now = (
            bool(changed.get("Read", False))
            or str(changed.get("Status", "")).lower() in ("read", "complete")
        )
        if not read_now:
            return
        if nid not in self._pending:
            return  # already closed/handled
        try:
            self._notif.CloseNotification(dbus.UInt32(nid))
            log.info("iPhone marked message read — closed popup %d", nid)
        except dbus.exceptions.DBusException as e:
            log.debug("CloseNotification(%d): %s", nid, e.get_dbus_name())
        # _on_closed will clean up the dict + signal match (reason=3)

    # ---- Linux user dismisses → mark-read on iPhone ----------------------

    def _on_closed(self, nid, reason) -> None:
        try:
            nid_i = int(nid)
            reason_i = int(reason)
        except (TypeError, ValueError):
            return

        message_path = self._pending.pop(nid_i, None)

        # Clean up call-popup bookkeeping if this was an incoming-call popup.
        call_path = self._notif_calls.pop(nid_i, None)
        if call_path is not None:
            self._call_notifs.pop(call_path, None)

        # Always remove the per-message subscription, no matter the reason
        sub = self._msg_subs.pop(nid_i, None)
        if sub is not None:
            try:
                sub.remove()
            except Exception:
                pass

        if message_path is None:
            return

        # Only propagate read-state to iPhone when the human actively
        # dismissed (reason=2). Don't loop on programmatic close (reason=3,
        # which is fired when we closed it ourselves because iPhone already
        # marked it read).
        if reason_i != _REASON_DISMISSED:
            return
        try:
            dbus.Interface(
                session_bus.get_object("org.bluez.obex", message_path),
                "org.freedesktop.DBus.Properties",
            ).Set("org.bluez.obex.Message1", "Read", dbus.Boolean(True))
            log.info("marked %s as read on iPhone (user dismissed popup)",
                     message_path.rsplit("/", 1)[-1])
        except dbus.exceptions.DBusException as e:
            log.debug("mark-read failed for %s: %s",
                      message_path, e.get_dbus_name())
