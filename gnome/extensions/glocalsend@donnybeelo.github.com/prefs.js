import Gio from "gi://Gio";
import Adw from "gi://Adw";
import { ExtensionPreferences } from "resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js";
import { DEFAULT_PORT, KEY_ALIAS, KEY_AUTO_ACCEPT, KEY_DOWNLOAD_FOLDER, KEY_FINGERPRINT, KEY_PORT, ensureAlias, getDefaultDownloadFolder, } from "./common.js";
const DEFAULT_AUTO_DISABLE_MINUTES = 10;
const DESCRIPTION = "Configura el uso compartido de LocalSend, las transferencias entrantes y la identidad del dispositivo anunciada en tu red local.";
export default class LocalSendCompanionPreferences extends ExtensionPreferences {
    async fillPreferencesWindow(window) {
        const settings = this.getSettings();
        if (settings.get_string(KEY_ALIAS).trim().length === 0)
            settings.set_string(KEY_ALIAS, ensureAlias(""));
        if (settings.get_string(KEY_FINGERPRINT).trim().length === 0)
            settings.set_string(KEY_FINGERPRINT, "");
        if (settings.get_string(KEY_DOWNLOAD_FOLDER).trim().length === 0)
            settings.set_string(KEY_DOWNLOAD_FOLDER, getDefaultDownloadFolder());
        if (settings.get_int(KEY_PORT) <= 0)
            settings.set_int(KEY_PORT, DEFAULT_PORT);
        if (settings.get_int("auto-disable-minutes") <= 0)
            settings.set_int("auto-disable-minutes", DEFAULT_AUTO_DISABLE_MINUTES);
        const page = new Adw.PreferencesPage({
            title: "LocalSend",
        });
        const identityGroup = new Adw.PreferencesGroup({
            title: "Identidad",
            description: DESCRIPTION,
        });
        const aliasRow = new Adw.EntryRow({
            title: "Nombre del dispositivo",
        });
        settings.bind(KEY_ALIAS, aliasRow, "text", Gio.SettingsBindFlags.DEFAULT);
        const portRow = Adw.SpinRow.new_with_range(1, 65535, 1);
        portRow.title = "Puerto";
        portRow.subtitle = "Puerto usado para descubrimiento y transferencias en la red local";
        portRow.value = settings.get_int(KEY_PORT);
        settings.bind(KEY_PORT, portRow, "value", Gio.SettingsBindFlags.DEFAULT);
        identityGroup.add(aliasRow);
        identityGroup.add(portRow);
        const transferGroup = new Adw.PreferencesGroup({
            title: "Recepción",
            description: "Las transferencias entrantes se guardan localmente y se aprueban desde la interfaz de GNOME.",
        });
        const behaviorGroup = new Adw.PreferencesGroup({
            title: "Comportamiento",
            description: "Controla cuánto tiempo permanece activo LocalSend después de activarlo.",
        });
        const folderRow = new Adw.EntryRow({
            title: "Carpeta de descargas",
        });
        folderRow.text =
            settings.get_string(KEY_DOWNLOAD_FOLDER) || getDefaultDownloadFolder();
        settings.bind(KEY_DOWNLOAD_FOLDER, folderRow, "text", Gio.SettingsBindFlags.DEFAULT);
        const autoAcceptRow = new Adw.SwitchRow({
            title: "Aceptar automáticamente las transferencias entrantes",
            subtitle: "Aceptar solicitudes sin pedir confirmación",
        });
        settings.bind(KEY_AUTO_ACCEPT, autoAcceptRow, "active", Gio.SettingsBindFlags.DEFAULT);
        transferGroup.add(folderRow);
        transferGroup.add(autoAcceptRow);
        const startOnLoginRow = new Adw.SwitchRow({
            title: "Iniciar LocalSend al iniciar sesión",
            subtitle: "Activa LocalSend automáticamente al entrar en GNOME",
        });

        settings.bind(
            "start-on-login",
            startOnLoginRow,
            "active",
            Gio.SettingsBindFlags.DEFAULT
        );

        behaviorGroup.add(startOnLoginRow);

        const autoDisableRow = new Adw.SwitchRow({
            title: "Desactivar LocalSend automáticamente",
            subtitle: "Desactiva LocalSend automáticamente después de un tiempo",
        });
        const autoDisableMinutesRow = Adw.SpinRow.new_with_range(1, 1440, 1);
        autoDisableMinutesRow.title = "Tiempo hasta desactivación automática";
        autoDisableMinutesRow.subtitle =
            "Minutos antes de que LocalSend se desactive automáticamente";
        autoDisableMinutesRow.value =
            settings.get_int("auto-disable-minutes") || DEFAULT_AUTO_DISABLE_MINUTES;
        settings.bind("auto-disable-enabled", autoDisableRow, "active", Gio.SettingsBindFlags.DEFAULT);
        settings.bind("auto-disable-minutes", autoDisableMinutesRow, "value", Gio.SettingsBindFlags.DEFAULT);
        behaviorGroup.add(autoDisableRow);
        behaviorGroup.add(autoDisableMinutesRow);
        page.add(identityGroup);
        page.add(transferGroup);
        page.add(behaviorGroup);
        window.add(page);
    }
}
