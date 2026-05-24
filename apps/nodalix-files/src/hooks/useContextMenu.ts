import { useCallback, useEffect, useState, type ReactNode } from "react";

export interface ContextMenuState {
  open: boolean;
  x: number;
  y: number;
  items: ReactNode;
}

const initial: ContextMenuState = {
  open: false,
  x: 0,
  y: 0,
  items: null,
};

export function useContextMenu() {
  const [menu, setMenu] = useState<ContextMenuState>(initial);

  const close = useCallback(() => setMenu(initial), []);

  const open = useCallback((x: number, y: number, items: ReactNode) => {
    setMenu({ open: true, x, y, items });
  }, []);

  useEffect(() => {
    if (!menu.open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [menu.open, close]);

  return { menu, open, close };
}
