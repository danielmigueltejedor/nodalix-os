// Native Phone Link Quick Settings, formerly nodalix-connect@getnodalia.com.
// It is compiled into GNOME Shell's own JavaScript resources.
import Gio from 'gi://Gio';
import GioUnix from 'gi://GioUnix';
import GObject from 'gi://GObject';
import * as QuickSettings from 'resource:///org/gnome/shell/ui/quickSettings.js';

const PhoneToggle = GObject.registerClass(
    {GTypeName: 'NodalixShellPhoneToggle'},
    class PhoneToggle extends QuickSettings.QuickToggle {
        constructor(notifyError) {
            super({title: 'Enlace móvil', iconName: 'phone-symbolic', toggleMode: false});
            this.connect('clicked', () => {
                const app = GioUnix.DesktopAppInfo.new('com.nodalix.PhoneLink.desktop');
                if (app)
                    app.launch([], global.create_app_launch_context(0, -1));
                else
                    notifyError('Enlace móvil', 'No se encuentra la aplicación de Nodalix');
            });
        }
    });

export class NodalixConnect {
    constructor(quickSettings, notifyError) {
        this._indicator = new QuickSettings.SystemIndicator();
        this._toggle = new PhoneToggle(notifyError);
        this._indicator.quickSettingsItems.push(this._toggle);
        quickSettings.addExternalIndicator(this._indicator);

        // The shell only reflects the service state; Bluetooth stays in Phone Link.
        this._watchId = Gio.bus_watch_name(
            Gio.BusType.SESSION,
            'com.gabriel.iphonebridge',
            Gio.BusNameWatcherFlags.NONE,
            () => {
                if (this._toggle)
                    this._toggle.subtitle = 'Servicio activo';
            },
            () => {
                if (this._toggle)
                    this._toggle.subtitle = 'Servicio desconectado';
            });
    }

    destroy() {
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
