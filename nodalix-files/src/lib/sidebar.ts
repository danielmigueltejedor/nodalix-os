import { invoke } from "@tauri-apps/api/core";

export interface SidebarItem {
  id: string;
  label: string;
  path: string;
  kind: string;
  pinned: boolean;
  custom: boolean;
  can_rename: boolean;
  can_remove: boolean;
}

export interface SidebarConfig {
  hidden_builtin: string[];
  pins: { id: string; label: string; path: string }[];
  renamed: Record<string, string>;
}

export const getSidebarItems = () => invoke<SidebarItem[]>("get_sidebar_items");
export const pinSidebarPath = (path: string, label?: string) =>
  invoke<void>("pin_sidebar_path", { path, label: label ?? null });
export const unpinSidebarPath = (path: string) =>
  invoke<void>("unpin_sidebar_path", { path });
export const hideBuiltinSidebar = (id: string) =>
  invoke<void>("hide_builtin_sidebar", { id });
export const showBuiltinSidebar = (id: string) =>
  invoke<void>("show_builtin_sidebar", { id });
export const renameSidebarAccess = (id: string, label: string) =>
  invoke<void>("rename_sidebar_access", { id, label });
export const removeSidebarAccess = (id: string) =>
  invoke<void>("remove_sidebar_access", { id });
