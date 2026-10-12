#!/usr/bin/python3
"""Apply the bounded Nodalix additions to GNOME Control Center 51.0."""
from pathlib import Path
import shutil
import sys
root=Path(sys.argv[1]);source=Path(__file__).resolve().parent
if 'cc_nodalix_updates_page' in (root/'panels/system/cc-system-panel.c').read_text():
    raise SystemExit('Already patched')
for name in ['cc-nodalix-pages.h','cc-nodalix-pages.c','cc-nodalix-localsend.c']:
    shutil.copy2(source/name,root/'panels/system'/name)
p=root/'panels/system/meson.build';t=p.read_text().replace("  'cc-system-panel.c',","  'cc-system-panel.c',\n  'cc-nodalix-pages.c',\n  'cc-nodalix-localsend.c',");t=t.replace("deps = common_deps + [", "deps = common_deps + [\n  dependency('json-glib-1.0'),");p.write_text(t)
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
p=root/'panels/background/cc-background-panel.blp';t=p.read_text();pos=t.rfind('        };');t=t[:pos]+'''          Adw.PreferencesGroup {
            title: "Fondos animados";
            Adw.Bin {
              styles ["card"]
              accessible-role: group;
              $CcNodalixBackgroundChooser { hexpand: true; }
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
