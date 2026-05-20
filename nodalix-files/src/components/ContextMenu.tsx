import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";

interface ContextMenuProps {
  open: boolean;
  x: number;
  y: number;
  onClose: () => void;
  children: ReactNode;
}

export function ContextMenu({ open, x, y, onClose, children }: ContextMenuProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ x, y });

  useLayoutEffect(() => {
    if (!open || !ref.current) {
      setPos({ x, y });
      return;
    }
    const rect = ref.current.getBoundingClientRect();
    let nx = x;
    let ny = y;
    if (nx + rect.width > window.innerWidth - 8) nx = window.innerWidth - rect.width - 8;
    if (ny + rect.height > window.innerHeight - 8) ny = window.innerHeight - rect.height - 8;
    setPos({ x: Math.max(8, nx), y: Math.max(8, ny) });
  }, [open, x, y, children]);

  useEffect(() => {
    if (!open) return;
    const close = () => onClose();
    const t = window.setTimeout(() => {
      window.addEventListener("click", close);
      window.addEventListener("contextmenu", close);
    }, 0);
    return () => {
      clearTimeout(t);
      window.removeEventListener("click", close);
      window.removeEventListener("contextmenu", close);
    };
  }, [open, onClose]);

  if (!open) return null;

  return createPortal(
    <>
      <div className="fixed inset-0 z-[200]" onClick={onClose} />
      <div
        ref={ref}
        role="menu"
        className="nodalix-animate-in fixed z-[201] min-w-[210px] rounded-xl border border-nodalix-border bg-[rgba(24,24,37,0.94)] py-1 shadow-2xl shadow-black/50 backdrop-blur-xl"
        style={{ left: pos.x, top: pos.y }}
        onClick={(e) => e.stopPropagation()}
      >
        {children}
      </div>
    </>,
    document.body,
  );
}

interface ItemProps {
  label: string;
  icon?: ReactNode;
  disabled?: boolean;
  title?: string;
  onClick?: () => void;
}

export function ContextMenuItem({ label, icon, disabled, title, onClick }: ItemProps) {
  return (
    <button
      type="button"
      role="menuitem"
      title={title}
      disabled={disabled}
      onClick={() => !disabled && onClick?.()}
      className="flex w-full items-center gap-2.5 px-3 py-1.5 text-left text-[13px] text-nodalix-text transition enabled:hover:bg-nodalix-accent-dim disabled:cursor-not-allowed disabled:opacity-40"
    >
      <span className="flex h-4 w-4 shrink-0 items-center justify-center text-nodalix-muted">
        {icon}
      </span>
      <span className="truncate">{label}</span>
    </button>
  );
}

export function ContextMenuDangerItem(props: ItemProps) {
  return (
    <button
      type="button"
      role="menuitem"
      disabled={props.disabled}
      onClick={() => !props.disabled && props.onClick?.()}
      className="flex w-full items-center gap-2.5 px-3 py-1.5 text-left text-[13px] text-nodalix-danger transition enabled:hover:bg-nodalix-danger/15 disabled:opacity-40"
    >
      <span className="flex h-4 w-4 shrink-0 items-center justify-center">{props.icon}</span>
      <span>{props.label}</span>
    </button>
  );
}

export function ContextMenuSeparator() {
  return <div className="my-1 h-px bg-nodalix-border" />;
}

export function ContextMenuSubmenu({
  label,
  icon,
  children,
}: {
  label: string;
  icon?: ReactNode;
  children: ReactNode;
}) {
  const [open, setOpen] = useState(false);
  return (
    <div
      className="relative"
      onMouseEnter={() => setOpen(true)}
      onMouseLeave={() => setOpen(false)}
    >
      <div className="flex w-full items-center gap-2.5 px-3 py-1.5 text-[13px] text-nodalix-text hover:bg-nodalix-accent-dim">
        <span className="flex h-4 w-4 shrink-0 items-center justify-center text-nodalix-muted">
          {icon}
        </span>
        <span className="flex-1">{label}</span>
        <span className="text-nodalix-muted">›</span>
      </div>
      {open && (
        <div className="absolute left-full top-0 ml-1 min-w-[180px] rounded-xl border border-nodalix-border bg-[rgba(24,24,37,0.96)] py-1 shadow-xl backdrop-blur-xl">
          {children}
        </div>
      )}
    </div>
  );
}
