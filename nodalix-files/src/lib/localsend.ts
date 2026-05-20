import { invoke } from "@tauri-apps/api/core";

export interface LocalSendInfo {
  available: boolean;
  command: string | null;
}

export const getLocalSendInfo = () => invoke<LocalSendInfo>("get_localsend_info");
export const sendWithLocalSend = (paths: string[]) =>
  invoke<void>("send_with_localsend", { paths });
