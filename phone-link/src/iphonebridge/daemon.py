"""iphonebridge daemon — orchestrates everything.

Startup order:
  1. bluez_setup.prepare — set adapter CoD, register BLE advert
  2. SessionManager.open_all — long-lived MAP + PBAP OBEX sessions
     (retries on Forbidden — see _try_open_sessions)
  3. ContactsResolver — warm SQLite cache; if empty, pull PBAP
  4. MapEventListener — subscribe to MAP MNS push events
  5. Sinks — register libnotify + jsonl
  6. DBus service (com.gabriel.iphonebridge.Messages1)
  7. GLib.MainLoop().run()

Shutdown order is the reverse.

Degraded mode: if MAP/PBAP can't open (typically because the user hasn't
enabled iPhone toggles yet), the daemon stays alive, logs a clear
remediation hint, and retries every 60s. This avoids the systemd
crash-loop we hit in an earlier version.
"""
from __future__ import annotations

import logging
import signal
import threading
import time

import dbus
from gi.repository import GLib

from iphonebridge import bluez_setup, config
from iphonebridge.ancs.client import AncsClient
from iphonebridge.ancs.events import AncsEvent
from iphonebridge.bus import bluez, main_loop
from iphonebridge.contacts import ContactsResolver, pull_phonebook
from iphonebridge.dbus_service import MessagesService, claim_bus_name
from iphonebridge.events import SmsEvent, sms_sent_event
from iphonebridge.hfp.events import CallEvent
from iphonebridge.hfp.ofono_client import HfpManager
from iphonebridge.obex.map_events import MapEventListener
from iphonebridge.obex.sessions import SessionError, SessionManager
from iphonebridge.sinks import Sink
from iphonebridge.sinks.clipboard import ClipboardSink
from iphonebridge.sinks.jsonl import JsonlSink
from iphonebridge.sinks.libnotify import LibnotifySink

log = logging.getLogger(__name__)

# How often to re-pull the iPhone's phonebook (so the cache picks up new contacts)
CONTACTS_REFRESH_SEC = 24 * 60 * 60  # 24h

# How often to retry MAP/PBAP session open when blocked by the iPhone
# (toggles off, paired-but-not-connected, etc.)
SESSION_RETRY_SEC = 60

# Keep trying while the phone is away, but back off so an out-of-range iPhone
# does not create constant Bluetooth traffic or waste power.
RECONNECT_TICK_SEC = 15
RECONNECT_MAX_SEC = 5 * 60


class Daemon:
    def __init__(self) -> None:
        self.sessions = SessionManager()
        self.contacts = ContactsResolver()
        self.sinks: list[Sink] = []
        self.listener: MapEventListener | None = None
        self.ancs: AncsClient | None = None
        self.hfp: HfpManager | None = None
        self._contacts_refresh_id: int | None = None
        self._session_retry_id: int | None = None
        self._reconnect_id: int | None = None
        self._reconnect_in_flight = False
        self._reconnect_backoff = RECONNECT_TICK_SEC
        self._reconnect_next = 0.0
        self._bus_name = None
        self._dbus_service: MessagesService | None = None
        self._post_sessions_done = False

    # ---- lifecycle -------------------------------------------------------

    def start(self) -> None:
        log.info("=== iphonebridge starting ===")
        config.ensure_dirs()

        # Class-of-device is maintained by the root-owned Nodalix system
        # service.  Never attempt sudo from this hardened user service.
        if not bluez_setup.prepare(allow_sudo=False):
            log.warning(
                "bluez_setup.prepare reported issues — continuing anyway, "
                "but MAP/PBAP may be refused. Re-pair on iPhone after the "
                "adapter is in A/V Hands-Free CoD if the toggles aren't there."
            )

        # ANCS — per-app notifications via BLE GATT. Independent of MAP/PBAP;
        # may or may not work depending on whether BlueZ has established a
        # BLE link to the iPhone (we don't yet do the LastUsedBearer=le
        # dance). Either way, the client just waits patiently for the three
        # ANCS characteristics to appear and subscribes when they do.
        device_path = (
            f"/org/bluez/{config.ADAPTER}"
            f"/dev_{config.IPHONE_MAC.replace(':', '_')}"
        )
        if config.NOTIFICATIONS_ENABLED:
            self.ancs = AncsClient(device_path, on_event=self._fanout_ancs)
            self.ancs.start()
        else:
            log.info("ANCS application notifications disabled in Nodalix Settings")

        # HFP — take/place calls via oFono. Also independent of MAP/PBAP; if
        # oFono isn't set up it logs a hint and stays dormant.
        if config.CALLS_ENABLED:
            self.hfp = HfpManager(
                on_event=self._fanout_call,
                resolve_contact=lambda raw: self.contacts.resolve(raw),
            )
            self.hfp.start()
        else:
            log.info("HFP calls disabled in Nodalix Settings")

        # BlueZ retries briefly after a link loss and then gives up. Keep a
        # lightweight, exponentially backed-off retry running so a trusted
        # iPhone reconnects when it comes back into range without user input.
        if config.AUTO_RECONNECT:
            self._reconnect_id = GLib.timeout_add_seconds(
                RECONNECT_TICK_SEC, self._maintain_phone_connection
            )
        else:
            log.info("automatic iPhone reconnect disabled in Nodalix Settings")

        # Sinks don't need the OBEX sessions — set them up now so ANCS and
        # HFP events still reach the desktop while MAP/PBAP are degraded.
        self._setup_sinks()

        # Try to open MAP/PBAP. If blocked, stay alive and retry every minute.
        self._try_open_sessions(first_attempt=True)

        # Always set up the DBus service so a CLI can at least query
        # IsHealthy and learn the daemon's status. Send() will fail until
        # the MAP session is open, but the service surface itself is up.
        try:
            self._bus_name = claim_bus_name()
            self._dbus_service = MessagesService(
                self._bus_name, self.sessions, hfp=self.hfp,
                on_sent=self._record_sent)
            log.info("DBus service ready: com.gabriel.iphonebridge")
        except Exception:
            log.exception("DBus service registration failed — continuing "
                          "without send capability")

        # Signal handlers
        for sig in (signal.SIGINT, signal.SIGTERM):
            signal.signal(sig, self._signal)

        if not self._post_sessions_done:
            log.warning("=== iphonebridge running in DEGRADED mode ===")
            log.warning("    No MAP/PBAP session yet. Retrying every %ds.",
                        SESSION_RETRY_SEC)
        # The "ready" line in the happy path is emitted by
        # _post_sessions_setup, so we don't duplicate it here.

    def _try_open_sessions(self, *, first_attempt: bool) -> None:
        """Open MAP + PBAP. On Forbidden, schedule a periodic retry instead
        of crashing. Idempotent."""
        try:
            self.sessions.open_all()
        except SessionError as e:
            msg = str(e)
            log.warning("could not open MAP/PBAP sessions: %s", msg)
            if "Forbidden" in msg or "0x43" in msg:
                log.warning("")
                log.warning("  → This usually means the iPhone toggles aren't on.")
                log.warning("  → On the iPhone:")
                log.warning("       Settings → Bluetooth → tap (i) next to this device")
                log.warning("       Enable: Show Message Notifications")
                log.warning("       Enable: Sync Contacts")
                log.warning("")
            if first_attempt and self._session_retry_id is None:
                self._session_retry_id = GLib.timeout_add_seconds(
                    SESSION_RETRY_SEC, self._retry_sessions
                )
                log.warning("  → Daemon stays running. Will retry every %ds.",
                            SESSION_RETRY_SEC)
            return
        # Sessions opened — wire everything that depends on them.
        self._post_sessions_setup()

    def _retry_sessions(self) -> bool:
        """GLib timer callback. Return True to keep the timer firing."""
        log.info("retrying MAP/PBAP session open ...")
        try:
            self.sessions.open_all()
        except SessionError as e:
            # Still blocked — keep timer alive
            log.info("still blocked: %s", str(e)[:120])
            return True

        log.info("sessions opened on retry — promoting to ready state")
        self._post_sessions_setup()
        # Stop the retry timer
        self._session_retry_id = None
        return False

    def _setup_sinks(self) -> None:
        """Register the JSONL + libnotify sinks. Independent of the OBEX
        sessions, so ANCS/HFP events reach the desktop even in degraded mode."""
        if self.sinks:
            return
        self.sinks.append(JsonlSink())
        try:
            self.sinks.append(LibnotifySink(hfp=self.hfp))
        except Exception:
            log.exception("libnotify sink failed to init — continuing")
        if config.COPY_VERIFICATION_CODES:
            try:
                self.sinks.append(ClipboardSink())
            except Exception:
                log.exception("clipboard sink failed to init — continuing")
        else:
            log.info("verification-code clipboard copy disabled")
        log.info("sinks ready: %s", [s.name for s in self.sinks])

    def _post_sessions_setup(self) -> None:
        """Everything that requires live MAP+PBAP sessions. Idempotent so
        we can call it either at first-attempt success or at retry success."""
        if self._post_sessions_done:
            return
        self._post_sessions_done = True

        # Warm contacts; if empty, do a one-time pull. PBAP pull is cheap.
        if self.contacts.count() == 0:
            log.info("contacts cache empty — pulling from iPhone via PBAP")
            self._refresh_contacts()

        # Schedule periodic contacts refresh
        if self._contacts_refresh_id is None:
            self._contacts_refresh_id = GLib.timeout_add_seconds(
                CONTACTS_REFRESH_SEC, self._periodic_refresh_contacts
            )

        # Wire up MAP MNS listener.
        # IMPORTANT: pass an indirect lambda so the resolver can be refreshed
        # in place via self.contacts.refresh() without breaking this binding.
        if self.listener is None:
            self.listener = MapEventListener(
                sessions=self.sessions,
                on_sms=self._fanout,
                resolve_contact=lambda raw: self.contacts.resolve(raw),
            )
            self.listener.start()

        log.info("=== iphonebridge ready (contacts=%d, sinks=%s) ===",
                 self.contacts.count(),
                 [s.name for s in self.sinks])

    def _refresh_contacts(self) -> None:
        """Pull phonebook from iPhone + reload in-process cache. Idempotent."""
        try:
            pulled = pull_phonebook(self.sessions)
            count = self.contacts.refresh()
            log.info("contacts refresh: pulled %d, cached %d", pulled, count)
        except Exception:
            log.exception("contacts refresh failed — running with previous cache")

    def _periodic_refresh_contacts(self) -> bool:
        """GLib timeout callback. Return True to keep the timer running."""
        log.info("periodic contacts refresh tick")
        self._refresh_contacts()
        return True

    def _maintain_phone_connection(self) -> bool:
        """Reconnect the configured iPhone without blocking the GLib loop."""
        now = time.monotonic()
        if self._reconnect_in_flight or now < self._reconnect_next:
            return True

        device_path = (
            f"/org/bluez/{config.ADAPTER}"
            f"/dev_{config.IPHONE_MAC.replace(':', '_')}"
        )
        try:
            props = bluez(device_path, "org.freedesktop.DBus.Properties")
            if bool(props.Get("org.bluez.Device1", "Connected")):
                self._reconnect_backoff = RECONNECT_TICK_SEC
                self._reconnect_next = now + RECONNECT_TICK_SEC
                return True
        except dbus.exceptions.DBusException as error:
            log.debug("iPhone connection state unavailable: %s", error)

        self._reconnect_in_flight = True

        def connect() -> None:
            error_text = ""
            try:
                log.info("iPhone is nearby but disconnected; reconnecting")
                bluez(device_path, "org.bluez.Device1").Connect(timeout=12)
            except dbus.exceptions.DBusException as error:
                error_text = error.get_dbus_message() or str(error)
            except Exception as error:
                error_text = str(error)
            GLib.idle_add(self._finish_reconnect, error_text)

        threading.Thread(
            target=connect,
            name="nodalix-phone-reconnect",
            daemon=True,
        ).start()
        return True

    def _finish_reconnect(self, error_text: str) -> bool:
        self._reconnect_in_flight = False
        now = time.monotonic()
        if error_text:
            log.debug("automatic iPhone reconnect deferred: %s", error_text)
            self._reconnect_next = now + self._reconnect_backoff
            self._reconnect_backoff = min(
                self._reconnect_backoff * 2, RECONNECT_MAX_SEC
            )
        else:
            log.info("automatic iPhone reconnect succeeded")
            self._reconnect_backoff = RECONNECT_TICK_SEC
            self._reconnect_next = now + RECONNECT_TICK_SEC
        return False

    def stop(self) -> None:
        log.info("=== iphonebridge stopping ===")
        for tid_attr in ("_contacts_refresh_id", "_session_retry_id", "_reconnect_id"):
            tid = getattr(self, tid_attr, None)
            if tid is not None:
                try:
                    GLib.source_remove(tid)
                except Exception:
                    pass
                setattr(self, tid_attr, None)
        if self.listener is not None:
            self.listener.stop()
        if self.ancs is not None:
            self.ancs.stop()
        if self.hfp is not None:
            self.hfp.stop()
        self.sessions.close_all()
        bluez_setup.unregister_advert()
        main_loop.quit()

    def run(self) -> None:
        self.start()
        try:
            main_loop.run()
        finally:
            self.stop()

    # ---- internals -------------------------------------------------------

    def _fanout(self, event: SmsEvent) -> None:
        for sink in self.sinks:
            try:
                sink.handle(event)
            except Exception:
                log.exception("sink %s failed on event %s",
                              sink.name, event.handle)
        if self._dbus_service is not None:
            self._dbus_service.emit_message(event)

    def _record_sent(self, recipient: str, body: str, transfer_path: str) -> None:
        """Hook for DBus Send() — log + broadcast a message we just sent so it
        shows up in conversation history alongside incoming messages."""
        event = sms_sent_event(
            recipient, body,
            contact_name=self.contacts.resolve(recipient),
            transfer_path=transfer_path,
        )
        log.info("sms_sent to %s: %r", event.display_sender, (body or "")[:80])
        self._fanout(event)

    def _fanout_ancs(self, event: AncsEvent) -> None:
        for sink in self.sinks:
            try:
                handler = getattr(sink, "handle_ancs", None)
                if handler is None:
                    continue  # sink doesn't know about ANCS events
                handler(event)
            except Exception:
                log.exception("sink %s failed on ANCS event %d",
                              sink.name, event.notification_id)
        if self._dbus_service is not None:
            self._dbus_service.emit_ancs(event)

    def _fanout_call(self, event: CallEvent) -> None:
        for sink in self.sinks:
            try:
                handler = getattr(sink, "handle_call", None)
                if handler is None:
                    continue  # sink doesn't know about call events
                handler(event)
            except Exception:
                log.exception("sink %s failed on call event %s",
                              sink.name, event.call_path)
        if self._dbus_service is not None:
            self._dbus_service.emit_call_state(event)

    def _signal(self, signum, _frame):
        log.info("received signal %d, stopping", signum)
        main_loop.quit()
