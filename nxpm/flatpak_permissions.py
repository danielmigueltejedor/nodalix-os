#!/usr/bin/env python3
"""Nodalix Flatpak permissions: explicit per-user overrides, never auto-grant."""
import configparser
import subprocess

import gi
gi.require_version("Adw", "1")
gi.require_version("Gtk", "4")
from gi.repository import Adw, Gtk

FLATPAK = "/usr/bin/flatpak"
NXPM = "/usr/bin/nxpm"
SCOPES = (
    ("home", "Carpeta personal", "Acceso a archivos de tu carpeta personal"),
    ("xdg-documents", "Documentos", "Carpeta Documentos"),
    ("xdg-download", "Descargas", "Carpeta Descargas"),
    ("xdg-pictures", "Imágenes", "Carpeta Imágenes"),
    ("xdg-music", "Música", "Carpeta Música"),
    ("xdg-videos", "Vídeos", "Carpeta Vídeos"),
    ("xdg-desktop", "Escritorio", "Archivos del escritorio"),
)


def run(command):
    return subprocess.run(command, text=True, capture_output=True, check=False)


def installed_apps():
    result = run([FLATPAK, "list", "--app", "--columns=application,name"])
    if result.returncode:
        return [], result.stderr.strip() or "Flatpak no está disponible."
    apps = []
    for line in result.stdout.splitlines():
        fields = line.split("\t", 1)
        if fields and fields[0].strip():
            app_id = fields[0].strip()
            name = fields[1].strip() if len(fields) > 1 else app_id
            apps.append((app_id, name))
    return sorted(set(apps), key=lambda row: row[1].casefold()), ""


def override_filesystems(app_id):
    result = run([FLATPAK, "override", "--user", "--show", app_id])
    if result.returncode or not result.stdout.strip():
        return set()
    config = configparser.ConfigParser(interpolation=None, strict=False)
    try:
        config.read_string(result.stdout)
        return {item for item in config.get("Context", "filesystems", fallback="").split(";") if item}
    except configparser.Error:
        return set()


class Window(Adw.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app, title="Permisos Flatpak — NXPM",
                         default_width=710, default_height=700)
        self.apps, problem = installed_apps()
        toolbar = Adw.ToolbarView()
        toolbar.add_top_bar(Adw.HeaderBar())
        page = Adw.PreferencesPage()
        toolbar.set_content(page)
        self.set_content(toolbar)

        selection = Adw.PreferencesGroup(title="Aplicación",
            description="Selecciona una aplicación instalada para gestionar sus permisos.")
        page.add(selection)
        if not self.apps:
            row = Adw.ActionRow(title="No se encontraron aplicaciones Flatpak",
                                subtitle=problem or "Instala una aplicación Flatpak para gestionar sus permisos.")
            selection.add(row)
            return
        self.selector = Gtk.DropDown.new_from_strings(
            [name + " — " + app_id for app_id, name in self.apps])
        choose = Adw.ActionRow(title="Aplicación Flatpak",
                               subtitle="Los cambios afectan únicamente a tu usuario.")
        choose.add_suffix(self.selector)
        self.selector.set_valign(Gtk.Align.CENTER)
        selection.add(choose)

        self.permissions = Adw.PreferencesGroup(title="Acceso a carpetas",
            description="Solo se cambian permisos que elijas explícitamente. "
                        "«Por defecto» respeta los permisos de la aplicación.")
        page.add(self.permissions)
        self.status_labels = {}
        for key, title, description in SCOPES:
            row = Adw.ActionRow(title=title, subtitle=description)
            label = Gtk.Label(label="Por defecto")
            label.add_css_class("dim-label")
            row.add_suffix(label)
            allow = Gtk.Button(label="Permitir")
            deny = Gtk.Button(label="Bloquear")
            allow.add_css_class("flat")
            deny.add_css_class("flat")
            allow.set_tooltip_text("Permitir acceso a " + title)
            deny.set_tooltip_text("Bloquear acceso a " + title)
            allow.connect("clicked", self.change, "grant", key)
            deny.connect("clicked", self.change, "revoke", key)
            row.add_suffix(allow)
            row.add_suffix(deny)
            self.permissions.add(row)
            self.status_labels[key] = label

        details = Adw.PreferencesGroup(title="Detalles",
            description="Permisos declarados y modificaciones aplicadas a esta aplicación.")
        page.add(details)
        reset = Gtk.Button(label="Restablecer todos los permisos personalizados")
        reset.add_css_class("destructive-action")
        reset.connect("clicked", self.reset)
        details.add(reset)
        self.raw = Gtk.Label(xalign=0, selectable=True, wrap=True)
        self.raw.set_margin_top(12)
        self.raw.set_margin_bottom(12)
        details.add(self.raw)
        self.message = Adw.PreferencesGroup()
        page.add(self.message)
        self.feedback = Adw.ActionRow(title="Cambios aplicados al próximo inicio de la aplicación")
        self.message.add(self.feedback)
        self.selector.connect("notify::selected", self.refresh)
        self.refresh()

    def selected(self):
        return self.apps[self.selector.get_selected()][0]

    def refresh(self, *_):
        app_id = self.selected()
        changes = override_filesystems(app_id)
        for scope, _title, _description in SCOPES:
            status = ("Bloqueado" if "!" + scope in changes else
                      "Permitido" if scope in changes else "Por defecto")
            self.status_labels[scope].set_label(status)
        info = run([NXPM, "permissions", "show", app_id])
        self.raw.set_label(info.stdout.strip() if info.returncode == 0
                           else (info.stderr.strip() or "No se pudieron consultar los permisos."))

    def change(self, _button, action, scope):
        result = run([NXPM, "permissions", action, self.selected(), scope])
        self.feedback.set_title(
            "Permiso actualizado; reinicia la aplicación para aplicar el cambio."
            if result.returncode == 0 else
            "No se pudo modificar el permiso: " + (result.stderr.strip() or "error desconocido"))
        self.refresh()

    def reset(self, _button):
        result = run([NXPM, "permissions", "reset", self.selected()])
        self.feedback.set_title(
            "Se han restablecido las modificaciones de permisos."
            if result.returncode == 0 else
            "No se pudieron restablecer los permisos.")
        self.refresh()


class App(Adw.Application):
    def __init__(self):
        super().__init__(application_id="com.nodalix.NXPMPermissions")

    def do_activate(self):
        self.window = Window(self)
        self.window.present()


if __name__ == "__main__":
    raise SystemExit(App().run([]))
