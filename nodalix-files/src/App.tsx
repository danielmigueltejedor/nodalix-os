import { useCallback, useEffect, useMemo, useState } from "react";
import FileGrid from "./components/FileGrid";
import Sidebar, { buildSidebarLocations } from "./components/Sidebar";
import Toolbar, {
  createTab,
  navigateTab,
  tabGoBack,
  tabGoForward,
  tabGoUp,
  type Tab,
} from "./components/Toolbar";
import type { FileEntry, SpecialDirs } from "./lib/files";
import {
  createFolder,
  getSpecialDirs,
  listDirectory,
  openPath,
  parentPath,
  renamePath,
  trashPath,
} from "./lib/files";

export default function App() {
  const [specialDirs, setSpecialDirs] = useState<SpecialDirs | null>(null);
  const [tabs, setTabs] = useState<Tab[]>([]);
  const [activeTabId, setActiveTabId] = useState("");
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(new Set());

  const activeTab = useMemo(
    () => tabs.find((t) => t.id === activeTabId) ?? tabs[0],
    [tabs, activeTabId],
  );

  const locations = useMemo(
    () => (specialDirs ? buildSidebarLocations(specialDirs) : []),
    [specialDirs],
  );

  const refresh = useCallback(async (path: string) => {
    setLoading(true);
    setError(null);
    try {
      const list = await listDirectory(path);
      setEntries(list);
    } catch (e) {
      setEntries([]);
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const dirs = await getSpecialDirs();
        if (cancelled) return;
        setSpecialDirs(dirs);
        const tab = createTab(dirs.home);
        setTabs([tab]);
        setActiveTabId(tab.id);
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : String(e));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!activeTab) return;
    setSelectedPaths(new Set());
    void refresh(activeTab.path);
  }, [activeTab?.path, refresh]);

  const updateActiveTab = useCallback((updater: (tab: Tab) => Tab | null) => {
    setTabs((prev) =>
      prev.map((t) => {
        if (t.id !== activeTabId) return t;
        const next = updater(t);
        return next ?? t;
      }),
    );
  }, [activeTabId]);

  const goTo = useCallback(
    (path: string) => {
      updateActiveTab((tab) => navigateTab(tab, path));
    },
    [updateActiveTab],
  );

  const handleOpen = useCallback(
    async (entry: FileEntry) => {
      if (entry.is_dir) {
        goTo(entry.path);
        return;
      }
      try {
        await openPath(entry.path);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    },
    [goTo],
  );

  const handleSelect = useCallback((entry: FileEntry, additive: boolean) => {
    setSelectedPaths((prev) => {
      if (!additive) return new Set([entry.path]);
      const next = new Set(prev);
      if (next.has(entry.path)) next.delete(entry.path);
      else next.add(entry.path);
      return next;
    });
  }, []);

  const selectedEntry = useMemo(() => {
    if (selectedPaths.size !== 1) return null;
    const path = [...selectedPaths][0];
    return entries.find((e) => e.path === path) ?? null;
  }, [selectedPaths, entries]);

  const handleNewFolder = useCallback(async () => {
    if (!activeTab) return;
    const name = window.prompt("Folder name", "New Folder");
    if (!name?.trim()) return;
    try {
      await createFolder(activeTab.path, name.trim());
      await refresh(activeTab.path);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [activeTab, refresh]);

  const handleRename = useCallback(async () => {
    if (!selectedEntry || !activeTab) return;
    const name = window.prompt("Rename to", selectedEntry.name);
    if (!name?.trim() || name === selectedEntry.name) return;
    try {
      await renamePath(selectedEntry.path, name.trim());
      await refresh(activeTab.path);
      setSelectedPaths(new Set());
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [selectedEntry, activeTab, refresh]);

  const handleTrash = useCallback(async () => {
    if (!activeTab || selectedPaths.size === 0) return;
    const confirmed = window.confirm(
      `Move ${selectedPaths.size} item(s) to trash?`,
    );
    if (!confirmed) return;
    try {
      for (const path of selectedPaths) {
        await trashPath(path);
      }
      setSelectedPaths(new Set());
      await refresh(activeTab.path);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [activeTab, selectedPaths, refresh]);

  const handleNewTab = useCallback(() => {
    const home = specialDirs?.home ?? "/";
    const tab = createTab(home);
    setTabs((prev) => [...prev, tab]);
    setActiveTabId(tab.id);
  }, [specialDirs]);

  const handleCloseTab = useCallback(
    (tabId: string) => {
      setTabs((prev) => {
        if (prev.length <= 1) return prev;
        const next = prev.filter((t) => t.id !== tabId);
        if (activeTabId === tabId) {
          setActiveTabId(next[0].id);
        }
        return next;
      });
    },
    [activeTabId],
  );

  if (!specialDirs || !activeTab) {
    return (
      <div className="flex h-full items-center justify-center text-nodalix-muted">
        {error ?? "Starting Nodalix Files…"}
      </div>
    );
  }

  return (
    <div className="flex h-full overflow-hidden rounded-xl border border-nodalix-border bg-nodalix-bg/60 shadow-2xl shadow-black/40 backdrop-blur-2xl">
      <Sidebar
        locations={locations}
        activePath={activeTab.path}
        onNavigate={goTo}
      />
      <main className="flex min-w-0 flex-1 flex-col">
        <Toolbar
          tabs={tabs}
          activeTabId={activeTabId}
          canGoBack={activeTab.historyIndex > 0}
          canGoForward={activeTab.historyIndex < activeTab.history.length - 1}
          canGoUp={parentPath(activeTab.path) !== null}
          onBack={() => updateActiveTab((t) => tabGoBack(t))}
          onForward={() => updateActiveTab((t) => tabGoForward(t))}
          onUp={() => updateActiveTab((t) => tabGoUp(t))}
          onBreadcrumb={goTo}
          onTabSelect={setActiveTabId}
          onTabClose={handleCloseTab}
          onNewTab={handleNewTab}
          onNewFolder={() => void handleNewFolder()}
          onRename={() => void handleRename()}
          onTrash={() => void handleTrash()}
          hasSelection={selectedPaths.size > 0}
        />
        <FileGrid
          entries={entries}
          loading={loading}
          error={error}
          selectedPaths={selectedPaths}
          onSelect={handleSelect}
          onOpen={(entry) => void handleOpen(entry)}
        />
      </main>
    </div>
  );
}
