"""Fake updater service; never calls logind or changes the system."""
import json
from gi.repository import Gio,GLib
XML='''<node><interface name="com.nodalix.Settings1"><method name="GetUpdates"><arg type="s" direction="out"/></method><method name="Restart"/><method name="TestRestartCount"><arg type="u" direction="out"/></method><method name="SetAutomaticUpdate"><arg type="s" direction="in"/><arg type="b" direction="in"/></method><method name="RefreshAutomaticUpdates"/><method name="TestAutomaticCounts"><arg type="u" direction="out"/><arg type="u" direction="out"/></method><signal name="Changed"/></interface></node>'''
state={'busy':False,'checking':False,'reboot_mandatory':True,'reboot_required':True,'message':'Reinicio obligatorio para terminar la migración a GNOME','log':'','sources':[{'available':True,'detail':'Instalado'} for _ in range(5)]}
count=0
enabled_calls=0
disabled_calls=0
state.update(automatic_busy=False,automatic_error='',automatic={kind:{'enabled':False,'partial':False,'active':False,'available':True,'owned':True} for kind in ('nodalix','system','flatpak','firmware')})
loop=GLib.MainLoop()
def call(connection,sender,path,interface,method,parameters,invocation):
    global count,enabled_calls,disabled_calls
    if method=='GetUpdates':invocation.return_value(GLib.Variant('(s)',(json.dumps(state),)))
    elif method=='Restart':count+=1;invocation.return_value(None)
    elif method=='TestRestartCount':invocation.return_value(GLib.Variant('(u)',(count,)))
    elif method=='RefreshAutomaticUpdates':invocation.return_value(None)
    elif method=='SetAutomaticUpdate':
        kind,enabled=parameters.unpack()
        if enabled:
            enabled_calls+=1
            if enabled_calls==1:
                state['automatic_error']='Autorización cancelada'
                invocation.return_dbus_error('com.nodalix.Settings1.Error','Autorización cancelada')
                return
        else:disabled_calls+=1
        state['automatic'][kind].update(enabled=enabled,active=enabled)
        state['automatic_error']=''
        invocation.return_value(None)
        connection.emit_signal(None,path,interface,'Changed',None)
    elif method=='TestAutomaticCounts':invocation.return_value(GLib.Variant('(uu)',(enabled_calls,disabled_calls)))
def acquired(connection,name):
    connection.register_object('/com/nodalix/Settings',Gio.DBusNodeInfo.new_for_xml(XML).interfaces[0],call,None,None)
Gio.bus_own_name(Gio.BusType.SESSION,'com.nodalix.Settings',Gio.BusNameOwnerFlags.NONE,acquired,None,None)
loop.run()
