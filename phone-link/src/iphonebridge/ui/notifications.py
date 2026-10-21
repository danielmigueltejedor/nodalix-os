"""Notifications page — a live feed of per-app ANCS notifications."""
from __future__ import annotations

from gi.repository import Adw, Gtk

from iphonebridge.ui.util import event_ts, format_ts


class NotificationsPage(Gtk.Box):
    def __init__(self, client, toast) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL)
        self._client = client

        self._list = Gtk.ListBox(
            selection_mode=Gtk.SelectionMode.NONE,
            css_classes=["boxed-list"], valign=Gtk.Align.START,
            margin_top=12, margin_bottom=12, margin_start=12, margin_end=12)
        scroll = Gtk.ScrolledWindow(child=self._list, vexpand=True)
        self._empty = Adw.StatusPage(
            icon_name="preferences-system-notifications-symbolic",
            title="Todavía no hay notificaciones",
            description="Aquí aparecerán las notificaciones de las aplicaciones "
                        "de tu iPhone, como Slack, Mail o WhatsApp, a medida que lleguen.")
        self._stack = Gtk.Stack(vexpand=True)
        self._stack.add_named(scroll, "list")
        self._stack.add_named(self._empty, "empty")
        self.append(self._stack)

        self._count = 0
        for ev in self._client.read_events(kinds={"ancs_notification"}):
            self._prepend(ev)
        self._update_stack()
        client.connect("ancs-notification", self._on_notification)

    def _on_notification(self, _client, ev: dict) -> None:
        if ev.get("is_preexisting"):
            return
        self._prepend(ev)
        self._update_stack()

    def _prepend(self, ev: dict) -> None:
        app = ev.get("app_name") or ev.get("app_id") or "Notificación"
        title = (ev.get("title") or "").strip()
        body = (ev.get("body") or "").strip()
        subtitle = " — ".join(p for p in (title, body) if p) or "Sin vista previa"

        row = Adw.ActionRow(use_markup=False)
        row.set_title(app)
        row.set_subtitle(subtitle)
        row.set_subtitle_lines(2)
        ts = format_ts(event_ts(ev), fmt="%H:%M")
        if ts:
            row.add_suffix(Gtk.Label(label=ts, css_classes=["dim-label",
                                                            "caption"]))
        self._list.prepend(row)
        self._count += 1

    def _update_stack(self) -> None:
        self._stack.set_visible_child_name("list" if self._count else "empty")
