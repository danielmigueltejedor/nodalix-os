import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type MouseEvent as ReactMouseEvent,
  type PointerEvent as ReactPointerEvent,
  type ReactNode,
} from "react";
import { basename, parentPath } from "../lib/files";
import Breadcrumbs from "./Breadcrumbs";

export interface Tab {
  id: string;
  path: string;
  history: string[];
  historyIndex: number;
}

export type SortKey = "name" | "modified" | "created" | "type" | "size";
export type SortDirection = "asc" | "desc";

interface AccentOption {
  id: string;
  label: string;
  color: string;
}

interface ToolbarProps {
  tabs: Tab[];
  activeTabId: string;
  canGoBack: boolean;
  canGoForward: boolean;
  searchQuery: string;
  accentName: string;
  accentOptions: AccentOption[];
  pathLabels?: Record<string, string>;
  onBack: () => void;
  onForward: () => void;
  onBreadcrumb: (path: string) => void;
  onCopyPath: (path: string) => void;
  onBreadcrumbDrop: (path: string, dataTransfer: DataTransfer) => void;
  onPathHover: (path: string) => void;
  onPathLeave: (path: string) => void;
  onTabSelect: (tabId: string) => void;
  onTabClose: (tabId: string) => void;
  onTabReorder: (draggedTabId: string, targetTabId: string) => void;
  onNewTab: () => void;
  onSearchChange: (value: string) => void;
  onAccentChange: (value: string) => void;
}

function NavBtn({
  children,
  label,
  disabled,
  onClick,
}: {
  children: ReactNode;
  label: string;
  disabled?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      disabled={disabled}
      onClick={onClick}
      className="nodalix-icon-button disabled:opacity-30"
    >
      {children}
    </button>
  );
}

export default function Toolbar({
  tabs,
  activeTabId,
  canGoBack,
  canGoForward,
  searchQuery,
  accentName,
  accentOptions,
  pathLabels = {},
  onBack,
  onForward,
  onBreadcrumb,
  onCopyPath,
  onBreadcrumbDrop,
  onPathHover,
  onPathLeave,
  onTabSelect,
  onTabClose,
  onTabReorder,
  onNewTab,
  onSearchChange,
  onAccentChange,
}: ToolbarProps) {
  const activeTab = tabs.find((t) => t.id === activeTabId) ?? tabs[0];
  const win = getCurrentWindow();
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [searchOpen, setSearchOpen] = useState(() => searchQuery.trim() !== "");
  const [draggingTabId, setDraggingTabId] = useState<string | null>(null);
  const settingsPanelRef = useRef<HTMLDivElement>(null);
  const settingsButtonRef = useRef<HTMLButtonElement>(null);
  const multipleTabs = tabs.length > 1;
  const toggleSettings = useCallback(
    (event: ReactPointerEvent | ReactMouseEvent) => {
      event.preventDefault();
      event.stopPropagation();
      setSettingsOpen((open) => !open);
    },
    [],
  );

  useEffect(() => {
    if (!settingsOpen) return;

    const isInsideSettings = (target: EventTarget | null) => {
      if (!(target instanceof Node)) return false;
      return (
        settingsButtonRef.current?.contains(target) ||
        settingsPanelRef.current?.contains(target)
      );
    };

    const onPointerDown = (event: PointerEvent) => {
      if (isInsideSettings(event.target)) return;
      setSettingsOpen(false);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        setSettingsOpen(false);
      }
    };

    document.addEventListener("pointerdown", onPointerDown, true);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown, true);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [settingsOpen]);

  const tabsRow = (
    <div className="nodalix-tabs-row flex min-w-0 flex-1 items-center gap-1 overflow-x-auto">
      {tabs.map((tab) => (
        <div
          key={tab.id}
          draggable
          onDragStart={(event) => {
            setDraggingTabId(tab.id);
            event.dataTransfer.effectAllowed = "move";
            event.dataTransfer.setData("application/x-nodalix-tab-id", tab.id);
          }}
          onDragOver={(event) => {
            const tabId =
              draggingTabId ||
              event.dataTransfer.getData("application/x-nodalix-tab-id");
            if (!tabId || tabId === tab.id) return;
            event.preventDefault();
            event.dataTransfer.dropEffect = "move";
          }}
          onDrop={(event) => {
            const tabId =
              draggingTabId ||
              event.dataTransfer.getData("application/x-nodalix-tab-id");
            if (!tabId || tabId === tab.id) return;
            event.preventDefault();
            event.stopPropagation();
            onTabReorder(tabId, tab.id);
            setDraggingTabId(null);
          }}
          onDragEnd={() => setDraggingTabId(null)}
          className={[
            "nodalix-tab-chip group flex shrink-0 items-center gap-1 text-xs transition",
            draggingTabId === tab.id && "opacity-55",
            tab.id === activeTabId
              ? "nodalix-tab-chip-active min-w-0 flex-1 text-nodalix-text"
              : "max-w-36 text-nodalix-muted hover:text-nodalix-text",
          ].join(" ")}
        >
          {tab.id === activeTabId ? (
            <Breadcrumbs
              path={tab.path}
              pathLabels={pathLabels}
              compact
              className="nodalix-tab-breadcrumb"
              onNavigate={onBreadcrumb}
              onCopyPath={onCopyPath}
              onDropPath={onBreadcrumbDrop}
              onHoverPath={onPathHover}
              onLeaveHoverPath={onPathLeave}
            />
          ) : (
            <button
              type="button"
              className="truncate px-2"
              title={tab.path}
              onClick={() => {
                onTabSelect(tab.id);
                onCopyPath(tab.path);
              }}
              onContextMenu={(event) => {
                event.preventDefault();
                onCopyPath(tab.path);
              }}
            >
              {(pathLabels[tab.path] ?? basename(tab.path)) || "Root"}
            </button>
          )}
          {tabs.length > 1 && (
            <button
              type="button"
              aria-label="Close tab"
              title="Close tab"
              className="nodalix-tab-close"
              onClick={(event) => {
                event.stopPropagation();
                onTabClose(tab.id);
              }}
            >
              ×
            </button>
          )}
        </div>
      ))}
      <button
        type="button"
        onClick={onNewTab}
        className="nodalix-icon-button nodalix-new-tab-button h-7 w-7 shrink-0"
        aria-label="New tab"
        title="New tab"
      >
        +
      </button>
    </div>
  );

  return (
    <header
      className={[
        "nodalix-toolbar relative z-[300] px-3 py-2",
        searchOpen && "search-open",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <div className="nodalix-toolbar-row relative flex min-h-8 items-center gap-2">
        <div className="flex shrink-0 items-center gap-1">
          <NavBtn label="Back" disabled={!canGoBack} onClick={onBack}>
            <Chevron dir="left" />
          </NavBtn>
          <NavBtn label="Forward" disabled={!canGoForward} onClick={onForward}>
            <Chevron dir="right" />
          </NavBtn>
        </div>
        <div className="nodalix-route-tabs flex min-w-0 flex-1 items-center gap-2">
          {multipleTabs ? (
            tabsRow
          ) : (
            <>
              <div className="nodalix-route-wrap min-w-0 flex-1">
                {activeTab && (
                  <Breadcrumbs
                    path={activeTab.path}
                    pathLabels={pathLabels}
                    onNavigate={onBreadcrumb}
                    onCopyPath={onCopyPath}
                    onDropPath={onBreadcrumbDrop}
                    onHoverPath={onPathHover}
                    onLeaveHoverPath={onPathLeave}
                  />
                )}
              </div>
              <button
                type="button"
                onClick={onNewTab}
                className="nodalix-icon-button nodalix-new-tab-button h-7 w-7 shrink-0"
                aria-label="New tab"
                title="New tab"
              >
                +
              </button>
            </>
          )}
        </div>
        <div className="nodalix-search-shell">
          <button
            type="button"
            aria-label={searchOpen ? "Close search" : "Search"}
            title={searchOpen ? "Close search" : "Search"}
            aria-pressed={searchOpen}
            onClick={() => setSearchOpen((open) => !open)}
            className={[
              "nodalix-icon-button nodalix-search-toggle",
              searchQuery.trim() && "nodalix-search-toggle-active",
            ]
              .filter(Boolean)
              .join(" ")}
          >
            <SearchIcon />
          </button>
          <label className="nodalix-search-expanded">
            <input
              value={searchQuery}
              onChange={(e) => onSearchChange(e.target.value)}
              placeholder="Search"
              className="min-w-0 flex-1 bg-transparent text-nodalix-text outline-none placeholder:text-nodalix-muted/70"
            />
          </label>
        </div>
        <div
          className="nodalix-settings-anchor relative flex shrink-0 items-center gap-2"
          onPointerDown={(e) => e.stopPropagation()}
          onClick={(e) => e.stopPropagation()}
          onContextMenu={(e) => e.preventDefault()}
        >
          <button
            ref={settingsButtonRef}
            type="button"
            aria-label="Nodalix Files Settings"
            aria-expanded={settingsOpen}
            title="Nodalix Files Settings"
            onPointerDown={toggleSettings}
            className="nodalix-settings-button"
          >
            <SettingsIcon />
          </button>
          {settingsOpen && (
            <div
              ref={settingsPanelRef}
              className="nodalix-settings-popover nodalix-animate-in"
              role="dialog"
              aria-label="Nodalix Files Settings"
              onPointerDown={(e) => e.stopPropagation()}
              onClick={(e) => e.stopPropagation()}
              onContextMenu={(e) => e.preventDefault()}
            >
              <section>
                <p className="nodalix-settings-title">Appearance</p>
                <div className="grid grid-cols-3 gap-1.5">
                  {accentOptions.map((option) => (
                    <button
                      key={option.id}
                      type="button"
                      className={[
                        "nodalix-accent-swatch",
                        accentName === option.id &&
                          "nodalix-accent-swatch-active",
                      ]
                        .filter(Boolean)
                        .join(" ")}
                      onClick={() => onAccentChange(option.id)}
                      title={option.label}
                    >
                      <span style={{ background: option.color }} />
                      {option.label}
                    </button>
                  ))}
                </div>
              </section>
            </div>
          )}
          <button
            type="button"
            aria-label="Close"
            title="Close"
            onClick={() => void win.close()}
            className="nodalix-close-chip"
          >
            ×
          </button>
        </div>
      </div>
    </header>
  );
}

function SettingsIcon() {
  return (
    <svg
      className="h-[15px] w-[15px]"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
    >
      <path d="M12 15.5a3.5 3.5 0 100-7 3.5 3.5 0 000 7z" />
      <path d="M19.4 15a1.8 1.8 0 00.36 1.98l.04.04a2.1 2.1 0 01-2.98 2.98l-.04-.04A1.8 1.8 0 0014.8 19.6a1.8 1.8 0 00-1.1 1.64V21a2.1 2.1 0 01-4.2 0v-.06A1.8 1.8 0 008.4 19.3a1.8 1.8 0 00-1.98.36l-.04.04a2.1 2.1 0 01-2.98-2.98l.04-.04A1.8 1.8 0 003.8 14.7a1.8 1.8 0 00-1.64-1.1H2.1a2.1 2.1 0 010-4.2h.06A1.8 1.8 0 003.8 8.3a1.8 1.8 0 00-.36-1.98L3.4 6.28A2.1 2.1 0 016.38 3.3l.04.04A1.8 1.8 0 008.4 3.7a1.8 1.8 0 001.1-1.64V2a2.1 2.1 0 014.2 0v.06a1.8 1.8 0 001.1 1.64 1.8 1.8 0 001.98-.36l.04-.04a2.1 2.1 0 012.98 2.98l-.04.04A1.8 1.8 0 0019.4 8.3a1.8 1.8 0 001.64 1.1h.06a2.1 2.1 0 010 4.2h-.06A1.8 1.8 0 0019.4 15z" />
    </svg>
  );
}

function Chevron({ dir }: { dir: "left" | "right" }) {
  return (
    <svg
      className="h-4 w-4"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
    >
      <path d={dir === "left" ? "M15 6l-6 6 6 6" : "M9 6l6 6-6 6"} />
    </svg>
  );
}

function SearchIcon() {
  return (
    <svg
      className="h-4 w-4"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
    >
      <circle cx="11" cy="11" r="7" />
      <path d="M20 20l-3.5-3.5" />
    </svg>
  );
}

export function createTab(path: string, id?: string): Tab {
  return {
    id: id ?? crypto.randomUUID(),
    path,
    history: [path],
    historyIndex: 0,
  };
}

export function navigateTab(tab: Tab, path: string): Tab {
  const trimmed = path.replace(/\/+$/, "") || "/";
  if (trimmed === tab.path) return tab;
  const history = tab.history.slice(0, tab.historyIndex + 1);
  history.push(trimmed);
  return { ...tab, path: trimmed, history, historyIndex: history.length - 1 };
}

export function tabGoBack(tab: Tab): Tab | null {
  if (tab.historyIndex <= 0) return null;
  const historyIndex = tab.historyIndex - 1;
  return { ...tab, path: tab.history[historyIndex], historyIndex };
}

export function tabGoForward(tab: Tab): Tab | null {
  if (tab.historyIndex >= tab.history.length - 1) return null;
  const historyIndex = tab.historyIndex + 1;
  return { ...tab, path: tab.history[historyIndex], historyIndex };
}

export function tabGoUp(tab: Tab): Tab | null {
  const parent = parentPath(tab.path);
  if (!parent) return null;
  return navigateTab(tab, parent);
}
