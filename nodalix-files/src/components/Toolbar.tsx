import type { ReactNode } from "react";
import { basename, parentPath } from "../lib/files";

export interface Tab {
  id: string;
  path: string;
  history: string[];
  historyIndex: number;
}

interface ToolbarProps {
  tabs: Tab[];
  activeTabId: string;
  canGoBack: boolean;
  canGoForward: boolean;
  canGoUp: boolean;
  onBack: () => void;
  onForward: () => void;
  onUp: () => void;
  onBreadcrumb: (path: string) => void;
  onTabSelect: (tabId: string) => void;
  onTabClose: (tabId: string) => void;
  onNewTab: () => void;
  onNewFolder: () => void;
  onRename: () => void;
  onTrash: () => void;
  hasSelection: boolean;
}

function ChevronLeft() {
  return (
    <svg className="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <path d="M15 6l-6 6 6 6" />
    </svg>
  );
}

function ChevronRight() {
  return (
    <svg className="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <path d="M9 6l6 6-6 6" />
    </svg>
  );
}

function ArrowUp() {
  return (
    <svg className="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <path d="M12 19V5M5 12l7-7 7 7" />
    </svg>
  );
}

function buildBreadcrumbs(path: string): { label: string; path: string }[] {
  if (path === "/") return [{ label: "/", path: "/" }];
  const parts = path.split("/").filter(Boolean);
  const crumbs: { label: string; path: string }[] = [{ label: "/", path: "/" }];
  let acc = "";
  for (const part of parts) {
    acc += `/${part}`;
    crumbs.push({ label: part, path: acc });
  }
  return crumbs;
}

export default function Toolbar({
  tabs,
  activeTabId,
  canGoBack,
  canGoForward,
  canGoUp,
  onBack,
  onForward,
  onUp,
  onBreadcrumb,
  onTabSelect,
  onTabClose,
  onNewTab,
  onNewFolder,
  onRename,
  onTrash,
  hasSelection,
}: ToolbarProps) {
  const activeTab = tabs.find((t) => t.id === activeTabId) ?? tabs[0];
  const crumbs = activeTab ? buildBreadcrumbs(activeTab.path) : [];

  return (
    <header className="flex flex-col gap-2 border-b border-nodalix-border bg-nodalix-bg/90 px-3 py-2 backdrop-blur-xl">
      <div className="flex items-center gap-1 overflow-x-auto">
        {tabs.map((tab) => (
          <div
            key={tab.id}
            className={[
              "group flex max-w-[180px] shrink-0 items-center gap-1 rounded-lg border px-2 py-1 text-xs transition-colors",
              tab.id === activeTabId
                ? "border-nodalix-accent/50 bg-nodalix-accent-dim text-nodalix-text"
                : "border-transparent bg-nodalix-card/30 text-nodalix-muted hover:text-nodalix-text",
            ].join(" ")}
          >
            <button type="button" className="truncate" onClick={() => onTabSelect(tab.id)}>
              {basename(tab.path) || "Root"}
            </button>
            {tabs.length > 1 && (
              <button
                type="button"
                className="rounded px-1 text-nodalix-muted opacity-0 hover:bg-white/10 group-hover:opacity-100"
                onClick={() => onTabClose(tab.id)}
                aria-label="Close tab"
              >
                ×
              </button>
            )}
          </div>
        ))}
        <button
          type="button"
          onClick={onNewTab}
          className="rounded-lg px-2 py-1 text-lg leading-none text-nodalix-muted hover:bg-nodalix-accent-dim hover:text-nodalix-accent"
          aria-label="New tab"
        >
          +
        </button>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <div className="flex items-center gap-1">
          <IconButton label="Back" disabled={!canGoBack} onClick={onBack}>
            <ChevronLeft />
          </IconButton>
          <IconButton label="Forward" disabled={!canGoForward} onClick={onForward}>
            <ChevronRight />
          </IconButton>
          <IconButton label="Up" disabled={!canGoUp} onClick={onUp}>
            <ArrowUp />
          </IconButton>
        </div>

        <nav className="flex min-w-0 flex-1 flex-wrap items-center gap-1 text-sm">
          {crumbs.map((crumb, i) => (
            <span key={crumb.path} className="flex min-w-0 items-center gap-1">
              {i > 0 && <span className="text-nodalix-muted">/</span>}
              <button
                type="button"
                className="truncate rounded-md px-1.5 py-0.5 text-nodalix-muted transition hover:bg-nodalix-accent-dim hover:text-nodalix-accent"
                onClick={() => onBreadcrumb(crumb.path)}
              >
                {crumb.label}
              </button>
            </span>
          ))}
        </nav>

        <div className="flex items-center gap-1">
          <ActionButton onClick={onNewFolder}>New folder</ActionButton>
          <ActionButton onClick={onRename} disabled={!hasSelection}>
            Rename
          </ActionButton>
          <ActionButton onClick={onTrash} disabled={!hasSelection} variant="danger">
            Delete
          </ActionButton>
        </div>
      </div>
    </header>
  );
}

function IconButton({
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
      className="rounded-lg p-1.5 text-nodalix-muted transition enabled:hover:bg-nodalix-accent-dim enabled:hover:text-nodalix-accent disabled:opacity-35"
    >
      {children}
    </button>
  );
}

function ActionButton({
  children,
  onClick,
  disabled,
  variant = "default",
}: {
  children: ReactNode;
  onClick: () => void;
  disabled?: boolean;
  variant?: "default" | "danger";
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className={[
        "rounded-lg border px-2.5 py-1 text-xs transition enabled:hover:-translate-y-px",
        variant === "danger"
          ? "border-nodalix-danger/30 text-nodalix-danger enabled:hover:bg-nodalix-danger/15"
          : "border-nodalix-border text-nodalix-muted enabled:hover:border-nodalix-accent/40 enabled:hover:bg-nodalix-accent-dim enabled:hover:text-nodalix-text",
        "disabled:opacity-35",
      ].join(" ")}
    >
      {children}
    </button>
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
