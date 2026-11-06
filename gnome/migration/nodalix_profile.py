"""Apply the packaged Nodalix Shell desktop profile without replacing user choices."""
import json
from pathlib import Path
from gi.repository import Gio, GLib

# The built-in modules ship with the Shell; the other roots keep the same
# settings reachable from the stock GNOME rollback path.
MODULE_ROOTS = (
    Path('/usr/share/gnome-shell/nodalix-modules'),
    Path('/usr/share/gnome-shell/nodalix-runtime'),
    Path('/usr/share/gnome-shell/extensions'),
)


def module_schema(uuid, name, roots):
    for root in roots:
        directory = root/uuid/'schemas'
        if not (directory/'gschemas.compiled').is_file():
            continue
        source = Gio.SettingsSchemaSource.new_from_directory(
            str(directory), Gio.SettingsSchemaSource.get_default(), False)
        schema = source.lookup(name, False)
        if schema:
            return schema
    return None


def module_present(uuid, roots):
    return any((root/uuid/'metadata.json').is_file() for root in roots)


def apply_defaults(rows, roots=MODULE_ROOTS):
    """Write only keys the user has not set. Return the rows that could not be applied."""
    pending = []
    for row in rows:
        schema = module_schema(row['uuid'], row['schema'], roots)
        if schema is None:
            pending.append(row['schema'])
            continue
        prefs = Gio.Settings.new_full(schema, None, None)
        for key, text in row['values'].items():
            if not schema.has_key(key):
                continue
            try:
                value = GLib.Variant.parse(None, text, None, None)
            except GLib.Error:
                continue
            definition = schema.get_key(key)
            if value.get_type_string() != definition.get_value_type().dup_string() or not definition.range_check(value):
                continue
            if prefs.get_user_value(key) is None:
                prefs.set_value(key, value)
    return pending


def enable_modules(settings, uuids, roots=MODULE_ROOTS):
    """Add new built-in modules unless the user already turned them off."""
    enabled = list(settings.get_strv('enabled-extensions'))
    disabled = settings.get_strv('disabled-extensions')
    missing = []
    for uuid in uuids:
        if uuid in enabled or uuid in disabled:
            continue
        if not module_present(uuid, roots):
            missing.append(uuid)
            continue
        enabled.append(uuid)
    if enabled != list(settings.get_strv('enabled-extensions')):
        settings.set_strv('enabled-extensions', enabled)
    return missing


def apply_profile(path, roots=MODULE_ROOTS, shell=None):
    """Return True when every packaged module was available; otherwise retry at the next login."""
    profile = json.loads(Path(path).read_text(encoding='utf-8'))
    pending = apply_defaults(profile.get('extensions', []), roots)
    shell = shell or Gio.Settings.new('org.gnome.shell')
    pending += enable_modules(shell, profile.get('enableModules', []), roots)
    Gio.Settings.sync()
    return not pending
