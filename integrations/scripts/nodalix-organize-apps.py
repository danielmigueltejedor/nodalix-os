#!/usr/bin/env python3
"""Keep native GNOME app folders current without editing installed launchers."""
from pathlib import Path
import configparser
import json
import os

FOLDERS = {
    'nodalix-craft':'ArtCraft', 'engineering':'Ingeniería', 'internet':'Internet',
    'office':'Oficina', 'multimedia':'Multimedia', 'development':'Desarrollo',
    'games':'Juegos', 'science':'Ciencia y educación', 'settings':'Ajustes',
    'hardware':'Hardware', 'administration':'Administración',
    'utilities':'Utilidades', 'other':'Otros',
}
CRAFT_PUBLISHERS = ('ai.storyteller.', 'com.storytold.', 'io.tangencylabs.')
ENGINEERING_PATTERNS = ('matlab','simulink','octave','paraview','opencad','freecad',
    'openscad','gmsh','salome','ansys','comsol','openfoam','cadcraft','solidcraft')


def read_entry(path):
    parser=configparser.ConfigParser(interpolation=None,strict=False)
    parser.optionxform=str
    try:
        parser.read(path,encoding='utf-8')
        return dict(parser['Desktop Entry']) if parser.has_section('Desktop Entry') else None
    except (OSError,UnicodeError,configparser.Error):return None


def is_craft(desktop_id,entry):
    categories=set(filter(None,entry.get('Categories','').split(';')))
    identity=desktop_id.casefold()
    # Publisher namespaces and suite metadata cover new tools without a fixed
    # product-name list. An unrelated game containing "craft" stays a game.
    return bool(identity.startswith(CRAFT_PUBLISHERS)
            or entry.get('X-ArtCraft-App','').lower()=='true'
            or categories & {'ArtCraft','X-ArtCraft'})


def classify(desktop_id,entry):
    categories=set(filter(None,entry.get('Categories','').split(';')))
    searchable=' '.join((desktop_id,entry.get('Name',''),entry.get('Exec',''))).casefold()
    if any(word in searchable for word in ENGINEERING_PATTERNS) or categories & {'Engineering','CAD'}:
        return 'engineering'
    if desktop_id=='com.nodalix.PhoneLink.desktop' or entry.get('Name','').casefold() in {'enlace móvil','phone link','sesiones','sessions'}:
        return 'utilities'
    rules=[
        ('games',{'Game'}), ('development',{'Development','IDE'}),
        ('hardware',{'HardwareSettings','X-GNOME-HardwareSettings'}),
        ('settings',{'Settings'}),
        ('office',{'Office','Spreadsheet','WordProcessor','Presentation','Calendar'}),
        ('internet',{'Network','WebBrowser','Email','InstantMessaging','Chat','FileTransfer'}),
        ('science',{'Education','Science','Math','Physics','Chemistry','Astronomy','Biology'}),
        ('multimedia',{'AudioVideo','Audio','Video','Graphics','Photography','Player','Recorder'}),
        ('administration',{'System','PackageManager','Monitor'}),
        ('utilities',{'Utility','TextEditor','Accessibility'}),
    ]
    return next((folder for folder,values in rules if categories & values),'other')


def app_directories(home):
    data=Path(os.environ.get('XDG_DATA_HOME',str(home/'.local/share')))
    roots=[Path(value) for value in os.environ.get('XDG_DATA_DIRS','/usr/local/share:/usr/share').split(':') if value]
    # Lowest to highest precedence; user launchers can hide/override a package.
    ordered=[root/'applications' for root in reversed(roots)]
    system_flatpak=Path('/var/lib/flatpak/exports/share/applications')
    user_flatpak=home/'.local/share/flatpak/exports/share/applications'
    if system_flatpak not in ordered:ordered.insert(0,system_flatpak)
    if user_flatpak not in ordered:ordered.append(user_flatpak)
    return [folder for folder in ordered if folder!=data/'applications']+[data/'applications']


def collect(directories):
    from gi.repository import Gio,GLib
    result={}
    for folder in directories:
        if not folder.is_dir():continue
        for path in sorted(folder.rglob('*.desktop')):
            desktop_id='-'.join(path.relative_to(folder).parts)
            result[desktop_id]=path
    visible={}
    for desktop_id,path in result.items():
        entry=read_entry(path)
        if not entry or entry.get('Type','Application')!='Application' or any(entry.get(key,'false').lower()=='true' for key in ('Hidden','NoDisplay')):continue
        try:info=Gio.DesktopAppInfo.new_from_filename(str(path))
        except (TypeError,GLib.Error):continue
        if not info or not info.should_show():continue
        visible[desktop_id]=entry
    return visible


def organize(home=None,directories=None):
    from gi.repository import Gio,GLib
    home=Path(home or Path.home())
    entries=collect(directories or app_directories(home))
    settings=Gio.Settings.new('org.gnome.desktop.app-folders')
    current=list(settings.get_strv('folder-children'))
    managed=set(FOLDERS)|{'nodalix'}
    custom=[folder for folder in current if folder not in managed]
    folders={folder:Gio.Settings.new_with_path('org.gnome.desktop.app-folders.folder',
        '/org/gnome/desktop/app-folders/folders/'+folder+'/') for folder in current}
    # Ordinary apps already in a personal folder stay there. ArtCraft suite
    # membership is additional to an app's purpose and any personal folders.
    reserved=set()
    for folder in custom:
        prefs=folders[folder]
        excluded=set(prefs.get_strv('excluded-apps'))
        reserved.update(set(prefs.get_strv('apps'))-excluded)
        categories=set(prefs.get_strv('categories'))
        reserved.update(app for app,entry in entries.items()
            if app not in excluded and categories.intersection(entry.get('Categories','').split(';')))
    groups={folder:[] for folder in FOLDERS}
    for app,entry in entries.items():
        craft=is_craft(app,entry)
        if craft:groups['nodalix-craft'].append(app)
        if app not in reserved or craft:groups[classify(app,entry)].append(app)
    groups={key:sorted(set(apps)) for key,apps in groups.items() if apps}
    snapshot={folder:{key:prefs.get_value(key).unpack() for key in ('name','translate','apps','categories','excluded-apps')}
        for folder,prefs in folders.items()}
    state=Path(os.environ.get('XDG_STATE_HOME',str(home/'.local/state')))/'nodalix/app-folders'
    state.mkdir(parents=True,exist_ok=True,mode=0o700)
    backup=state/'before-040-multiple-folders.json'
    if not backup.exists():
        backup.write_text(json.dumps({'children':current,'folders':snapshot},ensure_ascii=False,indent=2)+'\n')
        backup.chmod(0o600)
    for folder,apps in groups.items():
        prefs=folders.get(folder) or Gio.Settings.new_with_path('org.gnome.desktop.app-folders.folder',
            '/org/gnome/desktop/app-folders/folders/'+folder+'/')
        values={'name':GLib.Variant('s',FOLDERS[folder]),'translate':GLib.Variant('b',False),
            'categories':GLib.Variant('as',[]),'apps':GLib.Variant('as',apps),
            'excluded-apps':GLib.Variant('as',[])}
        for key,value in values.items():
            if prefs.get_value(key)!=value:prefs.set_value(key,value)
    children=list(groups)+custom
    if children!=current:settings.set_strv('folder-children',children)
    Gio.Settings.sync()
    return groups


def main():
    groups=organize()
    for folder,apps in groups.items():print(f'{FOLDERS[folder]:20} {len(apps):3} apps')


if __name__=='__main__':main()
