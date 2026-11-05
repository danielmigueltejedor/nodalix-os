#!/usr/bin/python3
"""Open Overview searches in the default browser."""
import configparser
import ctypes
import json
import os
import sys
from pathlib import Path
from urllib.parse import urlencode

from gi.repository import Gio, GLib


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
    home = Path.home()
    relative_root = GECKO_BROWSERS[browser_id]
    if Path(command[0]).name == 'flatpak':
        home = home / '.var/app' / browser_id.removesuffix('.desktop')
        config = home / 'config'
    else:
        config = Path(os.environ.get('XDG_CONFIG_HOME', ''))
        if not config.is_absolute():
            config = home / '.config'
    try:
        root = home / relative_root
        # Recent Gecko builds create new profiles under XDG_CONFIG_HOME and
        # keep using the legacy directory on existing installations.
        if not root.exists() and not os.environ.get('MOZ_LEGACY_HOME', '').startswith('1'):
            root = config / relative_root.lstrip('.')
        profile = gecko_profile(root)
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


def main():
    if len(sys.argv) != 3 or sys.argv[1] != '--search':
        raise SystemExit('Usage: web-search-provider --search QUERY')
    value = query(sys.argv[2:])
    if value:
        open_search(value)


if __name__ == '__main__':
    main()
