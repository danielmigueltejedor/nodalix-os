"""Native GTK fixture: synthetic contacts and a fake dial backend only."""
import gi,json,time
from pathlib import Path
gi.require_version('Gtk','4.0');gi.require_version('Adw','1')
from gi.repository import Adw,Gtk,GLib,Gio
from iphonebridge.eds_contacts import DialContact
from iphonebridge.ui import contact_picker
from iphonebridge.ui.calls import CallsPage

def spin(until):
    deadline=time.monotonic()+5
    while not until() and time.monotonic()<deadline:
        while GLib.MainContext.default().pending():GLib.MainContext.default().iteration(False)
        time.sleep(.01)
    assert until()

class Client:
    def __init__(self):self.dialed=[]
    def connect(self,*args):return 1
    def list_calls(self):return []
    def dial(self,number,ok,error):self.dialed.append(number);ok('/qa/fake-call')

Adw.init();client=Client();toasts=[]
contacts=[DialContact('José QA','+34600000000','Móvil'),DialContact('José QA','910000000','Casa'),DialContact('Otra persona','900000000','Trabajo')]
contact_picker.read_gnome_contacts=lambda cancel:contacts
page=CallsPage(client,toasts.append)
window=Adw.Window(content=page,default_width=600,default_height=660);window.present()
dialog=contact_picker.ContactPicker(page._dial_number);dialog.present(page)
spin(lambda:not dialog._loading)
assert dialog._store.get_n_items()==3
dialog._search.set_text('jose');spin(lambda:dialog._filtered.get_n_items()==2)
until=time.monotonic()+.6
while time.monotonic()<until:
    while GLib.MainContext.default().pending():GLib.MainContext.default().iteration(False)
    time.sleep(.01)
output=Path.cwd()
bus=Gio.bus_get_sync(Gio.BusType.SESSION,None)
bus.call_sync('org.gnome.Shell.Screenshot','/org/gnome/Shell/Screenshot','org.gnome.Shell.Screenshot','Screenshot',GLib.Variant('(bbs)',(False,False,str((output/'contact-picker.png').resolve()))),None,Gio.DBusCallFlags.NONE,5000,None)
dialog._choose(dialog._filtered.get_item(1))
assert client.dialed==['910000000'],client.dialed
opened=[];page._show_contacts=lambda query='':opened.append(query)
page._entry.set_text('José');page._on_dial(None)
assert opened==['José'] and len(client.dialed)==1
window.close()
(output/'contact-picker.json').write_text(json.dumps({'synthetic_contacts':3,'accent_insensitive_matches':2,'selected_local_number_preserved':True,'name_opens_picker_without_guessing':True,'live_calls_made':False,'native_virtualized_list':True},indent=2)+'\n')
print('Native contact picker: search, multiple phones, explicit selection and fake dialing passed')
