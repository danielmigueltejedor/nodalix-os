import { useCallback, useState } from "react";
import type { SidebarItem } from "../lib/sidebar";
import {
  getSidebarItems,
  hideBuiltinSidebar,
  pinSidebarPath,
  removeSidebarAccess,
  renameSidebarAccess,
  setSidebarIcon,
  showBuiltinSidebar,
  unpinSidebarPath,
} from "../lib/sidebar";

export function useSidebarItems(initial: SidebarItem[] = []) {
  const [items, setItems] = useState<SidebarItem[]>(initial);

  const refresh = useCallback(async () => {
    const next = await getSidebarItems();
    setItems(next);
    return next;
  }, []);

  const pin = useCallback(
    async (path: string, label?: string) => {
      await pinSidebarPath(path, label);
      return refresh();
    },
    [refresh],
  );

  const unpin = useCallback(
    async (path: string) => {
      await unpinSidebarPath(path);
      return refresh();
    },
    [refresh],
  );

  const hideBuiltin = useCallback(
    async (id: string) => {
      await hideBuiltinSidebar(id);
      return refresh();
    },
    [refresh],
  );

  const showBuiltin = useCallback(
    async (id: string) => {
      await showBuiltinSidebar(id);
      return refresh();
    },
    [refresh],
  );

  const renameAccess = useCallback(
    async (id: string, label: string) => {
      await renameSidebarAccess(id, label);
      return refresh();
    },
    [refresh],
  );

  const removeAccess = useCallback(
    async (id: string) => {
      await removeSidebarAccess(id);
      return refresh();
    },
    [refresh],
  );

  const setIcon = useCallback(
    async (id: string, icon: string | null) => {
      await setSidebarIcon(id, icon);
      return refresh();
    },
    [refresh],
  );

  return {
    items,
    setItems,
    refresh,
    pin,
    unpin,
    hideBuiltin,
    showBuiltin,
    renameAccess,
    setIcon,
    removeAccess,
  };
}
