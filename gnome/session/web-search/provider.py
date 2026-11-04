#!/usr/bin/python3
"""Open Overview searches in the default browser."""
import configparser
import ctypes
import json
import sys
from pathlib import Path
from urllib.parse import urlencode

from gi.repository import Gio, GLib


BUS = 'com.nodalix.WebSearch'
PATH = '/com/nodalix/WebSearch'
GECKO_BROWSERS = {
    'zen.desktop': '.zen',
    'zen-browser.desktop': '.zen',
    'app.zen_browser.zen.desktop': '.zen',
    'firefox.desktop': '.mozilla/firefox',
    'firefox-esr.desktop': '.mozilla/firefox',
    'org.mozilla.firefox.desktop': '.mozilla/firefox',
}
DESKTOP_FILE_ARGUMENTS = {'%f', '%F', '%u', '%U', '@@', '@@u'}
SEARCH_SETTINGS_LIMIT = 4 * 1024 * 1024
# Gecko stores the IDs of bundled engines in the profile. The URLs below
# are public search endpoints and deliberately omit partner parameters.
GECKO_SEARCH_ENDPOINTS = {
    'bing': 'https://www.bing.com/search',
    'ddg': 'https://duckduckgo.com/',
    'ecosia': 'https://www.ecosia.org/search',
    'google': 'https://www.google.com/search',
    'qwant': 'https://www.qwant.com/',
}
INTERFACE = Gio.DBusNodeInfo.new_for_xml('''
<node><interface name="org.gnome.Shell.SearchProvider2">
  <method name="GetInitialResultSet"><arg type="as" direction="in"/><arg type="as" direction="out"/></method>
  <method name="GetSubsearchResultSet"><arg type="as" direction="in"/><arg type="as" direction="in"/><arg type="as" direction="out"/></method>
  <method name="GetResultMetas"><arg type="as" direction="in"/><arg type="aa{sv}" direction="out"/></method>
  <method name="ActivateResult"><arg type="s" direction="in"/><arg type="as" direction="in"/><arg type="u" direction="in"/></method>
  <method name="LaunchSearch"><arg type="as" direction="in"/><arg type="u" direction="in"/></method>
</interface></node>
''').interfaces[0]


def query(terms):
    return ' '.join(terms).strip()[:512]


def gecko_profile(root):
    profiles = configparser.ConfigParser(interpolation=None)
    if not profiles.read(root / 'profiles.ini', encoding='utf-8'):
        return None
    defaults = {profiles[section].get('Default') for section in profiles.sections()
                if section.startswith('Install')} - {None, ''}
    if not defaults:
        defaults = {profiles[section].get('Path') for section in profiles.sections()
                    if section.startswith('Profile') and profiles[section].get('Default') == '1'}
        defaults -= {None, ''}
    if len(defaults) == 1:
        return root / defaults.pop()
    return None


def gecko_search_url(browser_id, command, value):
    # A custom launcher may target a different profile or a private engine.
    if any(argument.lstrip('-').lower().split('=', 1)[0] in
           {'profile', 'p', 'profilemanager', 'private', 'private-window', 'no-remote', 'new-instance'}
           for argument in command[1:]):
        return None
    root = Path.home()
    if Path(command[0]).name == 'flatpak':
        root = root / '.var/app' / browser_id.removesuffix('.desktop')
    try:
        profile = gecko_profile(root / GECKO_BROWSERS[browser_id])
        if profile is None:
            return None
        with (profile / 'search.json.mozlz4').open('rb') as settings_file:
            data = settings_file.read(SEARCH_SETTINGS_LIMIT + 1)
        size = int.from_bytes(data[8:12], 'little')
        if (not data.startswith(b'mozLz40\0') or len(data) > SEARCH_SETTINGS_LIMIT
                or not 0 < size <= SEARCH_SETTINGS_LIMIT):
            return None
        decompress = ctypes.CDLL('liblz4.so.1').LZ4_decompress_safe
        decompress.argtypes = [ctypes.c_char_p, ctypes.c_void_p, ctypes.c_int, ctypes.c_int]
        decompress.restype = ctypes.c_int
        buffer = ctypes.create_string_buffer(size)
        if decompress(data[12:], buffer, len(data) - 12, size) != size:
            return None
        metadata = json.loads(buffer.raw)['metaData']
        # Old profiles store a customized engine by name. Let the browser
        # resolve it, along with any engine whose URL is unknown here.
        if metadata.get('current') and 'defaultEngineId' not in metadata:
            return None
        engine = metadata.get('defaultEngineId') or metadata.get('appDefaultEngineId')
        endpoint = GECKO_SEARCH_ENDPOINTS.get(engine)
    except (OSError, ValueError, AttributeError, KeyError, TypeError, configparser.Error):
        return None
    return endpoint + '?' + urlencode({'q': value}) if endpoint else None


def open_search(value):
    browser = Gio.AppInfo.get_default_for_uri_scheme('https')
    if not value and browser:
        browser.launch([], None)
        return
    if browser and browser.get_id() in GECKO_BROWSERS:
        _, command = GLib.shell_parse_argv(browser.get_commandline())
        command = [argument for argument in command if argument not in DESKTOP_FILE_ARGUMENTS]
        url = gecko_search_url(browser.get_id(), command, value)
        if url:
            arguments = ['--new-tab', url]
        else:
            # Gecko rejects a search parameter that starts with a dash.
            term = ' ' + value if value.startswith('-') else value
            arguments = ['--search', term]
        Gio.Subprocess.new([*command, *arguments], Gio.SubprocessFlags.NONE)
        return

    url = 'https://duckduckgo.com/'
    if value:
        url += '?' + urlencode({'q': value})
    Gio.AppInfo.launch_default_for_uri(url, None)


class Provider:
    def __init__(self, loop):
        self.loop = loop
        self.timeout = 0

    def reset_timeout(self):
        if self.timeout:
            GLib.source_remove(self.timeout)
        self.timeout = GLib.timeout_add_seconds(20, self.loop.quit)

    def register(self, connection, _name):
        connection.register_object(PATH, INTERFACE, self.call, None, None)
        self.reset_timeout()

    def call(self, _connection, _sender, _path, _interface, method, parameters, invocation):
        self.reset_timeout()
        arguments = parameters.unpack()
        try:
            if method in ('GetInitialResultSet', 'GetSubsearchResultSet'):
                value = query(arguments[-1])
                result = GLib.Variant('(as)', ([value] if value else [],))
            elif method == 'GetResultMetas':
                result = GLib.Variant('(aa{sv})', ([
                    {'id': GLib.Variant('s', value), 'name': GLib.Variant('s', value),
                     'gicon': GLib.Variant('s', 'web-browser-symbolic')}
                    for value in arguments[0]
                ],))
            else:
                value = arguments[0] if method == 'ActivateResult' else query(arguments[0])
                open_search(value)
                result = GLib.Variant('()', ())
        except GLib.Error as error:
            invocation.return_gerror(error)
        else:
            invocation.return_value(result)


def main():
    if sys.argv[1:] == ['--open']:
        open_search('')
        return

    loop = GLib.MainLoop()
    provider = Provider(loop)
    owner = Gio.bus_own_name(Gio.BusType.SESSION, BUS, Gio.BusNameOwnerFlags.NONE,
                             provider.register, None, lambda *_: loop.quit())
    try:
        loop.run()
    finally:
        Gio.bus_unown_name(owner)


if __name__ == '__main__':
    main()
