"""Fake updater service; never calls logind or changes the system."""
import json
from gi.repository import Gio,GLib
XML='''<node><interface name="com.nodalix.Settings1"><method name="GetUpdates"><arg type="s" direction="out"/></method><method name="Restart"/><method name="TestRestartCount"><arg type="u" direction="out"/></method></interface></node>'''
state={'busy':False,'checking':False,'reboot_mandatory':True,'reboot_required':True,'message':'Reinicio obligatorio para terminar la migración a GNOME','log':'','sources':[{'available':True,'detail':'Instalado'} for _ in range(5)]}
count=0
loop=GLib.MainLoop()
def call(connection,sender,path,interface,method,parameters,invocation):
    global count
    if method=='GetUpdates':invocation.return_value(GLib.Variant('(s)',(json.dumps(state),)))
    elif method=='Restart':count+=1;invocation.return_value(None)
    elif method=='TestRestartCount':invocation.return_value(GLib.Variant('(u)',(count,)))
def acquired(connection,name):
    connection.register_object('/com/nodalix/Settings',Gio.DBusNodeInfo.new_for_xml(XML).interfaces[0],call,None,None)
Gio.bus_own_name(Gio.BusType.SESSION,'com.nodalix.Settings',Gio.BusNameOwnerFlags.NONE,acquired,None,None)
loop.run()
