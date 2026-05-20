import type { ReactNode } from "react";
import { basename, parentPath } from "../lib/files";
import Breadcrumbs from "./Breadcrumbs";

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
      className="rounded-lg p-1.5 text-nodalix-muted transition enabled:hover:bg-nodalix-accent-dim enabled:hover:text-nodalix-accent disabled:opacity-30"
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
  canGoUp,
  onBack,
  onForward,
  onUp,
  onBreadcrumb,
  onTabSelect,
  onTabClose,
  onNewTab,
  onNewFolder,
}: ToolbarProps) {
  const activeTab = tabs.find((t) => t.id === activeTabId) ?? tabs[0];

  return (
    <header className="flex flex-col gap-1.5 border-b border-nodalix-border bg-nodalix-bg/80 px-3 py-2 backdrop-blur-md">
      <div className="flex items-center gap-1 overflow-x-auto">
        {tabs.map((tab) => (
          <div
            key={tab.id}
            className={[
              "group flex max-w-[160px] shrink-0 items-center gap-0.5 rounded-lg px-2 py-0.5 text-xs transition",
              tab.id === activeTabId
                ? "bg-nodalix-accent-dim text-nodalix-text"
                : "text-nodalix-muted hover:bg-white/5",
            ].join(" ")}
          >
            <button type="button" className="truncate" onClick={() => onTabSelect(tab.id)}>
              {basename(tab.path) || "Root"}
            </button>
            {tabs.length > 1 && (
              <button
                type="button"
                className="rounded px-1 opacity-0 group-hover:opacity-100 hover:bg-white/10"
                onClick={() => onTabClose(tab.id)}
              >
                ×
              </button>
            )}
          </div>
        ))}
        <button
          type="button"
          onClick={onNewTab}
          className="rounded-lg px-2 text-lg text-nodalix-muted hover:bg-nodalix-accent-dim hover:text-nodalix-accent"
        >
          +
        </button>
      </div>

      <div className="flex items-center gap-2">
        <div className="flex shrink-0 items-center gap-0.5">
          <NavBtn label="Back" disabled={!canGoBack} onClick={onBack}>
            <Chevron dir="left" />
          </NavBtn>
          <NavBtn label="Forward" disabled={!canGoForward} onClick={onForward}>
            <Chevron dir="right" />
          </NavBtn>
          <NavBtn label="Up" disabled={!canGoUp} onClick={onUp}>
            <svg className="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M12 19V5M5 12l7-7 7 7" />
            </svg>
          </NavBtn>
        </div>
        {activeTab && (
          <Breadcrumbs path={activeTab.path} onNavigate={onBreadcrumb} />
        )}
        <button
          type="button"
          onClick={onNewFolder}
          className="shrink-0 rounded-lg border border-nodalix-border px-2.5 py-1 text-xs text-nodalix-muted transition hover:border-nodalix-accent/40 hover:bg-nodalix-accent-dim hover:text-nodalix-text"
        >
          New folder
        </button>
      </div>
    </header>
  );
}

function Chevron({ dir }: { dir: "left" | "right" }) {
  return (
    <svg className="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d={dir === "left" ? "M15 6l-6 6 6 6" : "M9 6l6 6-6 6"} />
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
