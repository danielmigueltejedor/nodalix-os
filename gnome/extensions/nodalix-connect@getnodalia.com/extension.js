import Gio from 'gi://Gio';
import GioUnix from 'gi://GioUnix';
import GObject from 'gi://GObject';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as QuickSettings from 'resource:///org/gnome/shell/ui/quickSettings.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const PhoneToggle = GObject.registerClass({GTypeName: 'NodalixConnectPhoneToggle'},
class PhoneToggle extends QuickSettings.QuickToggle {
    constructor() {
        super({title: 'Enlace móvil', iconName: 'phone-symbolic', toggleMode: false});
        this.connect('clicked', () => {
            const app = GioUnix.DesktopAppInfo.new('com.nodalix.PhoneLink.desktop');
            if (app)
                app.launch([], global.create_app_launch_context(0, -1));
            else
                Main.notifyError('Enlace móvil', 'No se encuentra la aplicación de Nodalix');
        });
    }
});

export default class NodalixConnectExtension extends Extension {
    enable() {
        this._indicator = new QuickSettings.SystemIndicator();
        this._toggle = new PhoneToggle();
        this._indicator.quickSettingsItems.push(this._toggle);
        const quickSettings = Main.panel.statusArea.quickSettings;
        quickSettings.addExternalIndicator(this._indicator);
        // Watch only the service owner. Pairing, notifications, calls and
        // Bluetooth work remain in the existing Nodalix backend.
        this._watchId = Gio.bus_watch_name(Gio.BusType.SESSION,
            'com.gabriel.iphonebridge', Gio.BusNameWatcherFlags.NONE,
            () => { this._toggle.subtitle = 'Servicio activo'; },
            () => { this._toggle.subtitle = 'Servicio desconectado'; });
    }

    disable() {
        if (this._watchId) {
            Gio.bus_unwatch_name(this._watchId);
            this._watchId = 0;
        }
        this._toggle?.destroy();
        this._toggle = null;
        this._indicator?.destroy();
        this._indicator = null;
    }
}
