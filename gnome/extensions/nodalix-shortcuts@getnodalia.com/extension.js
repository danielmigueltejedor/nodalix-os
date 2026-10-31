import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import Gio from 'gi://Gio';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as Screenshot from 'resource:///org/gnome/shell/ui/screenshot.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

export function isTerminal(app, window) {
    const categories=app?.get_app_info()?.get_categories()?.split(';') ?? [];
    const id=`${app?.get_id() ?? ''} ${window?.get_wm_class() ?? ''}`;
    return categories.includes('TerminalEmulator') || /ghostty|kgx|gnome-terminal|ptyxis|konsole|alacritty|kitty|wezterm|foot/i.test(id);
}

export default class NodalixShortcuts extends Extension {
    enable() {
        this._settings=this.getSettings();
        this._pending=new Set();
        this._cancellable=new Gio.Cancellable();
        this._keys=[];
        this._tracker=Shell.WindowTracker.get_default();
        this._keyboard=global.stage.get_context().get_backend().get_default_seat().create_virtual_device(Clutter.InputDeviceType.KEYBOARD_DEVICE);
        this._changed=this._settings.connect('changed::enabled',()=>this._bind());
        this._bind();
        // Keep GNOME's own area picker, capture storage and permission boundary.
        this._screenshotSettings=new Gio.Settings({schema_id:'org.gnome.shell.keybindings'});
        Main.wm.removeKeybinding('show-screenshot-ui');
        Main.wm.addKeybinding('show-screenshot-ui',this._screenshotSettings,Meta.KeyBindingFlags.IGNORE_AUTOREPEAT,
            Shell.ActionMode.NORMAL|Shell.ActionMode.OVERVIEW,()=>{
            Main.screenshotUI.open().then(()=>{Main.screenshotUI._selectionButton.checked=true;}).catch(logError);
        });
    }

    _bind() {
        for(const key of this._keys) Main.wm.removeKeybinding(key);
        this._keys=[];
        if(!this._settings.get_boolean('enabled'))return;
        for(const [key,copy] of [['nodalix-copy',true],['nodalix-paste',false]]) {
            Main.wm.addKeybinding(key,this._settings,Meta.KeyBindingFlags.IGNORE_AUTOREPEAT,
                Shell.ActionMode.NORMAL,()=>this._edit(copy));
            this._keys.push(key);
        }
    }

    _edit(copy) {
        const target=global.display.focus_window;
        if(!target)return;
        const terminal=isTerminal(this._tracker.get_window_app(target),target);
        const ghostty=/ghostty/i.test(`${this._tracker.get_window_app(target)?.get_id() ?? ''} ${target.get_wm_class() ?? ''}`);
        const deadline=GLib.get_monotonic_time()+3000000;
        const id=GLib.timeout_add(GLib.PRIORITY_DEFAULT,16,()=>{
            if(global.display.focus_window!==target || GLib.get_monotonic_time()>deadline) {
                this._pending.delete(id);return GLib.SOURCE_REMOVE;
            }
            // Physical modifiers must be released before synthesizing Ctrl/Ctrl+Shift.
            if(global.get_pointer()[2] & (Clutter.ModifierType.SUPER_MASK|Clutter.ModifierType.CONTROL_MASK|Clutter.ModifierType.SHIFT_MASK|Clutter.ModifierType.MOD1_MASK))
                return GLib.SOURCE_CONTINUE;
            if (ghostty) {
                this._ghosttyClipboard(copy,target);
                this._pending.delete(id);return GLib.SOURCE_REMOVE;
            }
            const modifiers=[Clutter.KEY_Control_L,...(terminal?[Clutter.KEY_Shift_L]:[])];
            let time=GLib.get_monotonic_time();
            for(const key of modifiers)this._keyboard.notify_keyval(time+=10,key,Clutter.KeyState.PRESSED);
            const key=copy?Clutter.KEY_c:Clutter.KEY_v;
            this._keyboard.notify_keyval(time+=10,key,Clutter.KeyState.PRESSED);
            this._keyboard.notify_keyval(time+=10,key,Clutter.KeyState.RELEASED);
            for(const key of modifiers.reverse())this._keyboard.notify_keyval(time+=10,key,Clutter.KeyState.RELEASED);
            this._pending.delete(id);return GLib.SOURCE_REMOVE;
        });
        this._pending.add(id);
    }

    _ghosttyClipboard(copy,target) {
        const name=target.get_gtk_unique_bus_name();
        const path=target.get_gtk_window_object_path();
        if(!name || !path)return;
        const cancel=this._cancellable;
        Gio.DBus.session.call(name,path,'org.gtk.Actions','Activate',
            new GLib.Variant('(sava{sv})',[copy?'copy':'paste',[],{}]),null,
            Gio.DBusCallFlags.NONE,1500,cancel,(connection,result)=>{
                try {connection.call_finish(result);}
                catch(error) {if(!cancel.is_cancelled())Main.notifyError('Atajos de Nodalix','No se pudo acceder al portapapeles de Ghostty.');}
            });
    }

    disable() {
        for(const key of this._keys ?? [])Main.wm.removeKeybinding(key);
        for(const id of this._pending ?? [])GLib.source_remove(id);
        this._pending?.clear();
        this._cancellable?.cancel();
        if(this._changed)this._settings.disconnect(this._changed);
        Main.wm.removeKeybinding('show-screenshot-ui');
        Main.wm.addKeybinding('show-screenshot-ui',this._screenshotSettings,Meta.KeyBindingFlags.IGNORE_AUTOREPEAT,
            Shell.ActionMode.ALL & ~Shell.ActionMode.LOGIN_SCREEN,Screenshot.showScreenshotUI);
        this._keyboard=this._settings=this._tracker=null;
        this._keys=[];
    }
}
