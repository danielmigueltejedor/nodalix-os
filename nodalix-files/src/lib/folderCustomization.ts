import { invoke } from "@tauri-apps/api/core";

export interface FolderStyle {
  color?: string | null;
  icon?: string | null;
}

export const FOLDER_COLORS = [
  "#cba6f7",
  "#89b4fa",
  "#a6e3a1",
  "#f9e2af",
  "#fab387",
  "#f38ba8",
  "#94e2d5",
  "#b4befe",
] as const;

export const FOLDER_ICONS = [
  "default",
  "star",
  "work",
  "code",
  "download",
  "photo",
  "music",
] as const;

export const getFolderCustomizations = () =>
  invoke<Record<string, FolderStyle>>("get_folder_customizations");

export const setFolderColor = (path: string, color: string | null) =>
  invoke<void>("set_folder_color", { path, color });

export const setFolderIcon = (path: string, icon: string | null) =>
  invoke<void>("set_folder_icon", { path, icon });
