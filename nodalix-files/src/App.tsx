import { useCallback, useEffect, useMemo, useState } from "react";
import {
  ContextMenu,
  ContextMenuDangerItem,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuSubmenu,
} from "./components/ContextMenu";
import ConfirmModal from "./components/ConfirmModal";
import FileGrid from "./components/FileGrid";
import FolderCustomizeModal from "./components/FolderCustomizeModal";
import MoveToModal from "./components/MoveToModal";
import OpenWithModal from "./components/OpenWithModal";
import PropertiesModal from "./components/PropertiesModal";
import RenameModal from "./components/RenameModal";
import Sidebar from "./components/Sidebar";
import Toolbar, {
  createTab,
  navigateTab,
  tabGoBack,
  tabGoForward,
  tabGoUp,
  type Tab,
} from "./components/Toolbar";
import WindowChrome from "./components/WindowChrome";
import { useContextMenu } from "./hooks/useContextMenu";
import { useDirectory } from "./hooks/useDirectory";
import { useFileActions } from "./hooks/useFileActions";
import { perfMark, usePerformanceMarks } from "./hooks/usePerformanceMarks";
import { useSidebarItems } from "./hooks/useSidebarItems";
import type { FileEntry } from "./lib/files";
import { parentPath, startupBundle } from "./lib/files";
import type { FolderStyle } from "./lib/folderCustomization";
import { setFolderColor, setFolderIcon } from "./lib/folderCustomization";
import type { PlatformInfo } from "./lib/platform";
import type { SidebarItem } from "./lib/sidebar";

type ModalKind =
  | "rename"
  | "properties"
  | "move"
  | "openWith"
  | "folderColor"
  | "folderIcon"
  | "confirmTrash"
  | "newFolder"
  | null;

export default function App() {
  const [ready, setReady] = useState(false);
  const [home, setHome] = useState("");
  const [platform, setPlatform] = useState<PlatformInfo | null>(null);
  const [localsendAvailable, setLocalsendAvailable] = useState(false);
  const [folderStyles, setFolderStyles] = useState<Record<string, FolderStyle>>({});
  const [tabs, setTabs] = useState<Tab[]>([]);
  const [activeTabId, setActiveTabId] = useState("");
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(new Set());
  const [modal, setModal] = useState<ModalKind>(null);
  const [modalTarget, setModalTarget] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [renameInitial, setRenameInitial] = useState("");

  const { menu, open: openMenu, close: closeMenu } = useContextMenu();
  const { items: sidebarItems, setItems: setSidebarItems, pin, unpin, hideBuiltin, renameAccess, removeAccess } = useSidebarItems();
  const { entries, loading, error, load, invalidate, setError } = useDirectory();

  const activeTab = useMemo(
    () => tabs.find((t) => t.id === activeTabId) ?? tabs[0],
    [tabs, activeTabId],
  );

  const cwd = activeTab?.path ?? home;

  const refreshDir = useCallback(
    async (path: string) => {
      invalidate(path);
      return load(path);
    },
    [invalidate, load],
  );

  const actions = useFileActions(refreshDir);
  const { mark } = usePerformanceMarks(platform?.debug ?? false);

  useEffect(() => {
    perfMark("react_mount");
    startupBundle()
      .then((bundle) => {
        setHome(bundle.home);
        setPlatform(bundle.platform);
        setSidebarItems(bundle.sidebar_items);
        setLocalsendAvailable(bundle.localsend_available);
        setFolderStyles(bundle.folder_customizations);
        const tab = createTab(bundle.home);
        setTabs([tab]);
        setActiveTabId(tab.id);
        setReady(true);
        mark("startup_bundle_done");
        perfMark("window_content_ready");
      })
      .catch((e) => setError(e instanceof Error ? e.message : String(e)));
  }, [mark, setSidebarItems, setError]);

  useEffect(() => {
    if (!ready || !activeTab) return;
    setSelectedPaths(new Set());
    void load(activeTab.path);
    mark(`navigate ${activeTab.path}`);
  }, [activeTab?.path, ready, load, mark]);

  const updateActiveTab = useCallback(
    (updater: (tab: Tab) => Tab | null) => {
      setTabs((prev) =>
        prev.map((t) => {
          if (t.id !== activeTabId) return t;
          const next = updater(t);
          return next ?? t;
        }),
      );
    },
    [activeTabId],
  );

  const goTo = useCallback(
    (path: string) => updateActiveTab((tab) => navigateTab(tab, path)),
    [updateActiveTab],
  );

  const handleOpen = useCallback(
    async (entry: FileEntry) => {
      if (entry.is_dir) {
        goTo(entry.path);
        return;
      }
      try {
        await actions.open(entry);
      } catch (e) {
        setActionError(e instanceof Error ? e.message : String(e));
      }
    },
    [goTo, actions],
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

  const pathsForAction = useCallback(() => {
    if (selectedPaths.size > 0) return [...selectedPaths];
    return modalTarget ? [modalTarget] : [];
  }, [selectedPaths, modalTarget]);

  const showFileContextMenu = useCallback(
    (entry: FileEntry, x: number, y: number) => {
      const targets =
        selectedPaths.has(entry.path) && selectedPaths.size > 1
          ? [...selectedPaths]
          : [entry.path];
      setModalTarget(entry.path);
      if (!selectedPaths.has(entry.path)) setSelectedPaths(new Set([entry.path]));

      openMenu(
        x,
        y,
        <>
          <ContextMenuItem
            label={targets.length > 1 ? `Open (${targets.length})` : "Open"}
            onClick={() => void handleOpen(entry)}
          />
          <ContextMenuSubmenu label="Open with…">
            <ContextMenuItem
              label="Choose application…"
              onClick={() => {
                closeMenu();
                setModal("openWith");
              }}
            />
          </ContextMenuSubmenu>
          <ContextMenuSeparator />
          <ContextMenuItem
            label="Rename"
            onClick={() => {
              closeMenu();
              setRenameInitial(entry.name);
              setModal("rename");
            }}
          />
          <ContextMenuItem
            label="Copy"
            onClick={() => {
              closeMenu();
              void actions.copy(targets);
            }}
          />
          <ContextMenuItem
            label="Cut"
            onClick={() => {
              closeMenu();
              void actions.cut(targets);
            }}
          />
          <ContextMenuItem
            label="Paste"
            disabled={!actions.clipboardActive}
            onClick={() => void actions.paste(cwd)}
          />
          <ContextMenuItem
            label="Move to…"
            onClick={() => {
              closeMenu();
              setModal("move");
            }}
          />
          <ContextMenuSeparator />
          <ContextMenuDangerItem
            label="Move to Trash"
            onClick={() => {
              closeMenu();
              setModal("confirmTrash");
            }}
          />
          {localsendAvailable && (
            <ContextMenuItem
              label="Send with LocalSend"
              onClick={() => void actions.localSend(targets)}
            />
          )}
          <ContextMenuItem
            label="Properties"
            onClick={() => {
              closeMenu();
              setModal("properties");
            }}
          />
          {entry.is_dir && (
            <>
              <ContextMenuSeparator />
              <ContextMenuSubmenu label="Folder appearance">
                <ContextMenuItem
                  label="Change color…"
                  onClick={() => {
                    closeMenu();
                    setModal("folderColor");
                  }}
                />
                <ContextMenuItem
                  label="Change icon…"
                  onClick={() => {
                    closeMenu();
                    setModal("folderIcon");
                  }}
                />
              </ContextMenuSubmenu>
              <ContextMenuItem
                label="Pin to sidebar"
                onClick={() => void pin(entry.path, entry.name)}
              />
              <ContextMenuItem
                label="Open terminal here"
                onClick={() => void actions.terminalHere(entry.path)}
              />
            </>
          )}
        </>,
      );
    },
    [
      openMenu,
      closeMenu,
      handleOpen,
      actions,
      cwd,
      localsendAvailable,
      pin,
      selectedPaths,
    ],
  );

  const showSidebarContextMenu = useCallback(
    (item: SidebarItem, x: number, y: number) => {
      openMenu(
        x,
        y,
        <>
          <ContextMenuItem label="Open" onClick={() => goTo(item.path)} />
          <ContextMenuItem
            label="Open in new window"
            disabled
            title="TODO: multi-window support"
          />
          <ContextMenuSeparator />
          {item.pinned ? (
            <ContextMenuItem
              label="Unpin from sidebar"
              onClick={() =>
                void (item.custom ? unpin(item.path) : hideBuiltin(item.id))
              }
            />
          ) : (
            <ContextMenuItem
              label="Pin to sidebar"
              onClick={() => void pin(item.path, item.label)}
            />
          )}
          {item.can_rename && (
            <ContextMenuItem
              label="Rename access"
              onClick={() => {
                closeMenu();
                setRenameInitial(item.label);
                setModalTarget(item.id);
                setModal("rename");
              }}
            />
          )}
          {item.can_remove && (
            <ContextMenuItem
              label="Remove from sidebar"
              onClick={() => void removeAccess(item.id)}
            />
          )}
          <ContextMenuSeparator />
          <ContextMenuItem
            label="Show in folder"
            onClick={() => {
              const parent = parentPath(item.path);
              if (parent) goTo(parent);
            }}
          />
          <ContextMenuItem
            label="Properties"
            onClick={() => {
              closeMenu();
              setModalTarget(item.path);
              setModal("properties");
            }}
          />
        </>,
      );
    },
    [openMenu, closeMenu, goTo, pin, unpin, hideBuiltin, removeAccess],
  );

  const showBackgroundMenu = useCallback(
    (x: number, y: number) => {
      openMenu(
        x,
        y,
        <>
          <ContextMenuItem
            label="New folder"
            onClick={() => setModal("newFolder")}
          />
          <ContextMenuItem
            label="Paste"
            disabled={!actions.clipboardActive}
            onClick={() => void actions.paste(cwd)}
          />
        </>,
      );
    },
    [openMenu, actions, cwd],
  );

  if (!ready || !platform || !activeTab) {
    return (
      <div className="flex h-full flex-col bg-[#0b0b12]">
        <div className="flex flex-1 flex-col items-center justify-center gap-3">
          <div className="h-10 w-10 animate-pulse rounded-2xl bg-nodalix-accent-dim" />
          <p className="text-sm text-nodalix-muted">
            {error ?? "Loading Nodalix Files…"}
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col overflow-hidden bg-[#0b0b12]">
      <WindowChrome platform={platform} />
      <div className="flex min-h-0 flex-1">
        <Sidebar
          items={sidebarItems}
          activePath={activeTab.path}
          onNavigate={goTo}
          onContextMenu={showSidebarContextMenu}
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
            onTabClose={(id) => {
              setTabs((prev) => {
                if (prev.length <= 1) return prev;
                const next = prev.filter((t) => t.id !== id);
                if (activeTabId === id) setActiveTabId(next[0].id);
                return next;
              });
            }}
            onNewTab={() => {
              const t = createTab(home);
              setTabs((p) => [...p, t]);
              setActiveTabId(t.id);
            }}
            onNewFolder={() => setModal("newFolder")}
          />
          {actionError && (
            <div className="mx-3 mt-1 rounded-lg border border-nodalix-danger/30 bg-nodalix-danger/10 px-3 py-1.5 text-xs text-nodalix-danger">
              {actionError}
              <button
                type="button"
                className="ml-2 underline"
                onClick={() => setActionError(null)}
              >
                dismiss
              </button>
            </div>
          )}
          <FileGrid
            entries={entries}
            loading={loading}
            error={error}
            selectedPaths={selectedPaths}
            folderStyles={folderStyles}
            onSelect={handleSelect}
            onOpen={(e) => void handleOpen(e)}
            onContextMenu={showFileContextMenu}
            onBackgroundContext={showBackgroundMenu}
          />
        </main>
      </div>

      <ContextMenu
        open={menu.open}
        x={menu.x}
        y={menu.y}
        onClose={closeMenu}
      >
        {menu.items}
      </ContextMenu>

      <RenameModal
        open={modal === "rename" || modal === "newFolder"}
        initialName={modal === "newFolder" ? "New Folder" : renameInitial}
        onClose={() => setModal(null)}
        error={actionError}
        onConfirm={async (name) => {
          try {
            if (modal === "newFolder") {
              await actions.newFolder(cwd, name);
            } else if (modalTarget && sidebarItems.some((s) => s.id === modalTarget)) {
              await renameAccess(modalTarget, name);
            } else {
              const p = modalTarget ?? pathsForAction()[0];
              if (p) await actions.rename(p, name, cwd);
            }
            setActionError(null);
          } catch (e) {
            setActionError(e instanceof Error ? e.message : String(e));
            throw e;
          }
        }}
      />

      <PropertiesModal
        open={modal === "properties"}
        path={modalTarget}
        onClose={() => setModal(null)}
      />

      <MoveToModal
        open={modal === "move"}
        startPath={cwd}
        onClose={() => setModal(null)}
        onConfirm={async (target) => {
          await actions.move(pathsForAction(), target, cwd);
          setModal(null);
        }}
      />

      <OpenWithModal
        open={modal === "openWith"}
        path={modalTarget}
        onClose={() => setModal(null)}
        onSelect={async (appId) => {
          if (modalTarget) await actions.openWithApp(modalTarget, appId);
        }}
      />

      <FolderCustomizeModal
        open={modal === "folderColor" || modal === "folderIcon"}
        mode={modal === "folderIcon" ? "icon" : "color"}
        onClose={() => setModal(null)}
        onPick={async (value) => {
          const p = modalTarget;
          if (!p) return;
          if (modal === "folderColor") {
            await setFolderColor(p, value || null);
            setFolderStyles((s) => ({
              ...s,
              [p]: { ...s[p], color: value || null },
            }));
          } else {
            await setFolderIcon(p, value || null);
            setFolderStyles((s) => ({
              ...s,
              [p]: { ...s[p], icon: value || null },
            }));
          }
          setModal(null);
        }}
      />

      <ConfirmModal
        open={modal === "confirmTrash"}
        title="Move to Trash"
        message={`Move ${pathsForAction().length} item(s) to trash?`}
        confirmLabel="Move to Trash"
        onClose={() => setModal(null)}
        onConfirm={async () => {
          await actions.trash(pathsForAction(), cwd);
          setSelectedPaths(new Set());
          setModal(null);
        }}
      />
    </div>
  );
}
