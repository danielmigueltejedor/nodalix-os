#!/usr/bin/python3
"""Turn a pristine Nautilus 51.1 tree into Archivador, Nodalix OS's file manager.

Run from the root of the Nautilus source tree (the PKGBUILD does this in prepare()).
Every edit is anchored on exact upstream text and fails loudly when the anchor is missing,
so a Nautilus update can never silently drop part of the layer.

What it does:
  * installs the Nodalia Design Language (NDL) sheets, pinned in gnome/control-center/ndl
    (one copy for every Nodalix app), plus Archivador's own overrides, as GResources;
  * loads them at startup below Archivador's style.css and follows light/dark/contrast;
  * sets the sidebar and the content as two NDL islands;
  * names the app Archivador;
  * builds Nodalix's own extensions into it (today: "Compartir con LocalSend").
"""
from pathlib import Path
import shutil

HERE = Path(__file__).resolve().parent
NDL = HERE.parent / 'control-center' / 'ndl'
root = Path.cwd()

NDL_SHEETS = ('nodalia-light.css', 'nodalia-dark.css', 'nodalia-light-hc.css', 'nodalia-dark-hc.css',
              'nodalia-metrics-regular.css', 'nodalia-gtk.css')
OVERRIDES = 'nodalix-overrides.css'


def replace_once(relative, old, new):
    path = root / relative
    text = path.read_text()
    if text.count(old) != 1:
        raise SystemExit(f'Archivador overlay anchor must match exactly once in {relative}: {old!r}')
    path.write_text(text.replace(old, new, 1))


if not (root / 'src/nautilus-application.c').is_file():
    raise SystemExit('Run apply.py from the root of a Nautilus source tree')

# Sheets and loader.
(root / 'src/resources/ndl').mkdir(exist_ok=True)
for name in NDL_SHEETS:
    shutil.copy2(NDL / name, root / 'src/resources/ndl' / name)
shutil.copy2(HERE / OVERRIDES, root / 'src/resources/ndl' / OVERRIDES)
for name in ('nautilus-nodalix-theme.c', 'nautilus-nodalix-theme.h'):
    shutil.copy2(HERE / name, root / 'src' / name)

replace_once('src/meson.build', "  'nautilus-application.c',",
             "  'nautilus-application.c',\n  'nautilus-nodalix-theme.c',")
replace_once('src/resources/nautilus.gresource.xml.in', '    <file>style.css</file>',
             '    <file>style.css</file>\n' +
             ''.join(f'    <file>ndl/{name}</file>\n' for name in (*NDL_SHEETS, OVERRIDES)).rstrip('\n'))
replace_once('src/nautilus-application.c', '#include "nautilus-application.h"',
             '#include "nautilus-application.h"\n#include "nautilus-nodalix-theme.h"')
replace_once('src/nautilus-application.c', '    gtk_window_set_default_icon_name (APPLICATION_ID);',
             '    gtk_window_set_default_icon_name (APPLICATION_ID);\n\n    nautilus_nodalix_theme_init ();')

# Own extensions, built with Nautilus (gnome/archivador/extensions/<name>).
EXTENSIONS = ('nodalix-localsend',)
for name in EXTENSIONS:
    shutil.copytree(HERE / 'extensions' / name, root / 'extensions' / name)
replace_once('extensions/meson.build', "subdir('image-properties')",
             "json_glib_localsend = dependency('json-glib-1.0')\n" +
             ''.join(f"subdir('{name}')\n" for name in EXTENSIONS) + "subdir('image-properties')")

# Two islands: sidebar and content.
window = 'src/resources/ui/nautilus-window.blp'
replace_once(window, '        child: Adw.OverlaySplitView split_view {\n          enable-show-gesture: false;',
             '        child: Adw.OverlaySplitView split_view {\n          styles [\n            "ndl-split",\n          ]\n\n'
             '          enable-show-gesture: false;')
replace_once(window, '          sidebar: Adw.ToolbarView {\n            reveal-bottom-bars',
             '          sidebar: Adw.ToolbarView {\n            styles [\n              "ndl-island",\n            ]\n\n'
             '            overflow: hidden;\n            reveal-bottom-bars')
replace_once(window, '          content: Adw.ToolbarView {\n            [top]\n            $NautilusToolbar toolbar {',
             '          content: Adw.ToolbarView {\n            styles [\n              "ndl-island",\n            ]\n\n'
             '            overflow: hidden;\n\n            [top]\n            $NautilusToolbar toolbar {')

# Name. Binary, application id and D-Bus names stay Nautilus's so portals, file-manager
# integration, nautilus-python extensions and every `nautilus` caller keep working.
replace_once('src/resources/ui/nautilus-window.blp', '                title: _("Files");', '                title: "Archivador";')
replace_once('data/org.gnome.Nautilus.desktop.in.in', 'Name=Files\n', 'Name=Archivador\n')
replace_once('data/org.gnome.Nautilus.metainfo.xml.in.in', '  <name>Files</name>', '  <name>Archivador</name>')
