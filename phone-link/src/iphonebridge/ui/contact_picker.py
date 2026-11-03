"""A searchable, virtualized phone picker backed by GNOME's address books."""
from __future__ import annotations

import threading
from gi.repository import Adw,Gio,GLib,GObject,Gtk,Pango
from iphonebridge.eds_contacts import read_gnome_contacts,search_text
from iphonebridge.events import normalize_phone


class ContactItem(GObject.Object):
    def __init__(self,contact):
        super().__init__()
        self.contact=contact
        self.search=search_text(contact.name+' '+contact.number)


class ContactPicker(Adw.Dialog):
    def __init__(self,on_selected,query=''):
        super().__init__(title='Contactos de GNOME',content_width=480,content_height=580)
        self._select=on_selected
        self._cancel=Gio.Cancellable()
        self._closed=False
        self._loading=True
        self._error=''
        self._store=Gio.ListStore.new(ContactItem)
        self._search=Gtk.SearchEntry(placeholder_text='Buscar por nombre o número',hexpand=True)
        self._search.set_text(query)
        self._search.connect('search-changed',self._search_changed)
        self._status=Gtk.Label(label='Cargando contactos…',wrap=True,xalign=0,css_classes=['dim-label'])
        self._filter=Gtk.CustomFilter.new(lambda item,*_args:search_text(self._search.get_text().strip()) in item.search)
        self._filtered=Gtk.FilterListModel(model=self._store,filter=self._filter)
        factory=Gtk.SignalListItemFactory()
        factory.connect('setup',self._setup_item)
        factory.connect('bind',self._bind_item)
        self._list=Gtk.ListView(model=Gtk.NoSelection(model=self._filtered),factory=factory,show_separators=True)
        self._list.connect('activate',lambda _list,pos:self._choose(self._filtered.get_item(pos)))
        scroll=Gtk.ScrolledWindow(vexpand=True,hscrollbar_policy=Gtk.PolicyType.NEVER,child=self._list)
        content=Gtk.Box(orientation=Gtk.Orientation.VERTICAL,spacing=12,margin_top=12,margin_bottom=12,margin_start=12,margin_end=12)
        for child in (self._search,self._status,scroll):content.append(child)
        toolbar=Adw.ToolbarView(content=content);toolbar.add_top_bar(Adw.HeaderBar())
        self.set_child(toolbar)
        self.connect('closed',self._on_closed)
        self._timeout=GLib.timeout_add_seconds(30,self._cancel_load)
        threading.Thread(target=self._load,daemon=True).start()

    def _load(self):
        error=''
        try:contacts=read_gnome_contacts(self._cancel)
        except Exception:
            contacts=[];error='No se han podido cargar los contactos de GNOME.'
        entries={}
        for contact in contacts:
            entries.setdefault((search_text(contact.name),normalize_phone(contact.number)),contact)
        if self._cancel.is_cancelled():error='La consulta ha tardado demasiado. Vuelve a abrir los contactos para reintentar.'
        ordered=sorted(entries.values(),key=lambda c:(search_text(c.name),c.number))
        GLib.idle_add(self._loaded,ordered,error)

    def _loaded(self,contacts,error):
        if self._timeout:GLib.source_remove(self._timeout);self._timeout=0
        if self._closed:return GLib.SOURCE_REMOVE
        self._loading=False;self._error=error
        self._store.splice(0,self._store.get_n_items(),[ContactItem(c) for c in contacts])
        self._search_changed()
        return GLib.SOURCE_REMOVE

    def _search_changed(self,*_args):
        self._filter.changed(Gtk.FilterChange.DIFFERENT)
        if self._loading:return
        count=self._filtered.get_n_items()
        message=self._error or ('Selecciona el teléfono del contacto al que quieres llamar.' if count else 'No hay contactos con teléfono que coincidan con la búsqueda.')
        self._status.set_text(message)

    def _setup_item(self,_factory,item):
        box=Gtk.Box(spacing=12,margin_top=10,margin_bottom=10,margin_start=8,margin_end=8)
        labels=Gtk.Box(orientation=Gtk.Orientation.VERTICAL,spacing=3,hexpand=True)
        name=Gtk.Label(xalign=0,ellipsize=Pango.EllipsizeMode.END)
        phone=Gtk.Label(xalign=0,css_classes=['dim-label'])
        labels.append(name);labels.append(phone)
        call=Gtk.Button(icon_name='call-start-symbolic',valign=Gtk.Align.CENTER)
        call.connect('clicked',lambda _button:self._choose(item.get_item()))
        box.append(labels);box.append(call);item.set_child(box)
        item._name=name;item._phone=phone;item._call=call

    def _bind_item(self,_factory,item):
        contact=item.get_item().contact
        item._name.set_text(contact.name)
        item._phone.set_text(' · '.join(part for part in (contact.label,contact.number) if part))
        item._call.set_tooltip_text('Llamar a '+contact.name)

    def _choose(self,item):
        if item is None:return
        contact=item.contact
        self.close()
        self._select(contact.number,contact.name)

    def _cancel_load(self):
        self._timeout=0;self._cancel.cancel()
        return GLib.SOURCE_REMOVE

    def _on_closed(self,*_args):
        self._closed=True;self._cancel.cancel()
        if self._timeout:GLib.source_remove(self._timeout);self._timeout=0
