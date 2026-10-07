import GLib from "gi://GLib";
export const PROTOCOL_VERSION = "2.1";
export const DEFAULT_PORT = 53317;
export const DEFAULT_MULTICAST_GROUP = "224.0.0.167";
export const DEFAULT_DOWNLOAD_FOLDER = "";
export const DEFAULT_AUTO_DISABLE_ENABLED = true;
export const DEFAULT_AUTO_DISABLE_MINUTES = 10;
export const KEY_ALIAS = "alias";
export const KEY_FINGERPRINT = "fingerprint";
export const KEY_PORT = "port";
export const KEY_DOWNLOAD_FOLDER = "download-folder";
export const KEY_AUTO_ACCEPT = "auto-accept";
export const KEY_AUTO_DISABLE_ENABLED = "auto-disable-enabled";
export const KEY_AUTO_DISABLE_MINUTES = "auto-disable-minutes";
export const CERT_DIR = GLib.build_filenamev([
    GLib.get_user_data_dir(),
    "glocalsend",
]);
export const CERT_PATH = GLib.build_filenamev([CERT_DIR, "cert.pem"]);
export const KEY_PATH = GLib.build_filenamev([CERT_DIR, "key.pem"]);
export var ProtocolType;
(function (ProtocolType) {
    ProtocolType["Http"] = "http";
    ProtocolType["Https"] = "https";
})(ProtocolType || (ProtocolType = {}));
export var DeviceType;
(function (DeviceType) {
    DeviceType["Mobile"] = "mobile";
    DeviceType["Desktop"] = "desktop";
    DeviceType["Web"] = "web";
    DeviceType["Headless"] = "headless";
    DeviceType["Server"] = "server";
})(DeviceType || (DeviceType = {}));
export function makeDefaultAlias() {
    return GLib.get_host_name() || "LocalSend";
}
export function ensureAlias(value) {
    const trimmed = value.trim();
    return trimmed.length > 0 ? trimmed : makeDefaultAlias();
}
export function sanitizeFileName(fileName) {
    const cleaned = fileName.replace(/[\\/\\0]/g, "-").trim();
    return cleaned.length > 0 ? cleaned : "localsend-file";
}
export function formatBytes(bytes) {
    if (!Number.isFinite(bytes) || bytes <= 0)
        return "0 B";
    const units = ["B", "KB", "MB", "GB", "TB"];
    let value = bytes;
    let unitIndex = 0;
    while (value >= 1024 && unitIndex < units.length - 1) {
        value /= 1024;
        unitIndex++;
    }
    return unitIndex === 0
        ? `${Math.round(value)} ${units[unitIndex]}`
        : `${value.toFixed(1)} ${units[unitIndex]}`;
}
export function encodeJson(value) {
    return GLib.Bytes.new(new TextEncoder().encode(JSON.stringify(value)));
}
export function decodeJson(bytes) {
    if (bytes === null || bytes === undefined)
        throw new Error("Empty response body.");
    return JSON.parse(new TextDecoder().decode(bytes));
}
export function stringFromBytes(bytes) {
    if (bytes === null || bytes === undefined)
        return "";
    return new TextDecoder().decode(bytes);
}
export function getDefaultDownloadFolder() {
    return GLib.build_filenamev([GLib.get_home_dir(), "Downloads", "LocalSend"]);
}
