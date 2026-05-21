import { invoke } from "@tauri-apps/api/core";

export interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  modified: number | null;
  // Linux filesystems do not expose birth/creation time reliably. The backend
  // provides created when available; sorting falls back to modified time.
  created: number | null;
  icon_kind?: FileKind | null;
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

export interface SpecialDirs {
  home: string;
  desktop: string | null;
  downloads: string | null;
  documents: string | null;
  pictures: string | null;
  videos: string | null;
  music: string | null;
  data: string | null;
  trash: string | null;
}

export interface StartupBundle {
  home: string;
  initial_path: string | null;
  platform: import("./platform").PlatformInfo;
  sidebar_items: import("./sidebar").SidebarItem[];
  localsend_available: boolean;
  folder_customizations: Record<
    string,
    import("./folderCustomization").FolderStyle
  >;
  special_dirs: SpecialDirs;
}

export type FileKind =
  | "folder"
  | "folder-home"
  | "folder-desktop"
  | "folder-downloads"
  | "folder-documents"
  | "folder-pictures"
  | "folder-videos"
  | "folder-music"
  | "trash"
  | "disk"
  | "network"
  | "image"
  | "image-jpg"
  | "image-png"
  | "image-svg"
  | "video"
  | "document"
  | "document-text"
  | "document-pdf"
  | "document-markdown"
  | "document-spreadsheet"
  | "music"
  | "audio-mp3"
  | "archive"
  | "code-python"
  | "code-c"
  | "code-js"
  | "code-rust"
  | "code-shell"
  | "code-generic"
  | "generic";

const IMAGE_EXT = new Set([
  "png",
  "jpg",
  "jpeg",
  "gif",
  "webp",
  "svg",
  "bmp",
  "ico",
  "avif",
]);
const VIDEO_EXT = new Set(["mp4", "mkv", "webm", "avi", "mov", "m4v"]);
const DOC_EXT = new Set([
  "pdf",
  "doc",
  "docx",
  "odt",
  "txt",
  "md",
  "rtf",
  "xls",
  "xlsx",
  "csv",
]);
const MUSIC_EXT = new Set(["mp3", "flac", "wav", "ogg", "m4a", "aac", "opus"]);
const ARCHIVE_EXT = new Set([
  "zip",
  "tar",
  "gz",
  "bz2",
  "xz",
  "7z",
  "rar",
  "zst",
]);

export function normalizePath(path: string): string {
  if (path === "/") return path;
  return path.replace(/\/+$/, "");
}

export function getTrashPath(specialDirs?: SpecialDirs | null): string | null {
  if (specialDirs?.trash) return normalizePath(specialDirs.trash);
  return specialDirs?.home
    ? normalizePath(`${specialDirs.home}/.local/share/Trash/files`)
    : null;
}

export function isTrashPath(
  path: string,
  specialDirs?: SpecialDirs | null,
): boolean {
  const trash = getTrashPath(specialDirs);
  if (!trash) return false;
  const normalized = normalizePath(path);
  return normalized === trash || normalized.startsWith(`${trash}/`);
}

export function getSpecialFolderKind(
  path: string,
  specialDirs?: SpecialDirs | null,
): FileKind | null {
  if (!specialDirs) return null;
  const normalized = normalizePath(path);
  const pairs: Array<[keyof SpecialDirs, FileKind]> = [
    ["home", "folder-home"],
    ["desktop", "folder-desktop"],
    ["downloads", "folder-downloads"],
    ["documents", "folder-documents"],
    ["pictures", "folder-pictures"],
    ["videos", "folder-videos"],
    ["music", "folder-music"],
    ["trash", "trash"],
  ];
  for (const [key, kind] of pairs) {
    const value = specialDirs[key];
    if (value && normalizePath(value) === normalized) return kind;
  }
  return null;
}

export function getFileKind(
  entry: FileEntry,
  specialDirs?: SpecialDirs | null,
): FileKind {
  if (entry.icon_kind) return entry.icon_kind;
  if (entry.is_dir)
    return getSpecialFolderKind(entry.path, specialDirs) ?? "folder";
  const ext = entry.name.split(".").pop()?.toLowerCase() ?? "";
  if (ext === "jpg" || ext === "jpeg") return "image-jpg";
  if (ext === "png") return "image-png";
  if (ext === "svg") return "image-svg";
  if (ext === "mp3") return "audio-mp3";
  if (ext === "txt" || ext === "rtf") return "document-text";
  if (ext === "pdf") return "document-pdf";
  if (ext === "md" || ext === "markdown") return "document-markdown";
  if (ext === "xls" || ext === "xlsx" || ext === "csv") {
    return "document-spreadsheet";
  }
  if (ext === "py" || ext === "pyw") return "code-python";
  if (ext === "c" || ext === "h" || ext === "cpp" || ext === "hpp") {
    return "code-c";
  }
  if (["js", "jsx", "ts", "tsx", "json"].includes(ext)) return "code-js";
  if (ext === "rs") return "code-rust";
  if (["sh", "bash", "zsh", "fish"].includes(ext)) return "code-shell";
  if (
    ["css", "html", "xml", "go", "java", "kt", "swift", "php", "rb"].includes(
      ext,
    )
  ) {
    return "code-generic";
  }
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

export function isHiddenPath(pathOrName: string): boolean {
  const name = basename(pathOrName);
  return name.startsWith(".") && name !== "." && name !== "..";
}

export function isHiddenEntry(
  entry: Pick<FileEntry, "name" | "path">,
): boolean {
  return isHiddenPath(entry.name || entry.path);
}

export const startupBundle = () => invoke<StartupBundle>("startup_bundle");
export const quitApp = () => invoke<void>("quit_app");
export const getSpecialDirs = () => invoke<SpecialDirs>("get_special_dirs");
export const listDirectory = (path: string) =>
  invoke<FileEntry[]>("list_directory", { path });
export const listDisks = () => invoke<FileEntry[]>("list_disks");
export const listNetworkLocations = () =>
  invoke<FileEntry[]>("list_network_locations");
export const copyTextToClipboard = (text: string) =>
  invoke<void>("copy_text_to_clipboard", { text });
export const connectNetworkLocation = (options: {
  protocol: string;
  host: string;
  share: string;
  username?: string;
  password?: string;
}) => invoke<void>("connect_network_location", options);
export const openPath = (path: string) => invoke<void>("open_path", { path });
export const openWith = (path: string, appId: string) =>
  invoke<void>("open_with", { path, appId });
export const getOpenWithApps = (path: string) =>
  invoke<OpenWithApp[]>("get_open_with_apps", { path });
export const createFolder = (parent: string, name: string) =>
  invoke<string>("create_folder", { parent, name });
export const createDocument = (parent: string, name: string) =>
  invoke<string>("create_document", { parent, name });
export const renamePath = (oldPath: string, newName: string) =>
  invoke<string>("rename_path", { oldPath, newName });
export const trashPaths = (paths: string[]) =>
  invoke<void>("trash_paths", { paths });
export const deletePathsPermanently = (paths: string[]) =>
  invoke<void>("delete_paths_permanently", { paths });
export const copyPaths = (paths: string[]) =>
  invoke<void>("copy_paths", { paths });
export const cutPaths = (paths: string[]) =>
  invoke<void>("cut_paths", { paths });
export const pasteInto = (targetDir: string) =>
  invoke<void>("paste_into", { targetDir });
export const clipboardHasContent = () =>
  invoke<boolean>("clipboard_has_content");
export const moveTo = (paths: string[], targetDir: string) =>
  invoke<void>("move_to", { paths, targetDir });
export const getProperties = (path: string) =>
  invoke<PathProperties>("get_properties", { path });
export const openTerminalHere = (path: string) =>
  invoke<void>("open_terminal_here", { path });
