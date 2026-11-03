"""Query the built-in GNOME Settings provider on a disposable desktop."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

from gi.repository import Gio, GLib

output = Path(sys.argv[1])
data_home = Path(os.environ['XDG_DATA_HOME'])
state_home = Path(os.environ['XDG_STATE_HOME'])
package = Path(os.environ['NODALIX_SEARCH_PACKAGE'])
desktop_entries = data_home/'applications'
desktop_entries.mkdir(parents=True, exist_ok=True)
for entry in (package/'usr/share/applications').glob('gnome-nodalix-*-panel.desktop'):
    shutil.copy2(entry, desktop_entries/entry.name)

applications = (Path.home()/'Aplicaciones', Path.home()/'Applications')
assert not any(folder.exists() for folder in applications)
subprocess.run(['/usr/bin/nodalix-app-library', 'sync'], check=True)
folder = next(folder for folder in applications if folder.is_dir())
registry = json.loads((state_home/'nodalix/applications/registry.json').read_text())
assert (folder/registry['org.gnome.Settings.desktop']['filename']).is_file()

# Exercise an existing profile whose Applications folder is missing on upgrade.
layout = state_home/'nodalix/layout/paths.json'
layout.parent.mkdir(parents=True, exist_ok=True)
layout.write_text(json.dumps({'applications': str(folder)}))
old_folder = folder.with_name('Previous QA catalog')
folder.rename(old_folder)
subprocess.run(['/usr/bin/nodalix-app-library', 'sync'], check=True)
entry_name = registry['org.gnome.Settings.desktop']['filename']
assert (folder/entry_name).is_file() and (old_folder/entry_name).is_file()


bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
name = 'org.gnome.Settings.SearchProvider'
object_path = '/org/gnome/Settings/SearchProvider'
provider = package/'usr/libexec/gnome-control-center-search-provider'
env = dict(os.environ)
env['PATH'] = str(package/'usr/bin')+':'+env['PATH']
queries = {
    'documentos grandes': 'nodalix-storage',
    'actualizaciones automáticas': 'nodalix-updates',
    'diagnóstico': 'nodalix-diagnostics',
    'versión nodalix': 'nodalix-version',
    'localsend': 'nodalix-localsend',
    'fondos animados': 'nodalix-wallpapers',
}
with (output/'search-provider.log').open('w') as log:
    process = subprocess.Popen([str(provider)], env=env, stdout=log, stderr=log)
    try:
        for _ in range(60):
            owned, = bus.call_sync('org.freedesktop.DBus', '/org/freedesktop/DBus',
                'org.freedesktop.DBus', 'NameHasOwner', GLib.Variant('(s)', (name,)), None,
                Gio.DBusCallFlags.NONE, 3000, None).unpack()
            if owned:
                break
            assert process.poll() is None, 'Settings search provider exited'
            time.sleep(.1)
        else:
            raise AssertionError('Settings search provider did not acquire its bus name')

        def call(method, args):
            return bus.call_sync(name, object_path, 'org.gnome.Shell.SearchProvider2',
                method, args, None, Gio.DBusCallFlags.NO_AUTO_START, 10000, None).unpack()

        for query, expected in queries.items():
            results, = call('GetInitialResultSet', GLib.Variant('(as)', (query.split(),)))
            assert expected in results, (query, results)
        ids = list(queries.values())
        metas, = call('GetResultMetas', GLib.Variant('(as)', (ids,)))
        assert [item['id'] for item in metas] == ids
        assert all(item.get('name') and item.get('icon') for item in metas)
        for slug in ('storage', 'updates', 'diagnostics', 'version', 'localsend', 'wallpapers'):
            call('ActivateResult', GLib.Variant('(sasu)', ('nodalix-'+slug, [], 0)))
            time.sleep(2)
            bus.call_sync('org.gnome.Shell', '/org/gnome/Shell', 'org.gnome.Shell', 'Eval',
                GLib.Variant('(s)', ('Main.overview.hide();true',)), None,
                Gio.DBusCallFlags.NONE, 5000, None)
            time.sleep(.6)
            bus.call_sync('org.gnome.Shell.Screenshot', '/org/gnome/Shell/Screenshot',
                'org.gnome.Shell.Screenshot', 'Screenshot',
                GLib.Variant('(bbs)', (False, False, str(output/(slug+'.png')))), None,
                Gio.DBusCallFlags.NONE, 5000, None)
    finally:
        process.terminate()
        process.wait(timeout=10)

(output/'settings-search.json').write_text(json.dumps({
    'native_gnome_settings_provider': True,
    'search_queries_passed': len(queries),
    'apps_directory_created_in_fresh_profile': True,
    'apps_catalog_contains_settings': True,
    'missing_apps_directory_recreated_for_existing_profile': True,
}, indent=2)+'\n')
print('Six native Settings searches and automatic Applications creation passed')
