import { memo, useState } from "react";
import type { LocalSendDevice } from "../lib/localsend";
import type { SidebarItem } from "../lib/sidebar";

interface SidebarProps {
  items: SidebarItem[];
  activePath: string;
  collapsed: boolean;
  onToggleCollapsed: () => void;
  onNavigate: (path: string) => void;
  onContextMenu: (item: SidebarItem, x: number, y: number) => void;
  onDropPaths?: (dataTransfer: DataTransfer) => void;
  onDropToItem?: (item: SidebarItem, dataTransfer: DataTransfer) => void;
  localsendAvailable?: boolean;
  localsendDevices?: LocalSendDevice[];
  localsendScanning?: boolean;
  localsendSendingId?: string | null;
  onRefreshLocalSend?: () => void;
  onDropToLocalSendDevice?: (
    device: LocalSendDevice,
    dataTransfer: DataTransfer,
  ) => void;
  onHoverItem?: (item: SidebarItem) => void;
  onLeaveHoverItem?: (item: SidebarItem) => void;
}

function canMoveToSidebarItem(item: SidebarItem) {
  return item.is_dir && item.kind !== "trash" && !item.path.startsWith("nodalix://");
}

function iconFor(kind: string) {
  const cls = "h-[18px] w-[18px] shrink-0";
  return (
    <svg
      className={cls}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      aria-hidden
    >
      {kind === "home" && (
        <path
          d="M4 10.5L12 4l8 6.5V20a1 1 0 01-1 1h-5v-6H10v6H5a1 1 0 01-1-1v-9.5z"
          fill="currentColor"
          stroke="none"
        />
      )}
      {kind === "desktop" && (
        <path
          d="M4 5h16v11H4zM9 20h6M12 16v4"
          strokeLinejoin="round"
          strokeLinecap="round"
        />
      )}
      {kind === "downloads" && (
        <path
          d="M12 4v11M7 10l5 5 5-5M5 20h14"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      )}
      {kind === "documents" && (
        <path
          d="M7 3h7l4 4v14H7V3zM14 3v5h4M9 12h6M9 16h6"
          strokeLinejoin="round"
          strokeLinecap="round"
        />
      )}
      {kind === "pictures" && (
        <path
          d="M4 6h16v13H4zM7 16l4-4 3 3 2-2 4 4M8 9.5h.1"
          strokeLinejoin="round"
          strokeLinecap="round"
        />
      )}
      {kind === "videos" && (
        <path
          d="M4 7h12v10H4zM16 10l4-2v8l-4-2"
          strokeLinejoin="round"
          strokeLinecap="round"
        />
      )}
      {kind === "music" && (
        <path
          d="M8 18a2.5 2.5 0 100-5 2.5 2.5 0 000 5zM16 16a2.5 2.5 0 100-5 2.5 2.5 0 000 5zM10.5 15.5V6l8-2v9.5"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      )}
      {kind === "trash" && (
        <path
          d="M6 7h12M9 7l1-3h4l1 3M8 7l.7 13h6.6L16 7M10.5 11v6M13.5 11v6"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      )}
      {kind === "data" && (
        <>
          <ellipse cx="12" cy="6" rx="8" ry="3" />
          <path d="M4 6v6c0 1.7 3.6 3 8 3s8-1.3 8-3V6M4 12v6c0 1.7 3.6 3 8 3s8-1.3 8-3v-6" />
        </>
      )}
      {kind === "disks" && (
        <path
          d="M6 5h12a2 2 0 012 2v10a2 2 0 01-2 2H6a2 2 0 01-2-2V7a2 2 0 012-2zM7 15h10M8 9h8M8 12h8M17 17h.1"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      )}
      {kind === "network" && (
        <path
          d="M12 5a7 7 0 017 7M12 5a7 7 0 00-7 7M12 5v14M5 12h14M7 17h10M8 7h8"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      )}
      {kind === "localsend" && (
        <>
          <circle cx="12" cy="12" r="5.8" />
          <path d="M1.65 9.05c-.25.95-.4 1.95-.4 2.95s.15 2 .4 2.9c.15 0 1.1-.3 1.15-1.3.05-.3 0-1.2 0-1.6-.05-.45.05-1.25 0-1.55-.1-1.05-1-1.4-1.15-1.4M22.35 9.1c.25.95.4 1.9.4 2.95 0 1-.15 2-.4 2.9-.15 0-1.1-.3-1.15-1.3-.05-.3 0-1.2 0-1.6.05-.45-.05-1.25 0-1.55.1-1.1 1-1.4 1.15-1.4M14.9 1.65c-.9-.25-1.9-.4-2.9-.4s-2 .15-2.9.4c0 .15.3 1.1 1.3 1.15.3.05 1.2 0 1.6 0 .45-.05 1.25.05 1.55 0 1.05-.1 1.35-1 1.35-1.15M14.9 22.35c-.95.25-1.9.4-2.95.4-1 0-2-.15-2.9-.4 0-.15.3-1.1 1.3-1.15.3-.05 1.2 0 1.6 0 .45.05 1.25-.05 1.55 0 1.1.1 1.4 1 1.4 1.15M6.75 2.6c-.85.45-1.65 1.05-2.35 1.8-.7.7-1.3 1.5-1.75 2.35.1.1 1 .55 1.75-.1.25-.2.85-.8 1.15-1.15.3-.3.9-.85 1.1-1.1.65-.8.2-1.65.1-1.8M21.35 17.25c-.45.85-1.05 1.65-1.8 2.35-.7.7-1.5 1.3-2.35 1.75-.1-.1-.55-1 .1-1.75.2-.25.8-.85 1.15-1.15.3-.3.85-.9 1.1-1.1.85-.6 1.7-.2 1.8-.1M17.25 2.6c.85.45 1.65 1.05 2.35 1.8.7.7 1.3 1.5 1.75 2.35-.1.1-1 .55-1.75-.1-.25-.2-.85-.8-1.15-1.15-.3-.3-.9-.85-1.1-1.1-.65-.8-.2-1.65-.1-1.8M2.65 17.25c.45.85 1.05 1.65 1.8 2.35.7.7 1.5 1.3 2.35 1.75.1-.1.55-1-.1-1.75-.2-.25-.8-.85-1.15-1.15-.3-.3-.85-.9-1.1-1.1-.85-.65-1.7-.2-1.8-.1" />
        </>
      )}
      {kind === "phone" && (
        <path d="M9 3h6a2 2 0 012 2v14a2 2 0 01-2 2H9a2 2 0 01-2-2V5a2 2 0 012-2zM11 18h2" strokeLinecap="round" strokeLinejoin="round" />
      )}
      {kind === "computer" && (
        <path d="M4 5h16v11H4zM9 20h6M12 16v4" strokeLinecap="round" strokeLinejoin="round" />
      )}
      {kind === "icloud" && (
        <path d="M5.5 17h12.5a4 4 0 00.4-8A6 6 0 006.9 7.4 5 5 0 005.5 17z" fill="currentColor" stroke="none" />
      )}
      {kind === "cloud" && (
        <path d="M5 17h13a4 4 0 10-.4-8A5.8 5.8 0 006.3 7.3 5 5 0 005 17z" strokeLinecap="round" strokeLinejoin="round" />
      )}
      {kind === "server" && (
        <path d="M5 5h14v5H5zM5 14h14v5H5zM8 7.5h.1M8 16.5h.1M12 7.5h4M12 16.5h4" strokeLinecap="round" strokeLinejoin="round" />
      )}
      {kind === "shared" && (
        <path d="M8 12a3 3 0 100-6 3 3 0 000 6zM17 10a2.5 2.5 0 100-5 2.5 2.5 0 000 5zM4 20a4 4 0 018 0M13.5 19a3.5 3.5 0 017 0" strokeLinecap="round" strokeLinejoin="round" />
      )}
      {kind === "terminal" && (
        <path d="M4 6h16v13H4zM7 10l3 2.5L7 15M12 16h5" strokeLinecap="round" strokeLinejoin="round" />
      )}
      {kind === "backup" && (
        <path d="M12 5a7 7 0 11-6.3 4M5 5v4h4M12 9v4l3 2" strokeLinecap="round" strokeLinejoin="round" />
      )}
      {kind === "apps" && (
        <path d="M5 5h5v5H5zM14 5h5v5h-5zM5 14h5v5H5zM14 14h5v5h-5z" fill="currentColor" stroke="none" />
      )}
      {kind === "folder" && (
        <path
          d="M4 7a2 2 0 012-2h5.5l2 2H18a2 2 0 012 2v9a2 2 0 01-2 2H6a2 2 0 01-2-2V7z"
          fill="currentColor"
          fillOpacity="0.9"
          stroke="none"
        />
      )}
      {![
        "home",
        "desktop",
        "downloads",
        "documents",
        "pictures",
        "videos",
        "music",
        "trash",
        "data",
        "disks",
        "network",
        "localsend",
        "phone",
        "computer",
        "icloud",
        "cloud",
        "server",
        "shared",
        "terminal",
        "backup",
        "apps",
        "folder",
      ].includes(kind) && (
        <path d="M7 4h7l5 5v11H7V4z" strokeLinejoin="round" />
      )}
    </svg>
  );
}

function LocalSendTarget({
  device,
  collapsed,
  sending,
  onDrop,
}: {
  device: LocalSendDevice;
  collapsed: boolean;
  sending: boolean;
  onDrop?: (device: LocalSendDevice, dataTransfer: DataTransfer) => void;
}) {
  const [dragOver, setDragOver] = useState(false);
  return (
    <button
      type="button"
      data-localsend-drop-target="true"
      data-localsend-device-id={device.id}
      title={collapsed ? device.alias : `${device.alias} · ${device.ip}`}
      className={[
        "nodalix-sidebar-item nodalix-sidebar-localsend-device mb-0.5 flex w-full items-center rounded-xl px-2.5 py-2 text-left text-[13px] transition duration-150",
        collapsed ? "justify-center gap-0" : "gap-2.5",
        dragOver && "nodalix-sidebar-localsend-drop-active",
        sending && "nodalix-sidebar-localsend-sending",
      ]
        .filter(Boolean)
        .join(" ")}
      onDragOver={(event) => {
        if (!onDrop) return;
        event.preventDefault();
        event.stopPropagation();
        event.dataTransfer.dropEffect = "copy";
      }}
      onDragEnter={(event) => {
        if (!onDrop) return;
        event.preventDefault();
        event.stopPropagation();
        setDragOver(true);
      }}
      onDragLeave={(event) => {
        if (event.currentTarget.contains(event.relatedTarget as Node | null)) {
          return;
        }
        setDragOver(false);
      }}
      onDrop={(event) => {
        if (!onDrop) return;
        event.preventDefault();
        event.stopPropagation();
        setDragOver(false);
        onDrop(device, event.dataTransfer);
      }}
      onDragEnd={() => setDragOver(false)}
    >
      <span
        className="nodalix-sidebar-icon"
        style={{ color: "var(--nx-sidebar-localsend-icon-color)" }}
      >
        {iconFor("localsend")}
      </span>
      {!collapsed && (
        <span className="min-w-0 flex-1">
          <span className="block truncate">{device.alias}</span>
          <span className="block truncate text-[10px] text-nodalix-muted/70">
            {sending ? "Sending…" : device.device_model || device.ip}
          </span>
        </span>
      )}
    </button>
  );
}

function SidebarButton({
  item,
  activePath,
  collapsed,
  onNavigate,
  onContextMenu,
  onDropToItem,
  onHoverItem,
  onLeaveHoverItem,
}: {
  item: SidebarItem;
  activePath: string;
  collapsed: boolean;
  onNavigate: (path: string) => void;
  onContextMenu: (item: SidebarItem, x: number, y: number) => void;
  onDropToItem?: (item: SidebarItem, dataTransfer: DataTransfer) => void;
  onHoverItem?: (item: SidebarItem) => void;
  onLeaveHoverItem?: (item: SidebarItem) => void;
}) {
  const active =
    activePath === item.path || activePath.startsWith(`${item.path}/`);
  const iconColor = active
    ? "var(--nx-sidebar-icon-active-color)"
    : item.kind === "trash"
      ? "var(--nx-sidebar-trash-icon-color)"
      : item.pinned
        ? "var(--nx-sidebar-pinned-icon-color)"
        : "var(--nx-sidebar-icon-color)";

  return (
    <button
      type="button"
      title={collapsed ? item.label : undefined}
      onClick={() => onNavigate(item.path)}
      onContextMenu={(e) => {
        e.preventDefault();
        e.stopPropagation();
        onContextMenu(item, e.clientX, e.clientY);
      }}
      onDragOver={(e) => {
        if (!onDropToItem || !canMoveToSidebarItem(item)) return;
        e.preventDefault();
        e.stopPropagation();
        e.dataTransfer.dropEffect = "move";
        onHoverItem?.(item);
      }}
      onDragEnter={() => {
        if (canMoveToSidebarItem(item)) onHoverItem?.(item);
      }}
      onDragLeave={() => {
        if (canMoveToSidebarItem(item)) onLeaveHoverItem?.(item);
      }}
      onDrop={(e) => {
        if (!onDropToItem || !canMoveToSidebarItem(item)) return;
        e.preventDefault();
        e.stopPropagation();
        onDropToItem(item, e.dataTransfer);
      }}
      className={[
        "nodalix-sidebar-item mb-0.5 flex w-full items-center rounded-xl px-2.5 py-2 text-left text-[13px] transition duration-150",
        collapsed ? "justify-center gap-0" : "gap-2.5",
        item.pinned && "nodalix-sidebar-item-pinned",
        active
          ? "nodalix-sidebar-item-active bg-nodalix-accent-dim text-nodalix-text"
          : "text-nodalix-muted hover:bg-white/[0.04] hover:text-nodalix-text",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <span className="nodalix-sidebar-icon" style={{ color: iconColor }}>
        {iconFor(item.kind)}
      </span>
      {!collapsed && <span className="truncate">{item.label}</span>}
    </button>
  );
}

function SidebarInner({
  items,
  activePath,
  collapsed,
  onToggleCollapsed,
  onNavigate,
  onContextMenu,
  onDropPaths,
  onDropToItem,
  localsendAvailable,
  localsendDevices = [],
  localsendScanning,
  localsendSendingId,
  onRefreshLocalSend,
  onDropToLocalSendDevice,
  onHoverItem,
  onLeaveHoverItem,
}: SidebarProps) {
  const lowerKinds = new Set(["trash"]);
  const primaryItems = items.filter((item) => !lowerKinds.has(item.kind));
  const fixedItems = primaryItems.filter((item) => !item.custom);
  const pinnedItems = primaryItems.filter((item) => item.custom);
  const lowerItems = items.filter((item) => lowerKinds.has(item.kind));

  return (
    <aside
      data-sidebar-pin-zone="true"
      className={[
        "nodalix-sidebar flex shrink-0 flex-col border-r border-nodalix-border/70 transition-[width] duration-200 ease-out",
        collapsed ? "w-16" : "w-52",
      ].join(" ")}
      onDragOver={(event) => {
        if (!onDropPaths) return;
        if (
          (event.target as HTMLElement | null)?.closest(
            "[data-localsend-drop-target='true']",
          )
        ) {
          return;
        }
        event.preventDefault();
        event.dataTransfer.dropEffect = "copy";
      }}
      onDrop={(event) => {
        if (
          !onDropPaths ||
          (event.target as HTMLElement | null)?.closest(
            "[data-localsend-drop-target='true']",
          ) ||
          !(event.target as HTMLElement | null)?.closest(
            "[data-sidebar-pin-zone='true']",
          )
        ) {
          return;
        }
        event.preventDefault();
        event.stopPropagation();
        onDropPaths(event.dataTransfer);
      }}
    >
      <div
        className={[
          "flex items-center gap-2 px-2 py-2",
          collapsed ? "justify-center" : "justify-between",
        ].join(" ")}
      >
        {!collapsed && (
          <p className="text-[10px] font-semibold uppercase tracking-wider text-nodalix-accent/85">
            Places
          </p>
        )}
        <button
          type="button"
          className="nodalix-icon-button nodalix-sidebar-toggle"
          title={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          onClick={onToggleCollapsed}
        >
          {collapsed ? "›" : "‹"}
        </button>
      </div>
      <nav className="flex-1 overflow-y-auto px-2 pb-2">
        {fixedItems.map((item) => (
          <SidebarButton
            key={item.id}
            item={item}
            activePath={activePath}
            collapsed={collapsed}
            onNavigate={onNavigate}
            onContextMenu={onContextMenu}
            onDropToItem={onDropToItem}
            onHoverItem={onHoverItem}
            onLeaveHoverItem={onLeaveHoverItem}
          />
        ))}
        {localsendAvailable && (
          <div className="nodalix-sidebar-localsend">
            <div
              className={[
                "nodalix-sidebar-localsend-header",
                collapsed && "nodalix-sidebar-localsend-header-collapsed",
              ]
                .filter(Boolean)
                .join(" ")}
            >
              <span
                className="nodalix-sidebar-icon"
                style={{ color: "var(--nx-sidebar-localsend-icon-color)" }}
              >
                {iconFor("localsend")}
              </span>
              {!collapsed && <span className="truncate">LocalSend</span>}
              {!collapsed && (
                <button
                  type="button"
                  className="nodalix-sidebar-localsend-refresh"
                  title="Refresh LocalSend devices"
                  aria-label="Refresh LocalSend devices"
                  onClick={(event) => {
                    event.stopPropagation();
                    onRefreshLocalSend?.();
                  }}
                >
                  {localsendScanning ? "…" : "↻"}
                </button>
              )}
            </div>
            {localsendDevices.map((device) => (
              <LocalSendTarget
                key={device.id}
                device={device}
                collapsed={collapsed}
                sending={localsendSendingId === device.id}
                onDrop={onDropToLocalSendDevice}
              />
            ))}
            {!collapsed && !localsendScanning && localsendDevices.length === 0 && (
              <p className="nodalix-sidebar-localsend-empty">
                No LocalSend devices
              </p>
            )}
          </div>
        )}
        {pinnedItems.length > 0 && (
          <div
            className={[
              "nodalix-sidebar-pinned-separator",
              collapsed && "nodalix-sidebar-pinned-separator-collapsed",
            ]
              .filter(Boolean)
              .join(" ")}
          >
            {!collapsed && <span>Pinned</span>}
          </div>
        )}
        {pinnedItems.map((item) => (
          <SidebarButton
            key={item.id}
            item={item}
            activePath={activePath}
            collapsed={collapsed}
            onNavigate={onNavigate}
            onContextMenu={onContextMenu}
            onDropToItem={onDropToItem}
            onHoverItem={onHoverItem}
            onLeaveHoverItem={onLeaveHoverItem}
          />
        ))}
      </nav>
      {lowerItems.length > 0 && (
        <nav className="border-t border-nodalix-border/70 px-2 py-2">
          {lowerItems.map((item) => (
            <SidebarButton
              key={item.id}
              item={item}
              activePath={activePath}
              collapsed={collapsed}
              onNavigate={onNavigate}
              onContextMenu={onContextMenu}
              onDropToItem={onDropToItem}
              onHoverItem={onHoverItem}
              onLeaveHoverItem={onLeaveHoverItem}
            />
          ))}
        </nav>
      )}
    </aside>
  );
}

export default memo(SidebarInner);
