#!/usr/bin/python3
"""GNOME Overview web results opened through the default HTTPS handler."""
from urllib.parse import urlencode

from gi.repository import Gio, GLib


BUS = 'com.nodalix.WebSearch'
PATH = '/com/nodalix/WebSearch'
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


def open_search(value):
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
