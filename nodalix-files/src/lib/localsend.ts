import { invoke } from "@tauri-apps/api/core";

export interface LocalSendInfo {
  available: boolean;
  command: string | null;
}

export interface LocalSendDevice {
  id: string;
  alias: string;
  device_model: string | null;
  device_type: string | null;
  fingerprint: string | null;
  ip: string;
  port: number;
  protocol: string;
}

export const getLocalSendInfo = () => invoke<LocalSendInfo>("get_localsend_info");
export const discoverLocalSendDevices = () =>
  invoke<LocalSendDevice[]>("discover_localsend_devices");
export const sendToLocalSendDevice = (
  paths: string[],
  device: LocalSendDevice,
) => invoke<void>("send_to_localsend_device", { paths, device });
export const sendWithLocalSend = (paths: string[]) =>
  invoke<void>("send_with_localsend", { paths });
