import { invoke } from "@tauri-apps/api/core";

export interface WindowControls {
  show_minimize: boolean;
  show_maximize: boolean;
  show_close: boolean;
}

export interface PlatformInfo {
  is_hyprland: boolean;
  window_controls: WindowControls;
  debug: boolean;
}

export const getPlatformInfo = () => invoke<PlatformInfo>("get_platform_info");
