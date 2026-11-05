"""Small native installer, shared by file associations and drag-to-Applications."""
import threading
from pathlib import Path
import gi
gi.require_version('Adw', '1')
from gi.repository import Adw, Gtk, Gio, GLib
from library import install_file, applications_folder, Library


def show_installer(path):
    if path is None or not path.is_file():
        raise ValueError('Selecciona un archivo de instalación local.')
    app = Adw.Application(application_id='com.nodalix.AppInstaller', flags=Gio.ApplicationFlags.NON_UNIQUE)
    def activate(application):
        window = Adw.ApplicationWindow(application=application, title='Instalar aplicación', default_width=480, default_height=360)
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=18, margin_top=24, margin_bottom=24, margin_start=24, margin_end=24)
        box.append(Gtk.Image(icon_name='system-software-install', pixel_size=64))
        title = Gtk.Label(label=path.name, wrap=True); title.add_css_class('title-2'); box.append(title)
        message = Gtk.Label(label='Se añadirá a Aplicaciones. Para desinstalarla, envíala a la papelera desde esa carpeta.', wrap=True)
        box.append(message)
        button = Gtk.Button(label='Instalar'); button.add_css_class('suggested-action'); box.append(button)
        spinner = Gtk.Spinner(); box.append(spinner)
        def clicked(_button):
            button.set_sensitive(False); spinner.start(); message.set_label('Instalando…')
            def worker():
                try:
                    install_file(path, applications_folder())
                    Library().sync(handle_trash=False, import_files=False)
                    GLib.idle_add(finished, '')
                except Exception as error:
                    GLib.idle_add(finished, str(error))
            threading.Thread(target=worker, daemon=True).start()
        def finished(error):
            spinner.stop()
            message.set_label('No se pudo instalar: '+error if error else 'La aplicación ya está en Aplicaciones.')
            button.set_label('Reintentar' if error else 'Abrir Aplicaciones')
            button.set_sensitive(True)
            if not error:
                button.disconnect(handler)
                button.connect('clicked', lambda _b: Gio.AppInfo.launch_default_for_uri(applications_folder().as_uri(), None))
            return GLib.SOURCE_REMOVE
        handler = button.connect('clicked', clicked)
        window.set_content(box); window.present()
    app.connect('activate', activate)
    app.run([])
