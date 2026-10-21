"""Calls page — dial out, and answer / hang up active HFP calls."""
from __future__ import annotations

import re

from gi.repository import Adw, Gtk

from iphonebridge.contacts import ContactsResolver

_PHONE_RE = re.compile(r"^\+?[\d\s()\-.]{7,}$")


class CallsPage(Gtk.Box):
    def __init__(self, client, toast) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL)
        self._client = client
        self._toast = toast
        self._contacts = ContactsResolver()
        self._call_rows: list = []

        page = Adw.PreferencesPage()
        self.append(page)

        dial_group = Adw.PreferencesGroup(
            title="Realizar una llamada",
            description="El audio de las llamadas utiliza el micrófono y "
                        "los altavoces de este ordenador.")
        self._entry = Adw.EntryRow(title="Número o nombre del contacto")
        self._entry.connect("entry-activated", self._on_dial)
        call_btn = Gtk.Button(
            icon_name="call-start-symbolic", valign=Gtk.Align.CENTER,
            css_classes=["suggested-action"], tooltip_text="Llamar")
        call_btn.connect("clicked", self._on_dial)
        self._entry.add_suffix(call_btn)
        dial_group.add(self._entry)
        page.add(dial_group)

        self._calls_group = Adw.PreferencesGroup(title="Llamadas activas")
        page.add(self._calls_group)
        self._refresh_calls()

        client.connect("call-state-changed", self._on_call_state)

    # ---- dialing -------------------------------------------------------

    def _on_dial(self, _widget) -> None:
        raw = self._entry.get_text().strip()
        if not raw:
            return
        number = self._resolve(raw)
        if number is None:
            self._toast(f"No se ha encontrado ningún contacto que coincida con «{raw}»")
            return
        self._entry.set_sensitive(False)

        def ok(_path: str) -> None:
            self._entry.set_sensitive(True)
            self._entry.set_text("")
            self._toast(f"Llamando a {raw}…")
            self._refresh_calls()

        def err(text: str) -> None:
            self._entry.set_sensitive(True)
            self._toast(f"No se ha podido realizar la llamada: {text}")

        self._client.dial(number, ok, err)

    def _resolve(self, raw: str) -> str | None:
        """A phone number passes through; a name is looked up in contacts."""
        if _PHONE_RE.match(raw):
            return raw
        matches = self._contacts.find_by_name(raw)
        if not matches:
            return None
        return "+" + matches[0][1]

    # ---- active calls --------------------------------------------------

    def _on_call_state(self, _client, _ev) -> None:
        self._refresh_calls()

    def _refresh_calls(self) -> None:
        for row in self._call_rows:
            self._calls_group.remove(row)
        self._call_rows = []
        calls = self._client.list_calls()
        if not calls:
            row = Adw.ActionRow(title="No hay llamadas activas",
                                css_classes=["dim-label"])
            self._calls_group.add(row)
            self._call_rows.append(row)
            return
        for c in calls:
            row = self._make_call_row(c)
            self._calls_group.add(row)
            self._call_rows.append(row)

    def _make_call_row(self, c: dict) -> Adw.ActionRow:
        peer = c.get("contact_name") or c.get("peer_phone") or "Contacto desconocido"
        state = c.get("state", "?")
        direction = c.get("direction", "")
        state_label = {
            "active": "En curso",
            "held": "En espera",
            "dialing": "Marcando",
            "alerting": "Sonando",
            "incoming": "Entrante",
            "waiting": "En espera",
            "disconnected": "Finalizada",
        }.get(state, "Estado desconocido")
        direction_label = {
            "incoming": "Entrante",
            "outgoing": "Saliente",
        }.get(direction, "")
        subtitle = " · ".join(part for part in (direction_label, state_label) if part)
        row = Adw.ActionRow(use_markup=False)
        row.set_title(peer)
        row.set_subtitle(subtitle)
        path = c.get("call_path", "")
        if state in ("incoming", "waiting"):
            answer = Gtk.Button(
                icon_name="call-start-symbolic", valign=Gtk.Align.CENTER,
                css_classes=["suggested-action"], tooltip_text="Responder")
            answer.connect("clicked", lambda _b, p=path: self._answer(p))
            row.add_suffix(answer)
        hang = Gtk.Button(
            icon_name="call-stop-symbolic", valign=Gtk.Align.CENTER,
            css_classes=["destructive-action"], tooltip_text="Colgar")
        hang.connect("clicked", lambda _b, p=path: self._hangup(p))
        row.add_suffix(hang)
        return row

    def _answer(self, path: str) -> None:
        err = self._client.answer_call(path)
        if err:
            self._toast(f"No se ha podido responder a la llamada: {err}")

    def _hangup(self, path: str) -> None:
        err = self._client.hangup_call(path)
        if err:
            self._toast(f"No se ha podido finalizar la llamada: {err}")
