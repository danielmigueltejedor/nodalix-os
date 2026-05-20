import { invoke } from "@tauri-apps/api/core";

export interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  modified: number | null;
}

export interface PathProperties {
  name: string;
  path: string;
  kind: string;
  size: number;
  size_display: string;
  created: number | null;
  modified: number | null;
  permissions: string;
  is_symlink: boolean;
  item_count: number | null;
}

export interface OpenWithApp {
  id: string;
  name: string;
}

export interface StartupBundle {
  home: string;
  platform: import("./platform").PlatformInfo;
  sidebar_items: import("./sidebar").SidebarItem[];
  localsend_available: boolean;
  folder_customizations: Record<string, import("./folderCustomization").FolderStyle>;
}

export type FileKind =
  | "folder"
  | "image"
  | "video"
  | "document"
  | "music"
  | "archive"
  | "generic";

const IMAGE_EXT = new Set(["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "ico", "avif"]);
const VIDEO_EXT = new Set(["mp4", "mkv", "webm", "avi", "mov", "m4v"]);
const DOC_EXT = new Set(["pdf", "doc", "docx", "odt", "txt", "md", "rtf", "xls", "xlsx", "csv"]);
const MUSIC_EXT = new Set(["mp3", "flac", "wav", "ogg", "m4a", "aac", "opus"]);
const ARCHIVE_EXT = new Set(["zip", "tar", "gz", "bz2", "xz", "7z", "rar", "zst"]);

export function getFileKind(entry: FileEntry): FileKind {
  if (entry.is_dir) return "folder";
  const ext = entry.name.split(".").pop()?.toLowerCase() ?? "";
  if (IMAGE_EXT.has(ext)) return "image";
  if (VIDEO_EXT.has(ext)) return "video";
  if (DOC_EXT.has(ext)) return "document";
  if (MUSIC_EXT.has(ext)) return "music";
  if (ARCHIVE_EXT.has(ext)) return "archive";
  return "generic";
}

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
}

export function formatTimestamp(secs: number | null): string {
  if (!secs) return "—";
  return new Date(secs * 1000).toLocaleString();
}

export function parentPath(path: string): string | null {
  const normalized = path.replace(/\/+$/, "");
  const idx = normalized.lastIndexOf("/");
  if (idx <= 0) return normalized === "/" ? null : "/";
  return normalized.slice(0, idx) || "/";
}

export function basename(path: string): string {
  const normalized = path.replace(/\/+$/, "");
  const idx = normalized.lastIndexOf("/");
  return idx === -1 ? normalized : normalized.slice(idx + 1);
}

export const startupBundle = () => invoke<StartupBundle>("startup_bundle");
export const listDirectory = (path: string) =>
  invoke<FileEntry[]>("list_directory", { path });
export const openPath = (path: string) => invoke<void>("open_path", { path });
export const openWith = (path: string, appId: string) =>
  invoke<void>("open_with", { path, appId });
export const getOpenWithApps = (path: string) =>
  invoke<OpenWithApp[]>("get_open_with_apps", { path });
export const createFolder = (parent: string, name: string) =>
  invoke<string>("create_folder", { parent, name });
export const renamePath = (oldPath: string, newName: string) =>
  invoke<string>("rename_path", { oldPath, newName });
export const trashPaths = (paths: string[]) =>
  invoke<void>("trash_paths", { paths });
export const copyPaths = (paths: string[]) => invoke<void>("copy_paths", { paths });
export const cutPaths = (paths: string[]) => invoke<void>("cut_paths", { paths });
export const pasteInto = (targetDir: string) =>
  invoke<void>("paste_into", { targetDir });
export const clipboardHasContent = () => invoke<boolean>("clipboard_has_content");
export const moveTo = (paths: string[], targetDir: string) =>
  invoke<void>("move_to", { paths, targetDir });
export const getProperties = (path: string) =>
  invoke<PathProperties>("get_properties", { path });
export const openTerminalHere = (path: string) =>
  invoke<void>("open_terminal_here", { path });
