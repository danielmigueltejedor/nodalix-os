import { memo } from "react";
import type { SidebarItem } from "../lib/sidebar";

interface SidebarProps {
  items: SidebarItem[];
  activePath: string;
  onNavigate: (path: string) => void;
  onContextMenu: (item: SidebarItem, x: number, y: number) => void;
}

function iconFor(kind: string) {
  const cls = "h-4 w-4 shrink-0 opacity-85";
  return (
    <svg className={cls} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" aria-hidden>
      {kind === "home" && <path d="M4 10.5L12 4l8 6.5V20a1 1 0 01-1 1h-5v-6H10v6H5a1 1 0 01-1-1v-9.5z" fill="currentColor" stroke="none" />}
      {kind === "data" && (
        <>
          <ellipse cx="12" cy="6" rx="8" ry="3" />
          <path d="M4 6v6c0 1.7 3.6 3 8 3s8-1.3 8-3V6M4 12v6c0 1.7 3.6 3 8 3s8-1.3 8-3v-6" />
        </>
      )}
      {kind !== "home" && kind !== "data" && (
        <path d="M7 4h7l5 5v11H7V4z" strokeLinejoin="round" />
      )}
    </svg>
  );
}

function SidebarInner({ items, activePath, onNavigate, onContextMenu }: SidebarProps) {
  return (
    <aside className="flex w-52 shrink-0 flex-col border-r border-nodalix-border bg-nodalix-surface/75 backdrop-blur-xl">
      <div className="px-3 py-3">
        <p className="text-[11px] font-semibold uppercase tracking-wider text-nodalix-accent">
          Places
        </p>
      </div>
      <nav className="flex-1 overflow-y-auto px-2 pb-3">
        {items.map((item) => {
          const active =
            activePath === item.path || activePath.startsWith(`${item.path}/`);
          return (
            <button
              key={item.id}
              type="button"
              onClick={() => onNavigate(item.path)}
              onContextMenu={(e) => {
                e.preventDefault();
                onContextMenu(item, e.clientX, e.clientY);
              }}
              className={[
                "mb-0.5 flex w-full items-center gap-2.5 rounded-xl px-2.5 py-2 text-left text-[13px] transition duration-150",
                active
                  ? "bg-nodalix-accent-dim text-nodalix-text shadow-sm shadow-nodalix-accent/10"
                  : "text-nodalix-muted hover:bg-white/[0.04] hover:text-nodalix-text",
              ].join(" ")}
            >
              {iconFor(item.kind)}
              <span className="truncate">{item.label}</span>
            </button>
          );
        })}
      </nav>
    </aside>
  );
}

export default memo(SidebarInner);
