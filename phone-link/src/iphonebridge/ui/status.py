"""Setup & status page — daemon health, data counts, the iPhone checklist."""
from __future__ import annotations

import logging

from gi.repository import Adw, Gtk

from iphonebridge.contacts import ContactsResolver

log = logging.getLogger(__name__)


class StatusPage(Gtk.Box):
    def __init__(self, client, toast) -> None:
        super().__init__(orientation=Gtk.Orientation.VERTICAL)
        self._client = client

        page = Adw.PreferencesPage()
        self.append(page)

        daemon_group = Adw.PreferencesGroup(title="Conexión")
        recheck = Gtk.Button(label="Volver a comprobar", valign=Gtk.Align.CENTER)
        recheck.connect("clicked", lambda _b: self._refresh())
        daemon_group.set_header_suffix(recheck)
        self._daemon_row = Adw.ActionRow(title="Servicio de Enlace móvil")
        self._daemon_icon = Gtk.Image()
        self._daemon_row.add_suffix(self._daemon_icon)
        self._map_row = Adw.ActionRow(title="Conexión de mensajes")
        self._map_icon = Gtk.Image()
        self._map_row.add_suffix(self._map_icon)
        daemon_group.add(self._daemon_row)
        daemon_group.add(self._map_row)
        page.add(daemon_group)

        data_group = Adw.PreferencesGroup(title="Datos")
        self._contacts_row = Adw.ActionRow(title="Contactos guardados")
        self._events_row = Adw.ActionRow(title="Eventos registrados")
        data_group.add(self._contacts_row)
        data_group.add(self._events_row)
        page.add(data_group)

        checklist = Adw.PreferencesGroup(
            title="Configuración del iPhone",
            description="En el iPhone, abre Ajustes → Bluetooth y pulsa ⓘ junto "
                        "a este ordenador. Después, activa estas opciones:")
        for item, sub in (
            ("Mostrar notificaciones de mensajes", "Mensajes SMS e iMessage"),
            ("Sincronizar contactos", "Mostrar el nombre de tus contactos"),
            ("Mostrar notificaciones del sistema", "Notificaciones de las aplicaciones del iPhone"),
        ):
            checklist.add(Adw.ActionRow(title=item, subtitle=sub))
        page.add(checklist)

        client.connect("availability-changed", lambda *_: self._refresh())
        self._refresh()

    def _refresh(self) -> None:
        self._client.refresh_availability()
        reachable = self._client.available
        self._daemon_row.set_subtitle("En funcionamiento" if reachable
                                      else "No disponible. Comprueba la conexión "
                                           "Bluetooth del iPhone.")
        self._daemon_icon.set_from_icon_name(
            "emblem-ok-symbolic" if reachable else "dialog-warning-symbolic")

        healthy = self._client.healthy
        self._map_row.set_subtitle(
            "Conectada" if healthy
            else "No disponible. Revisa las opciones de Bluetooth del iPhone.")
        self._map_icon.set_from_icon_name(
            "emblem-ok-symbolic" if healthy else "dialog-warning-symbolic")

        try:
            n_contacts = ContactsResolver().count()
        except Exception:
            log.exception("contact count failed")
            n_contacts = 0
        self._contacts_row.set_subtitle(f"{n_contacts}")
        self._events_row.set_subtitle(f"{len(self._client.read_events())}")
