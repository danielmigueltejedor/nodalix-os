import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

let interface_settings = null;

export function enable_system_style() {
    interface_settings = new Gio.Settings({ schema_id: 'org.gnome.desktop.interface' });
}

export function disable_system_style() {
    interface_settings = null;
}

function get_background_style() {
    const theme = St.ThemeContext.get_for_stage(global.stage).get_theme();
    const stylesheet = theme.application_stylesheet ?? theme.default_stylesheet;
    const uri = stylesheet?.get_uri().toLowerCase() ?? '';
    if (uri.includes('yaru'))
        return uri.includes('dark') ? 2 : 1;

    const shell_style = Main.getStyleVariant();
    if (shell_style === 'light')
        return 1;

    if (shell_style === 'dark')
        return 2;

    return interface_settings.get_string('color-scheme') === 'prefer-dark' ? 2 : 1;
}

export function is_light_style() {
    return get_background_style() !== 2;
}

export function get_component_style(style, avail_styles = 3) {
    const style_num = Array.isArray(avail_styles) ? avail_styles.length : avail_styles;

    if (style >= 0 && style < style_num)
        return style;

    return get_background_style();
}

// One switch between the light and the dark style fires up to four signals (the setting, the
// shell's own copy of it, the theme reloading and the session mode), and every component of the
// extension listens to all of them. Redoing the blur each time inside the one frame that is
// already restyling the whole shell is what froze the screen. Instead each component waits for
// the signals to settle, then redraws once, and the components take turns a few frames apart.
const SETTLE_MS = 80;
const STAGGER_MS = 45;
const STAGGER_SLOTS = 6;
let next_slot = 0;

export function connect_system_style_changes(connections, callback) {
    const delay = SETTLE_MS + (next_slot++ % STAGGER_SLOTS) * STAGGER_MS;
    let pending = 0;
    const queue = () => {
        if (pending)
            return;
        pending = GLib.timeout_add(GLib.PRIORITY_LOW, delay, () => {
            pending = 0;
            callback();
            return GLib.SOURCE_REMOVE;
        });
    };
    connections.on_disconnect(() => {
        if (pending)
            GLib.source_remove(pending);
        pending = 0;
    });
    connections.connect(
        interface_settings,
        ['changed::color-scheme', 'changed::gtk-theme'],
        queue
    );
    connections.connect(St.Settings.get(), 'notify::color-scheme', queue);
    connections.connect(St.ThemeContext.get_for_stage(global.stage), 'changed', queue);
    connections.connect(Main.sessionMode, 'updated', queue);
}
