import { invoke } from "@tauri-apps/api/core";

export interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  modified: number | null;
}

export interface SpecialDirs {
  home: string;
  desktop: string | null;
  downloads: string | null;
  documents: string | null;
  pictures: string | null;
  videos: string | null;
  music: string | null;
  data: string | null;
}

export type FileKind =
  | "folder"
  | "image"
  | "video"
  | "document"
  | "music"
  | "archive"
  | "generic";

const IMAGE_EXT = new Set([
  "png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "ico", "avif", "heic",
]);
const VIDEO_EXT = new Set([
  "mp4", "mkv", "webm", "avi", "mov", "m4v", "wmv", "flv",
]);
const DOC_EXT = new Set([
  "pdf", "doc", "docx", "odt", "txt", "md", "rtf", "xls", "xlsx", "ods",
  "ppt", "pptx", "odp", "csv",
]);
const MUSIC_EXT = new Set([
  "mp3", "flac", "wav", "ogg", "m4a", "aac", "opus", "wma",
]);
const ARCHIVE_EXT = new Set([
  "zip", "tar", "gz", "bz2", "xz", "7z", "rar", "zst", "deb", "rpm",
]);

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

export async function listDirectory(path: string): Promise<FileEntry[]> {
  return invoke<FileEntry[]>("list_directory", { path });
}

export async function getSpecialDirs(): Promise<SpecialDirs> {
  return invoke<SpecialDirs>("get_special_dirs");
}

export async function openPath(path: string): Promise<void> {
  await invoke("open_path", { path });
}

export async function createFolder(parent: string, name: string): Promise<string> {
  return invoke<string>("create_folder", { parent, name });
}

export async function renamePath(oldPath: string, newName: string): Promise<string> {
  return invoke<string>("rename_path", { oldPath, newName });
}

export async function trashPath(path: string): Promise<void> {
  await invoke("trash_path", { path });
}
