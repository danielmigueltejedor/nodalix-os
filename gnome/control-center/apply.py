#!/usr/bin/python3
"""Apply the bounded Nodalix additions to GNOME Control Center 51.0."""
from pathlib import Path
import shutil
import sys
root=Path(sys.argv[1]);source=Path(__file__).resolve().parent
if 'cc_nodalix_updates_page' in (root/'panels/system/cc-system-panel.c').read_text():
    raise SystemExit('Already patched')
for name in ['cc-nodalix-pages.h','cc-nodalix-pages.c']:
    shutil.copy2(source/name,root/'panels/system'/name)
p=root/'panels/system/meson.build';t=p.read_text().replace("  'cc-system-panel.c',","  'cc-system-panel.c',\n  'cc-nodalix-pages.c',");t=t.replace("deps = common_deps + [", "deps = common_deps + [\n  dependency('json-glib-1.0'),");p.write_text(t)
p=root/'panels/system/cc-system-panel.c';t=p.read_text().replace('#include "cc-system-panel.h"','#include "cc-system-panel.h"\n#include "cc-nodalix-pages.h"')
t=t.replace('    g_type_ensure (CC_TYPE_ABOUT_PAGE);','    g_type_ensure (CC_TYPE_NODALIX_UPDATES_PAGE);\n    g_type_ensure (CC_TYPE_ABOUT_PAGE);')
t=t.replace('    cc_panel_add_static_subpage (CC_PANEL (self), "about", CC_TYPE_ABOUT_PAGE);','    cc_panel_add_static_subpage (CC_PANEL (self), "updates", CC_TYPE_NODALIX_UPDATES_PAGE);\n    cc_panel_add_static_subpage (CC_PANEL (self), "about", CC_TYPE_ABOUT_PAGE);')
t=t.replace('show_software_updates_group (self));','FALSE);')
p.write_text(t)
p=root/'panels/system/cc-system-panel.blp';t=p.read_text().replace('        $CcListRow about_row {','''        $CcListRow {
          title: "Actualizaciones";
          subtitle: "Nodalix, aplicaciones, programas y kernel";
          icon-name: "system-software-update-symbolic";
          show-arrow: true;
          action-name: "navigation.push";
          action-target: "'updates'";
        }

        $CcListRow about_row {''');p.write_text(t)
p=root/'panels/background/cc-background-panel.c';t=p.read_text().replace('#include "cc-background-panel.h"','#include "cc-background-panel.h"\n#include "../system/cc-nodalix-pages.h"')
t=t.replace('    g_signal_handlers_block_by_func (self->settings, on_settings_changed, self);','''    g_dbus_connection_call (self->connection, "com.nodalix.Settings", "/com/nodalix/Settings",
                            "com.nodalix.Settings1", "StopWallpaper", NULL, NULL,
                            G_DBUS_CALL_FLAGS_NONE, 2000, NULL, NULL, NULL);
    g_signal_handlers_block_by_func (self->settings, on_settings_changed, self);''',1)
t=t.replace('    gtk_widget_init_template (GTK_WIDGET (self));','    gtk_widget_init_template (GTK_WIDGET (self));\n    cc_panel_add_static_subpage (CC_PANEL (self), "animated-backgrounds", CC_TYPE_NODALIX_WALLPAPERS_PAGE);')
p.write_text(t)
p=root/'panels/background/cc-background-panel.blp';t=p.read_text();pos=t.rfind('        };');t=t[:pos]+'''          Adw.PreferencesGroup {
            title: "Fondos animados";
            Adw.ActionRow {
              title: "Colección Nodalix";
              subtitle: "Fondos de vídeo, sin sonido";
              icon-name: "media-playback-start-symbolic";
              activatable: true;
              action-name: "navigation.push";
              action-target: "'animated-backgrounds'";
              [suffix] Image { icon-name: "go-next-symbolic"; }
            }
          }
'''+t[pos:];p.write_text(t)
