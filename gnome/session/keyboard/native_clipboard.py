"""Two fixed clipboard chords for GNOME native custom shortcuts.

The system broker checks the active unlocked seat owner. It reads only modifier
bitmaps (EVIOCGKEY), never input events or clipboard contents. No shell commands,
arbitrary key sequences, key logging, or window inspection are supported.
"""
import fcntl
import errno
import os
from pathlib import Path
import struct
import threading
import time
from gi.repository import Gio, GLib

BUS='com.nodalix.Keyboard'
PATH='/com/nodalix/Keyboard'
XML='<node><interface name="com.nodalix.Keyboard"><method name="Edit"><arg name="action" type="s" direction="in"/></method></interface></node>'
MODIFIERS=(29,97,42,54,56,100,125,126)
CHORDS={'copy':(29,110),'paste':(42,110)}  # Ctrl+Insert / Shift+Insert


def allowed_seat(uid,user,active,locked,session_type,remote):
    return uid==user and active and not locked and not remote and session_type in ('wayland','x11')


def properties(connection,path,interface):
    return connection.call_sync('org.freedesktop.login1',path,'org.freedesktop.DBus.Properties',
        'GetAll',GLib.Variant('(s)',(interface,)),None,Gio.DBusCallFlags.NONE,2000,None).unpack()[0]


def authorized(connection,sender):
    uid=connection.call_sync('org.freedesktop.DBus','/org/freedesktop/DBus','org.freedesktop.DBus',
        'GetConnectionUnixUser',GLib.Variant('(s)',(sender,)),None,Gio.DBusCallFlags.NONE,2000,None).unpack()[0]
    seat=properties(connection,'/org/freedesktop/login1/seat/seat0','org.freedesktop.login1.Seat')
    session=properties(connection,seat['ActiveSession'][1],'org.freedesktop.login1.Session')
    if not allowed_seat(uid,session['User'][0],session['Active'],session['LockedHint'],session['Type'],session['Remote']):
        raise PermissionError('El atajo requiere la sesión local activa y desbloqueada')


def modifiers_held(fds):
    for fd in fds:
        bitmap=bytearray(96)
        try:fcntl.ioctl(fd,0x80604518,bitmap,True)  # EVIOCGKEY(96)
        except OSError as error:
            if error.errno in (errno.ENODEV,errno.ENOENT):continue
            raise  # An unreadable modifier state must never permit injection.
        if any(bitmap[key//8] & (1<<(key%8)) for key in MODIFIERS):return True
    return False


def wait_released(fds,pressed=modifiers_held,clock=time.monotonic,sleep=time.sleep):
    deadline=clock()+3
    while pressed(fds):
        if clock()>=deadline:raise TimeoutError('Suelta las teclas modificadoras para copiar o pegar')
        sleep(.015)


class Keyboard:
    def __init__(self):self.fd=None
    def ready(self):
        if self.fd is not None:return
        fd=os.open('/dev/uinput',os.O_WRONLY|os.O_NONBLOCK|os.O_CLOEXEC)
        try:
            fcntl.ioctl(fd,0x40045564,1)  # UI_SET_EVBIT(EV_KEY)
            for key in range(1,256):fcntl.ioctl(fd,0x40045565,key)
            setup=struct.pack('HHHH80sI',6,0x0fac,0x0400,1,b'Nodalix native clipboard',0)
            fcntl.ioctl(fd,0x405c5503,setup);fcntl.ioctl(fd,0x5501)
            self.fd=fd;time.sleep(.3)  # Initial udev discovery, once per broker.
        except Exception:os.close(fd);raise
    def event(self,key,value):
        os.write(self.fd,struct.pack('llHHi',0,0,1,key,value))
        os.write(self.fd,struct.pack('llHHi',0,0,0,0,0))
    def edit(self,action):
        if action not in CHORDS:raise ValueError('Acción de portapapeles no admitida')
        self.ready()
        fds=[]
        try:
            for path in Path('/dev/input').glob('event*'):
                try:fds.append(os.open(path,os.O_RDONLY|os.O_NONBLOCK|os.O_CLOEXEC))
                except OSError as error:
                    if error.errno not in (errno.ENODEV,errno.ENOENT):raise
            if not fds:raise OSError('No se pudo comprobar el estado del teclado')
            wait_released(fds)
            modifier,key=CHORDS[action]
            try:
                self.event(modifier,1);time.sleep(.005)
                self.event(key,1);self.event(key,0)
            finally:self.event(modifier,0)
        finally:
            for fd in fds:os.close(fd)
    def close(self):
        if self.fd is not None:
            fcntl.ioctl(self.fd,0x5502);os.close(self.fd);self.fd=None


class Broker:
    def __init__(self):self.keyboard=Keyboard();self.lock=threading.Lock()
    def call(self,connection,sender,path,interface,method,parameters,invocation):
        action=parameters.unpack()[0]
        if method!='Edit' or action not in CHORDS:
            invocation.return_dbus_error(BUS+'.Error','Acción no admitida');return
        if not self.lock.acquire(blocking=False):
            invocation.return_dbus_error(BUS+'.Busy','Espera a que termine el atajo anterior');return
        def execute():
            error=None
            try:
                authorized(connection,sender)
                # The desktop may lock while the user holds Super. Check again
                # immediately before emission, after the modifier wait.
                self.keyboard.ready()
                def validated_event(key,value):
                    if value==1 and key in (29,42):authorized(connection,sender)
                    original(key,value)
                original=self.keyboard.event
                self.keyboard.event=validated_event
                try:self.keyboard.edit(action)
                finally:self.keyboard.event=original
            except (OSError,GLib.Error,ValueError,TimeoutError,PermissionError) as failure:error=str(failure)
            finally:self.lock.release()
            def reply():
                if error:invocation.return_dbus_error(BUS+'.Error',error)
                else:invocation.return_value(None)
                return GLib.SOURCE_REMOVE
            GLib.idle_add(reply)
        threading.Thread(target=execute,daemon=True).start()


def main():
    if os.geteuid()!=0:raise SystemExit('Run through the installed system service')
    broker=Broker();loop=GLib.MainLoop()
    def acquired(connection,name):
        connection.register_object(PATH,Gio.DBusNodeInfo.new_for_xml(XML).interfaces[0],broker.call,None,None)
    Gio.bus_own_name(Gio.BusType.SYSTEM,BUS,Gio.BusNameOwnerFlags.NONE,acquired,None,None)
    try:loop.run()
    finally:broker.keyboard.close()

if __name__=='__main__':main()
