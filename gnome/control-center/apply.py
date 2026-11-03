#!/usr/bin/python3
"""Apply the bounded Nodalix additions to GNOME Control Center 51.0."""
from pathlib import Path
import shutil
import sys
root=Path(sys.argv[1]);source=Path(__file__).resolve().parent
if 'cc_nodalix_updates_page' in (root/'panels/system/cc-system-panel.c').read_text():
    raise SystemExit('Already patched')
for name in ['cc-nodalix-pages.h','cc-nodalix-pages.c','cc-nodalix-localsend.c','cc-nodalix-diagnostics.c','cc-nodalix-release.c','cc-nodalix-release.h']:
    shutil.copy2(source/name,root/'panels/system'/name)
p=root/'panels/system/meson.build';t=p.read_text().replace("  'cc-system-panel.c',","  'cc-system-panel.c',\n  'cc-nodalix-pages.c',\n  'cc-nodalix-release.c',\n  'cc-nodalix-localsend.c',");t=t.replace("deps = common_deps + [", "deps = common_deps + [\n  dependency('json-glib-1.0'),");p.write_text(t)
p=root/'panels/system/cc-system-panel.c';t=p.read_text().replace('#include "cc-system-panel.h"','#include "cc-system-panel.h"\n#include "cc-nodalix-pages.h"')
t=t.replace('    g_type_ensure (CC_TYPE_ABOUT_PAGE);','    g_type_ensure (CC_TYPE_NODALIX_UPDATES_PAGE);\n    g_type_ensure (CC_TYPE_ABOUT_PAGE);')
t=t.replace('    cc_panel_add_static_subpage (CC_PANEL (self), "about", CC_TYPE_ABOUT_PAGE);','    cc_panel_add_static_subpage (CC_PANEL (self), "updates", CC_TYPE_NODALIX_UPDATES_PAGE);\n    cc_panel_add_static_subpage (CC_PANEL (self), "about", CC_TYPE_ABOUT_PAGE);')
t=t.replace('show_software_updates_group (self));','FALSE);')
p.write_text(t)

# Keep the Nodalix release identity separate from the base distribution version.
p=root/'panels/system/about/cc-about-page.blp';t=p.read_text()
t=t.replace('          $CcHostnameEntry hostname_row {','''          Adw.ActionRow nodalix_version_row {
            title: "Versión de Nodalix OS";
            subtitle-selectable: true;
            styles ["property"]
          }
        }

        Adw.PreferencesGroup {
          $CcHostnameEntry hostname_row {''')
p.write_text(t)
p=root/'panels/system/about/cc-about-page.c';t=p.read_text()
t=t.replace('#include "cc-about-page.h"','#include "cc-about-page.h"\n#include "../cc-nodalix-release.h"')
t=t.replace('    AdwActionRow *os_name_row;','    AdwActionRow *nodalix_version_row;\n    AdwActionRow *os_name_row;')
t=t.replace('    gtk_widget_class_bind_template_child (widget_class, CcAboutPage, os_name_row);','    gtk_widget_class_bind_template_child (widget_class, CcAboutPage, nodalix_version_row);\n    gtk_widget_class_bind_template_child (widget_class, CcAboutPage, os_name_row);')
t=t.replace('    gtk_widget_init_template (GTK_WIDGET (self));','    gtk_widget_init_template (GTK_WIDGET (self));\n    cc_nodalix_release_watch (self->nodalix_version_row, "/etc/nodalix-release");')
p.write_text(t)
p=root/'panels/system/meson.build';p.write_text(p.read_text().replace("  'cc-nodalix-pages.c',", "  'cc-nodalix-pages.c',\n  'cc-nodalix-diagnostics.c',"))
p=root/'panels/system/cc-system-panel.c';t=p.read_text().replace('    g_type_ensure (CC_TYPE_NODALIX_UPDATES_PAGE);','    g_type_ensure (CC_TYPE_NODALIX_UPDATES_PAGE);\n    g_type_ensure (CC_TYPE_NODALIX_DIAGNOSTICS_PAGE);')
t=t.replace('    cc_panel_add_static_subpage (CC_PANEL (self), "updates", CC_TYPE_NODALIX_UPDATES_PAGE);','    cc_panel_add_static_subpage (CC_PANEL (self), "diagnostics", CC_TYPE_NODALIX_DIAGNOSTICS_PAGE);\n    cc_panel_add_static_subpage (CC_PANEL (self), "updates", CC_TYPE_NODALIX_UPDATES_PAGE);');p.write_text(t)
p=root/'panels/system/cc-system-panel.blp';t=p.read_text().replace('        $CcListRow about_row {','''        $CcListRow {
          title: "Actualizaciones";
          subtitle: "Nodalix, aplicaciones, programas y kernel";
          icon-name: "view-refresh-symbolic";
          show-arrow: true;
          action-name: "navigation.push";
          action-target: "'updates'";
        }

        $CcListRow about_row {''');p.write_text(t)
p.write_text(p.read_text().replace('        $CcListRow about_row {','''        $CcListRow {
          title: "Diagnóstico del sistema";
          subtitle: "Versiones, pantallas, memoria y servicios";
          icon-name: "dialog-information-symbolic";
          show-arrow: true;
          action-name: "navigation.push";
          action-target: "'diagnostics'";
        }
        $CcListRow about_row {'''))
for name in ['cc-nodalix-background.c','cc-nodalix-background.h']:
    shutil.copy2(source/name,root/'panels/background'/name)
p=root/'panels/background/meson.build';t=p.read_text().replace("  'cc-background-chooser.c',","  'cc-background-chooser.c',\n  'cc-nodalix-background.c',")
t=t.replace("deps = common_deps + [", "deps = common_deps + [\n  dependency('json-glib-1.0'),")
p.write_text(t)
p=root/'panels/background/cc-background-panel.c';t=p.read_text().replace('#include "cc-background-panel.h"','#include "cc-background-panel.h"\n#include "cc-nodalix-background.h"')
t=t.replace('    g_type_ensure (CC_TYPE_BACKGROUND_CHOOSER);','    g_type_ensure (CC_TYPE_BACKGROUND_CHOOSER);\n    g_type_ensure (CC_TYPE_NODALIX_BACKGROUND_CHOOSER);')
t=t.replace('    g_signal_handlers_block_by_func (self->settings, on_settings_changed, self);','''    g_dbus_connection_call (self->connection, "com.nodalix.Settings", "/com/nodalix/Settings",
                            "com.nodalix.Settings1", "StopWallpaper", NULL, NULL,
                            G_DBUS_CALL_FLAGS_NONE, 2000, NULL, NULL, NULL);
    g_signal_handlers_block_by_func (self->settings, on_settings_changed, self);''',1)
p.write_text(t)
p=root/'panels/background/cc-background-panel.c';t=p.read_text()
t=t.replace('    gtk_widget_init_template (GTK_WIDGET (self));', '    gtk_widget_init_template (GTK_WIDGET (self));\n    cc_panel_add_static_subpage (CC_PANEL (self), "animated-wallpapers", CC_TYPE_NODALIX_WALLPAPER_PAGE);')
p.write_text(t)
p=root/'panels/background/cc-background-panel.blp';t=p.read_text();pos=t.rfind('        };');t=t[:pos]+'''          Adw.PreferencesGroup {
            Adw.ActionRow {
              title: "Fondos animados";
              subtitle: "Elige un vídeo para el escritorio";
              activatable: true;
              action-name: "navigation.push";
              action-target: "'animated-wallpapers'";
              [suffix]
              Image { icon-name: "go-next-symbolic"; }
            }
          }
'''+t[pos:];p.write_text(t)

p=root/'panels/sharing/cc-sharing-panel.c';t=p.read_text().replace('#include "cc-sharing-panel.h"','#include "cc-sharing-panel.h"\n#include "../system/cc-nodalix-pages.h"')
t=t.replace('    gtk_widget_init_template (GTK_WIDGET (self));','    gtk_widget_init_template (GTK_WIDGET (self));\n    cc_panel_add_static_subpage (CC_PANEL (self), "localsend", CC_TYPE_NODALIX_LOCALSEND_PAGE);')
# LocalSend provides a sharing page even without gnome-user-share or Rygel.
t=t.replace('visible = cc_sharing_panel_check_schema_available (FILE_SHARING_SCHEMA_ID)\n              || cc_sharing_panel_check_media_sharing_available ();', 'visible = TRUE; /* Nodalix LocalSend is always available. */')
p.write_text(t)
p=root/'panels/sharing/cc-sharing-panel.blp';t=p.read_text().replace('        $CcListRow personal_file_sharing_row {','''        $CcListRow {
          icon-name: "send-to-symbolic";
          show-arrow: true;
          title: "LocalSend";
          subtitle: "Compartir archivos desde Nautilus y con dispositivos cercanos";
          action-name: "navigation.push";
          action-target: "'localsend'";
        }
        $CcListRow personal_file_sharing_row {''');p.write_text(t)

# Allow a high resolution distributor image to shrink before the height clamp.
p=root/'panels/system/about/cc-about-page.blp';t=p.read_text()
t=t.replace('maximum-size: 192;', 'maximum-size: 96;').replace('tightening-threshold: 192;', 'tightening-threshold: 96;')
t=t.replace('can-shrink: false;', 'can-shrink: true;\n              height-request: 96;')
p.write_text(t)

# Storage management is a native Applications subpage and scans on demand.
for name in ('cc-nodalix-storage.c', 'cc-nodalix-storage.h'):
    shutil.copy2(source/name, root/'panels/applications'/name)
p = root/'panels/applications/meson.build'; t = p.read_text()
t = t.replace("  'cc-applications-panel.c',", "  'cc-applications-panel.c',\n  'cc-nodalix-storage.c',")
t = t.replace('deps = common_deps + [keyboard_shortcuts_dep]', "deps = common_deps + [keyboard_shortcuts_dep, dependency('json-glib-1.0')]")
p.write_text(t)
p = root/'panels/applications/cc-applications-panel.c'; t = p.read_text()
t = t.replace('#include "cc-applications-panel.h"', '#include "cc-applications-panel.h"\n#include "cc-nodalix-storage.h"')
t = t.replace('    gtk_widget_init_template (GTK_WIDGET (self));', '    gtk_widget_init_template (GTK_WIDGET (self));\n    cc_panel_add_static_subpage (CC_PANEL (self), "storage", CC_TYPE_NODALIX_STORAGE_PAGE);')
p.write_text(t)
p = root/'panels/applications/cc-applications-panel.blp'; t = p.read_text()
anchor = '              action-target: "\'default-apps\'";\n            }'
assert anchor in t
p.write_text(t.replace(anchor, anchor + '''
            $CcListRow {
              icon-name: "drive-harddisk-symbolic";
              show-arrow: true;
              title: "Almacenamiento";
              subtitle: "Revisa aplicaciones y archivos por tamaño";
              action-name: "navigation.push";
              action-target: "'storage'";
            }''', 1))

# Keep the built-in Settings search provider. These entries are searchable in
# both Settings and the GNOME launcher without adding apps to the app grid.
search_sections = (
    ('storage', 'Storage', 'Almacenamiento', 'applications', 'storage', 'drive-harddisk-symbolic',
     'disk;storage;space;large;files;documents;music;developer;', 'disco;espacio;archivos;documentos;grandes;música;desarrollo;'),
    ('updates', 'Updates', 'Actualizaciones', 'system', 'updates', 'view-refresh-symbolic',
     'update;updates;automatic;flatpak;kernel;firmware;nodalix;', 'actualizar;actualizaciones;automáticas;flatpak;kernel;firmware;nodalix;'),
    ('diagnostics', 'System Diagnostics', 'Diagnóstico del sistema', 'system', 'diagnostics', 'dialog-information-symbolic',
     'diagnostics;system;memory;display;services;performance;', 'diagnóstico;sistema;memoria;pantallas;servicios;rendimiento;'),
    ('version', 'Nodalix OS Version', 'Versión de Nodalix OS', 'system', 'about', 'org.gnome.Settings-about-symbolic',
     'version;nodalix;system;about;information;', 'versión;nodalix;sistema;acerca;información;'),
    ('localsend', 'LocalSend', 'LocalSend', 'sharing', 'localsend', 'send-to-symbolic',
     'localsend;share;files;nearby;transfer;', 'localsend;compartir;archivos;cercanos;transferir;'),
    ('wallpapers', 'Animated Wallpapers', 'Fondos animados', 'background', 'animated-wallpapers', 'preferences-desktop-wallpaper-symbolic',
     'wallpaper;wallpapers;animated;video;appearance;background;', 'fondo;fondos;animados;vídeo;apariencia;escritorio;'),
)
files = []
for slug, english, spanish, panel, subpage, icon, keywords, localized in search_sections:
    filename = f'gnome-nodalix-{slug}-panel.desktop'
    files.append(filename)
    (root/'search-provider'/filename).write_text(f'''[Desktop Entry]
Type=Application
Name={english}
Name[es]={spanish}
Exec=gnome-control-center nodalix-{slug}
Icon={icon}
Terminal=false
StartupNotify=true
NoDisplay=true
OnlyShowIn=GNOME;Unity;
Categories=GNOME;GTK;Settings;X-GNOME-Settings-Panel;X-GNOME-SystemSettings;
Keywords={keywords}{localized}
Keywords[es]={localized}{keywords}
X-Nodalix-Parent={panel}
X-Nodalix-Subpage={subpage}
''')
p=root/'search-provider/meson.build';t=p.read_text()
p.write_text(t+'\ninstall_data('+', '.join(repr(filename) for filename in files)+',\n             install_dir: control_center_desktopdir)\n')
p=root/'shell/cc-panel-loader.c';t=p.read_text();anchor='    { CC_CATEGORY_SYSTEM, "about" },'
assert anchor in t
p.write_text(t.replace(anchor,anchor+''.join(f'\n    {{ CC_CATEGORY_SYSTEM, "nodalix-{row[0]}" }},' for row in search_sections),1))

# Apply the same parent/subpage routing when a result is selected inside
# Control Center; the Shell provider uses the desktop file's normal Exec.
p=root/'shell/cc-window.c';t=p.read_text()
if '#include <gio/gdesktopappinfo.h>' not in t:
    t=t.replace('#include "cc-window.h"','#include "cc-window.h"\n#include <gio/gdesktopappinfo.h>')
t=t.replace('    g_autoptr(GVariant) system_param_overwrite = NULL;', '''    g_autoptr(GVariant) system_param_overwrite = NULL;
    g_autoptr(GAppInfo) entry_info = NULL;
    g_autofree gchar *nodalix_parent = NULL;
    g_autofree gchar *nodalix_subpage = NULL;''',1)
t=t.replace('    if (category == CC_CATEGORY_SYSTEM) {','''    if (g_str_has_prefix (start_id, "nodalix-")) {
        gtk_tree_model_get (GTK_TREE_MODEL (self->store), &iter, COL_APP, &entry_info, -1);
        if (G_IS_DESKTOP_APP_INFO (entry_info)) {
            nodalix_parent = g_desktop_app_info_get_string (G_DESKTOP_APP_INFO (entry_info), "X-Nodalix-Parent");
            nodalix_subpage = g_desktop_app_info_get_string (G_DESKTOP_APP_INFO (entry_info), "X-Nodalix-Subpage");
        }
    }
    if (nodalix_parent && nodalix_subpage) {
        GVariantBuilder arguments;
        g_variant_builder_init (&arguments, G_VARIANT_TYPE ("av"));
        g_variant_builder_add (&arguments, "v", g_variant_new_string (nodalix_subpage));
        system_param_overwrite = g_variant_ref_sink (g_variant_builder_end (&arguments));
        start_id = nodalix_parent;
    } else if (category == CC_CATEGORY_SYSTEM) {''',1)
p.write_text(t)
p=root/'panels/applications/cc-applications-panel.c';t=p.read_text()
anchor='''            g_variant_unref (v);

            select_app (CC_APPLICATIONS_PANEL (object), first_arg, TRUE);'''
assert anchor in t
p.write_text(t.replace(anchor,'''            if (g_strcmp0 (first_arg, "storage") == 0) {
                CcPanel *panel = CC_PANEL (object);
                AdwNavigationPage *subpage = cc_panel_get_static_subpage (panel, "storage");
                if (subpage)
                    g_ptr_array_add (cc_panel_get_navigation_stack (panel), subpage);
            } else {
                select_app (CC_APPLICATIONS_PANEL (object), first_arg, TRUE);
            }
            g_variant_unref (v);''',1))
