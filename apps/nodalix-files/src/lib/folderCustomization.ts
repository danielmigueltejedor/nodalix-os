import { invoke } from "@tauri-apps/api/core";
import type { FileEntry, SpecialDirs } from "./files";
import { getSpecialFolderKind, normalizePath } from "./files";

export interface FolderStyle {
  color?: string | null;
  icon?: string | null;
}

const DEFAULT_FOLDER_COLOR = "#cba6f7";

const SPECIAL_FOLDER_COLORS = {
  "folder-home": "#cba6f7",
  "folder-desktop": "#89b4fa",
  "folder-downloads": "#74c7ec",
  "folder-documents": "#a6e3a1",
  "folder-pictures": "#89dceb",
  "folder-videos": "#f38ba8",
  "folder-music": "#fab387",
  trash: "#9399b2",
} as const;

function specialFolderColor(kind: string | null): string | null {
  if (!kind) return null;
  return kind in SPECIAL_FOLDER_COLORS
    ? SPECIAL_FOLDER_COLORS[kind as keyof typeof SPECIAL_FOLDER_COLORS]
    : null;
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
  "terminal",
  "projects",
  "icloud",
  "cloud",
  "server",
  "shared",
  "download",
  "photo",
  "camera",
  "music",
  "video",
  "documents",
  "notes",
  "books",
  "design",
  "archive",
  "backup",
  "locked",
  "shield",
  "private",
  "apps",
  "finance",
  "favorite",
  "data",
] as const;

export function getFolderStyle(
  entry: FileEntry,
  preferences: { defaultFolderColor?: string | null },
  customFolderAppearance: Record<string, FolderStyle>,
  specialDirs?: SpecialDirs | null,
): FolderStyle | undefined {
  if (!entry.is_dir) return undefined;

  const normalizedPath = normalizePath(entry.path);
  const custom =
    customFolderAppearance[entry.path] ?? customFolderAppearance[normalizedPath];
  const specialColor = specialFolderColor(
    getSpecialFolderKind(entry.path, specialDirs),
  );
  const fallbackColor =
    specialColor ?? preferences.defaultFolderColor ?? DEFAULT_FOLDER_COLOR;

  return {
    ...custom,
    color: custom?.color ?? fallbackColor,
  };
}

export const getFolderCustomizations = () =>
  invoke<Record<string, FolderStyle>>("get_folder_customizations");

export const setFolderColor = (path: string, color: string | null) =>
  invoke<void>("set_folder_color", { path, color });

export const setFolderIcon = (path: string, icon: string | null) =>
  invoke<void>("set_folder_icon", { path, icon });
