import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type FormEvent,
  type ReactNode,
} from "react";
import { listen } from "@tauri-apps/api/event";
import {
  ContextMenu,
  ContextMenuDangerItem,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuSubmenu,
} from "./components/ContextMenu";
import ConfirmModal from "./components/ConfirmModal";
import FileGrid, { type ViewMode, type ZoomLevel } from "./components/FileGrid";
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
  type SortDirection,
  type SortKey,
  type Tab,
} from "./components/Toolbar";

import { useContextMenu } from "./hooks/useContextMenu";
import { useDirectory } from "./hooks/useDirectory";
import { useFileActions } from "./hooks/useFileActions";
import { perfMark, usePerformanceMarks } from "./hooks/usePerformanceMarks";
import { useSidebarItems } from "./hooks/useSidebarItems";
import type { FileEntry, SpecialDirs } from "./lib/files";
import {
  getFileKind,
  isHiddenEntry,
  isTrashPath,
  copyTextToClipboard,
  connectNetworkLocation,
  quitApp,
  listDisks,
  listNetworkLocations,
  normalizePath,
  parentPath,
  startupBundle,
} from "./lib/files";
import type { FolderStyle } from "./lib/folderCustomization";
import { setFolderColor, setFolderIcon } from "./lib/folderCustomization";
import {
  discoverLocalSendDevices,
  sendToLocalSendDevice,
  type LocalSendDevice,
} from "./lib/localsend";
import type { PlatformInfo } from "./lib/platform";
import type { SidebarItem } from "./lib/sidebar";
import { FileTypeIcon } from "./lib/icons";

type ModalKind =
  | "rename"
  | "properties"
  | "move"
  | "openWith"
  | "folderColor"
  | "folderIcon"
  | "sidebarIcon"
  | "confirmPermanentDelete"
  | null;

const sortKeys: SortKey[] = ["name", "modified", "created", "type", "size"];
const DISKS_PATH = "nodalix://disks";
const NETWORK_PATH = "nodalix://network";
type FilterType = "all" | "folders" | "files";

export type AccentName =
  | "purple"
  | "blue"
  | "green"
  | "orange"
  | "pink"
  | "graphite";

const accentNames: AccentName[] = [
  "purple",
  "blue",
  "green",
  "orange",
  "pink",
  "graphite",
];

const accentLabels: Record<AccentName, string> = {
  purple: "Morado",
  blue: "Azul",
  green: "Verde",
  orange: "Naranja",
  pink: "Rosa",
  graphite: "Grafito",
};

const accentPresets: Record<
  AccentName,
  { accent: string; soft: string; border: string; text: string }
> = {
  purple: {
    accent: "#cba6f7",
    soft: "rgba(203, 166, 247, 0.18)",
    border: "rgba(203, 166, 247, 0.34)",
    text: "#f5e0ff",
  },
  blue: {
    accent: "#89b4fa",
    soft: "rgba(137, 180, 250, 0.18)",
    border: "rgba(137, 180, 250, 0.34)",
    text: "#dbeafe",
  },
  green: {
    accent: "#a6e3a1",
    soft: "rgba(166, 227, 161, 0.18)",
    border: "rgba(166, 227, 161, 0.34)",
    text: "#dcfce7",
  },
  orange: {
    accent: "#fab387",
    soft: "rgba(250, 179, 135, 0.18)",
    border: "rgba(250, 179, 135, 0.34)",
    text: "#ffedd5",
  },
  pink: {
    accent: "#f5c2e7",
    soft: "rgba(245, 194, 231, 0.18)",
    border: "rgba(245, 194, 231, 0.34)",
    text: "#fce7f3",
  },
  graphite: {
    accent: "#a6adc8",
    soft: "rgba(166, 173, 200, 0.18)",
    border: "rgba(166, 173, 200, 0.34)",
    text: "#e5e7eb",
  },
};

function readStored<T extends string>(
  key: string,
  allowed: readonly T[],
  fallback: T,
): T {
  const value = window.localStorage.getItem(key) as T | null;
  return value && allowed.includes(value) ? value : fallback;
}

function readStoredZoom(): ZoomLevel {
  const value = window.localStorage.getItem("nodalix-files.zoom");
  const migrated =
    window.localStorage.getItem("nodalix-files.zoom.v2") === "true";
  if (
    migrated &&
    (value === "small" ||
      value === "medium" ||
      value === "large" ||
      value === "xlarge")
  ) {
    return value;
  }
  window.localStorage.setItem("nodalix-files.zoom.v2", "true");
  if (value === "medium") return "small";
  if (value === "xlarge") return "medium";
  return "medium";
}

function readStoredViewMode(): ViewMode {
  const value = window.localStorage.getItem("nodalix-files.viewMode");
  return value === "tree" ? "tree" : "grid";
}

function pathLabel(path: string) {
  return path.replace(/\/+$/, "").split("/").pop() || path;
}

function uniqueName(
  existingEntries: FileEntry[],
  base: string,
  extension = "",
) {
  const names = new Set(existingEntries.map((entry) => entry.name));
  const first = `${base}${extension}`;
  if (!names.has(first)) return first;
  for (let i = 2; i < 10_000; i += 1) {
    const candidate = `${base} ${i}${extension}`;
    if (!names.has(candidate)) return candidate;
  }
  return `${base} ${Date.now()}${extension}`;
}

function fileUri(path: string) {
  return `file://${path.split("/").map(encodeURIComponent).join("/")}`;
}

function isSameOrDescendantPath(path: string, target: string) {
  const normalizedPath = normalizePath(path);
  const normalizedTarget = normalizePath(target);
  return (
    normalizedPath === normalizedTarget ||
    normalizedTarget.startsWith(`${normalizedPath}/`)
  );
}

function dragPathsFromTransfer(dataTransfer: DataTransfer) {
  const raw = dataTransfer.getData("application/x-nodalix-file-paths");
  if (raw) {
    try {
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed)) {
        return parsed.filter((path): path is string => typeof path === "string");
      }
    } catch {
      return [];
    }
  }
  return dataTransfer
    .getData("text/plain")
    .split("\n")
    .map((path) => path.trim())
    .filter(Boolean);
}

function typeSortValue(entry: FileEntry, specialDirs: SpecialDirs | null) {
  if (entry.is_dir) return getFileKind(entry, specialDirs);
  return entry.name.includes(".")
    ? (entry.name.split(".").pop()?.toLowerCase() ?? "")
    : "";
}

function useDebouncedValue<T>(value: T, delayMs: number): T {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const timer = window.setTimeout(() => setDebounced(value), delayMs);
    return () => window.clearTimeout(timer);
  }, [value, delayMs]);
  return debounced;
}

export default function App() {
  const [ready, setReady] = useState(false);
  const [home, setHome] = useState("");
  const [platform, setPlatform] = useState<PlatformInfo | null>(null);
  const [localsendAvailable, setLocalsendAvailable] = useState(false);
  const [localsendDevices, setLocalsendDevices] = useState<LocalSendDevice[]>(
    [],
  );
  const [localsendScanning, setLocalsendScanning] = useState(false);
  const [localsendSendingId, setLocalsendSendingId] = useState<string | null>(
    null,
  );
  const localsendScanningRef = useRef(false);
  const [folderStyles, setFolderStyles] = useState<Record<string, FolderStyle>>(
    {},
  );
  const [specialDirs, setSpecialDirs] = useState<SpecialDirs | null>(null);
  const [tabs, setTabs] = useState<Tab[]>([]);
  const [activeTabId, setActiveTabId] = useState("");
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(new Set());
  const [visibleEntriesForSelection, setVisibleEntriesForSelection] = useState<
    FileEntry[]
  >([]);
  const [lastSelectedPath, setLastSelectedPath] = useState<string | null>(null);
  const [draggingPaths, setDraggingPaths] = useState<Set<string>>(new Set());
  const draggingPathsRef = useRef<string[]>([]);
  const dragHandledRef = useRef(false);
  const lastOpenRef = useRef<{ path: string; at: number } | null>(null);
  const hoverNavigateTimerRef = useRef<number | null>(null);
  const hoverNavigateTargetRef = useRef<string | null>(null);
  const hoverNavigatedDropDirRef = useRef<string | null>(null);
  const cwdRef = useRef("");
  const [dropTargetPath, setDropTargetPath] = useState<string | null>(null);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(
    () =>
      window.localStorage.getItem("nodalix-files.sidebarCollapsed") === "true",
  );
  const [showHiddenFiles, setShowHiddenFiles] = useState(
    () =>
      window.localStorage.getItem("nodalix-files.showHiddenFiles") === "true",
  );
  const [accentName, setAccentName] = useState<AccentName>(() =>
    readStored("nodalix-files.accent", accentNames, "purple"),
  );
  const [modal, setModal] = useState<ModalKind>(null);
  const [modalTarget, setModalTarget] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [renameInitial, setRenameInitial] = useState("");
  const [inlineRenamePath, setInlineRenamePath] = useState<string | null>(null);
  const [inlineRenameInitial, setInlineRenameInitial] = useState<string | null>(
    null,
  );
  const [searchQuery, setSearchQuery] = useState("");
  const [virtualEntries, setVirtualEntries] = useState<FileEntry[] | null>(
    null,
  );
  const [virtualError, setVirtualError] = useState<string | null>(null);
  const debouncedSearchQuery = useDebouncedValue(searchQuery, 90);
  const [sortKey, setSortKey] = useState<SortKey>(() =>
    readStored("nodalix-files.sortKey", sortKeys, "name"),
  );
  const [sortDirection, setSortDirection] = useState<SortDirection>(() =>
    readStored("nodalix-files.sortDirection", ["asc", "desc"], "asc"),
  );
  const [filterType, setFilterType] = useState<FilterType>(() =>
    readStored("nodalix-files.filterType", ["all", "folders", "files"], "all"),
  );
  const [filtersOpen, setFiltersOpen] = useState(false);
  const filtersRef = useRef<HTMLDivElement>(null);
  const filtersButtonRef = useRef<HTMLButtonElement>(null);
  const [networkConnectOpen, setNetworkConnectOpen] = useState(false);
  const networkConnectRef = useRef<HTMLDivElement>(null);
  const networkConnectButtonRef = useRef<HTMLButtonElement>(null);
  const [networkForm, setNetworkForm] = useState({
    protocol: "smb",
    host: "",
    share: "",
    username: "",
    password: "",
  });
  const [viewMode, setViewMode] = useState<ViewMode>(() => readStoredViewMode());
  const [zoom, setZoom] = useState<ZoomLevel>(() => readStoredZoom());

  const { menu, open: openMenu, close: closeMenu } = useContextMenu();
  const {
    items: sidebarItems,
    setItems: setSidebarItems,
    pin,
    unpin,
    hideBuiltin,
    renameAccess,
    setIcon: setSidebarItemIcon,
    removeAccess,
  } = useSidebarItems();
  const { entries, loading, error, load, preload, invalidate, setError } =
    useDirectory();

  const activeTab = useMemo(
    () => tabs.find((t) => t.id === activeTabId) ?? tabs[0],
    [tabs, activeTabId],
  );

  const cwd = activeTab?.path ?? home;
  useEffect(() => {
    cwdRef.current = cwd;
  }, [cwd]);
  const isDisksView = cwd === DISKS_PATH;
  const isNetworkView = cwd === NETWORK_PATH;
  const isVirtualView = isDisksView || isNetworkView;
  const currentIsTrash = isTrashPath(cwd, specialDirs);
  const pathLabels = useMemo(
    () => (specialDirs?.trash ? { [specialDirs.trash]: "Papelera" } : {}),
    [specialDirs?.trash],
  );

  const refreshDir = useCallback(
    async (path: string) => {
      invalidate(path);
      return load(path);
    },
    [invalidate, load],
  );

  const actions = useFileActions(refreshDir);
  const { mark } = usePerformanceMarks(platform?.debug ?? false);
  const accent = accentPresets[accentName];
  const accentOptions = useMemo(
    () =>
      accentNames.map((name) => ({
        id: name,
        label: accentLabels[name],
        color: accentPresets[name].accent,
      })),
    [],
  );
  const appStyle = useMemo(
    () =>
      ({
        "--nx-accent": accent.accent,
        "--nx-accent-soft": accent.soft,
        "--nx-accent-border": accent.border,
        "--nx-accent-text": accent.text,
        "--color-nodalix-accent": accent.accent,
        "--color-nodalix-accent-dim": accent.soft,
        "--color-nodalix-accent-strong": accent.border,
        "--nx-sidebar-icon-color": `color-mix(in srgb, ${accent.accent} 42%, rgba(210,216,226,0.78))`,
        "--nx-sidebar-icon-hover-color": `color-mix(in srgb, ${accent.accent} 50%, rgba(235,238,245,0.92))`,
        "--nx-sidebar-icon-active-color": `color-mix(in srgb, ${accent.accent} 62%, rgba(255,255,255,0.96))`,
        "--nx-sidebar-pinned-icon-color": `color-mix(in srgb, ${accent.accent} 58%, rgba(218,224,236,0.84))`,
      }) as CSSProperties,
    [accent],
  );

  useEffect(() => {
    window.localStorage.setItem("nodalix-files.sortKey", sortKey);
  }, [sortKey]);

  useEffect(() => {
    window.localStorage.setItem("nodalix-files.sortDirection", sortDirection);
  }, [sortDirection]);

  useEffect(() => {
    window.localStorage.setItem("nodalix-files.filterType", filterType);
  }, [filterType]);

  useEffect(() => {
    window.localStorage.setItem("nodalix-files.viewMode", viewMode);
  }, [viewMode]);

  useEffect(() => {
    window.localStorage.setItem("nodalix-files.zoom", zoom);
  }, [zoom]);

  useEffect(() => {
    window.localStorage.setItem(
      "nodalix-files.sidebarCollapsed",
      String(sidebarCollapsed),
    );
  }, [sidebarCollapsed]);

  useEffect(() => {
    window.localStorage.setItem(
      "nodalix-files.showHiddenFiles",
      String(showHiddenFiles),
    );
  }, [showHiddenFiles]);

  useEffect(() => {
    window.localStorage.setItem("nodalix-files.accent", accentName);
  }, [accentName]);

  useEffect(() => {
    perfMark("react_mount");
    startupBundle()
      .then((bundle) => {
        setHome(bundle.home);
        setPlatform(bundle.platform);
        setSidebarItems(bundle.sidebar_items);
        setLocalsendAvailable(bundle.localsend_available);
        setFolderStyles(bundle.folder_customizations);
        setSpecialDirs(bundle.special_dirs);
        const tab = createTab(bundle.initial_path ?? bundle.home);
        setTabs([tab]);
        setActiveTabId(tab.id);
        setReady(true);
        mark("startup_bundle_done");
        perfMark("window_content_ready");
      })
      .catch((e) => setError(e instanceof Error ? e.message : String(e)));
  }, [mark, setSidebarItems, setError]);

  const refreshLocalSendDevices = useCallback(async () => {
    if (!localsendAvailable || localsendScanningRef.current) return;
    localsendScanningRef.current = true;
    setLocalsendScanning(true);
    try {
      const devices = await discoverLocalSendDevices();
      setLocalsendDevices(devices);
    } catch (e) {
      setActionError(e instanceof Error ? e.message : String(e));
      setLocalsendDevices([]);
    } finally {
      localsendScanningRef.current = false;
      setLocalsendScanning(false);
    }
  }, [localsendAvailable]);

  useEffect(() => {
    if (!localsendAvailable) {
      setLocalsendDevices([]);
      return;
    }
    const timer = window.setTimeout(() => {
      void refreshLocalSendDevices();
    }, 900);
    return () => window.clearTimeout(timer);
  }, [localsendAvailable, refreshLocalSendDevices]);

  useEffect(() => {
    if (!ready || !activeTab) return;
    setSelectedPaths(new Set());
    setLastSelectedPath(null);
    setSearchQuery("");
    if (activeTab.path === DISKS_PATH || activeTab.path === NETWORK_PATH) {
      setVirtualError(null);
      const loader =
        activeTab.path === DISKS_PATH ? listDisks : listNetworkLocations;
      loader()
        .then((items) => setVirtualEntries(items))
        .catch((e) => {
          setVirtualEntries([]);
          setVirtualError(e instanceof Error ? e.message : String(e));
        });
    } else {
      setVirtualEntries(null);
      setVirtualError(null);
      void load(activeTab.path);
    }
    mark(`navigate ${activeTab.path}`);
  }, [activeTab?.path, ready, load, mark]);

  useEffect(() => {
    if (!ready || !activeTab) return;
    const refresh = () => {
      if (activeTab.path === DISKS_PATH || activeTab.path === NETWORK_PATH) {
        const loader =
          activeTab.path === DISKS_PATH ? listDisks : listNetworkLocations;
        void loader()
          .then((items) => setVirtualEntries(items))
          .catch((e) => {
            setVirtualEntries([]);
            setVirtualError(e instanceof Error ? e.message : String(e));
          });
        return;
      }
      invalidate(activeTab.path);
      void load(activeTab.path, { silent: true });
    };
    const timer = window.setInterval(refresh, 5000);
    return () => window.clearInterval(timer);
  }, [activeTab?.path, invalidate, load, ready]);

  useEffect(() => {
    if (!ready) return;
    const paths = sidebarItems
      .filter(
        (item) =>
          item.custom &&
          item.is_dir &&
          !item.path.startsWith("nodalix://") &&
          item.kind !== "trash",
      )
      .map((item) => item.path);
    if (paths.length === 0) return;
    const timer = window.setTimeout(() => {
      for (const [index, path] of paths.slice(0, 4).entries()) {
        window.setTimeout(() => void preload(path), index * 180);
      }
    }, 450);
    return () => window.clearTimeout(timer);
  }, [preload, ready, sidebarItems]);

  useEffect(() => {
    if (!filtersOpen) return;
    const isInsideFilters = (target: EventTarget | null) => {
      if (!(target instanceof Node)) return false;
      return (
        filtersButtonRef.current?.contains(target) ||
        filtersRef.current?.contains(target)
      );
    };
    const onPointerDown = (event: PointerEvent) => {
      if (!isInsideFilters(event.target)) setFiltersOpen(false);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        setFiltersOpen(false);
      }
    };
    document.addEventListener("pointerdown", onPointerDown, true);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown, true);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [filtersOpen]);

  useEffect(() => {
    if (!networkConnectOpen) return;
    const isInsidePanel = (target: EventTarget | null) => {
      if (!(target instanceof Node)) return false;
      return (
        networkConnectButtonRef.current?.contains(target) ||
        networkConnectRef.current?.contains(target)
      );
    };
    const onPointerDown = (event: PointerEvent) => {
      if (!isInsidePanel(event.target)) setNetworkConnectOpen(false);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        setNetworkConnectOpen(false);
      }
    };
    document.addEventListener("pointerdown", onPointerDown, true);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown, true);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [networkConnectOpen]);

  const displayedEntries = useMemo(() => {
    const q = debouncedSearchQuery.trim().toLowerCase();
    const sourceEntries = virtualEntries ?? entries;
    // Linux hidden entries are dot-prefixed; hidden entries are removed from the visible model, not only styled.
    const visibleEntries =
      showHiddenFiles || virtualEntries
        ? sourceEntries
        : sourceEntries.filter((entry) => !isHiddenEntry(entry));
    const filtered = q
      ? visibleEntries.filter((entry) => entry.name.toLowerCase().includes(q))
      : visibleEntries;
    const typed =
      filterType === "folders"
        ? filtered.filter((entry) => entry.is_dir)
        : filterType === "files"
          ? filtered.filter((entry) => !entry.is_dir)
          : filtered;
    const direction = sortDirection === "asc" ? 1 : -1;
    return [...typed].sort((a, b) => {
      if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
      let result = 0;
      switch (sortKey) {
        case "name":
          result = a.name.localeCompare(b.name, undefined, {
            sensitivity: "base",
            numeric: true,
          });
          break;
        case "modified":
          result = (a.modified ?? 0) - (b.modified ?? 0);
          break;
        case "created":
          // Linux birth/creation time can be unavailable; backend/front-end fall back to modified time.
          result =
            (a.created ?? a.modified ?? 0) - (b.created ?? b.modified ?? 0);
          break;
        case "type":
          result = typeSortValue(a, specialDirs).localeCompare(
            typeSortValue(b, specialDirs),
          );
          break;
        case "size":
          result = a.size - b.size;
          break;
      }
      return result === 0
        ? a.name.localeCompare(b.name, undefined, {
            sensitivity: "base",
            numeric: true,
          })
        : result * direction;
    });
  }, [
    debouncedSearchQuery,
    entries,
    filterType,
    showHiddenFiles,
    sortDirection,
    sortKey,
    specialDirs,
    virtualEntries,
  ]);

  useEffect(() => {
    if (showHiddenFiles) return;
    const visiblePaths = new Set(displayedEntries.map((entry) => entry.path));
    setSelectedPaths((prev) => {
      const next = new Set([...prev].filter((path) => visiblePaths.has(path)));
      return next.size === prev.size ? prev : next;
    });
    setLastSelectedPath((path) =>
      path && visiblePaths.has(path) ? path : null,
    );
  }, [displayedEntries, showHiddenFiles]);

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
    (path: string) =>
      updateActiveTab((tab) => navigateTab(tab, normalizePath(path))),
    [updateActiveTab],
  );

  useEffect(() => {
    if (!ready) return;
    let unlisten: (() => void) | undefined;
    void listen<string>("nodalix-open-path", (event) => {
      if (event.payload) goTo(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, [goTo, ready]);

  const handleOpen = useCallback(
    async (entry: FileEntry) => {
      const now = performance.now();
      const lastOpen = lastOpenRef.current;
      if (lastOpen?.path === entry.path && now - lastOpen.at < 260) return;
      lastOpenRef.current = { path: entry.path, at: now };
      if (entry.is_dir) {
        if (entry.path.startsWith("nodalix://")) return;
        if (normalizePath(entry.path) === normalizePath(cwdRef.current)) return;
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

  const preloadEntry = useCallback(
    (entry: FileEntry) => {
      if (entry.is_dir && !entry.path.startsWith("nodalix://")) {
        void preload(entry.path);
      }
    },
    [preload],
  );

  const getRangeSelection = useCallback(
    (anchorPath: string, targetPath: string) => {
      const orderedEntries = visibleEntriesForSelection.length
        ? visibleEntriesForSelection
        : displayedEntries;
      const from = orderedEntries.findIndex(
        (entry) => entry.path === anchorPath,
      );
      const to = orderedEntries.findIndex((entry) => entry.path === targetPath);
      if (from === -1 || to === -1) return [targetPath];
      const [start, end] = from < to ? [from, to] : [to, from];
      return orderedEntries.slice(start, end + 1).map((entry) => entry.path);
    },
    [displayedEntries, visibleEntriesForSelection],
  );

  const handleSelect = useCallback(
    (entry: FileEntry, additive: boolean, range: boolean) => {
      setSelectedPaths((prev) => {
        if (range && lastSelectedPath) {
          return new Set(getRangeSelection(lastSelectedPath, entry.path));
        }
        if (!additive) return new Set([entry.path]);
        const next = new Set(prev);
        if (next.has(entry.path)) next.delete(entry.path);
        else next.add(entry.path);
        return next;
      });
      setLastSelectedPath(entry.path);
      preloadEntry(entry);
    },
    [getRangeSelection, lastSelectedPath, preloadEntry],
  );

  const handleBoxSelect = useCallback((paths: string[], additive: boolean) => {
    setSelectedPaths((prev) => {
      const next = new Set(additive ? prev : []);
      for (const path of paths) next.add(path);
      return next;
    });
    if (paths.length > 0) setLastSelectedPath(paths[paths.length - 1]);
  }, []);

  const handleBackgroundClick = useCallback((preserveSelection: boolean) => {
    if (preserveSelection) return;
    setSelectedPaths(new Set());
    setLastSelectedPath(null);
  }, []);

  const createNewFolder = useCallback(async () => {
    try {
      setSearchQuery("");
      setFilterType("all");
      const name = uniqueName(entries, "Nueva carpeta");
      const created = await actions.newFolder(cwd, name);
      const createdPath = normalizePath(created);
      setSelectedPaths(new Set([createdPath]));
      setInlineRenamePath(createdPath);
      setInlineRenameInitial(name);
      setActionError(null);
    } catch (e) {
      setActionError(e instanceof Error ? e.message : String(e));
    }
  }, [actions, cwd, entries]);

  const createNewDocument = useCallback(async () => {
    try {
      setSearchQuery("");
      setFilterType("all");
      const name = uniqueName(entries, "Documento sin título", ".txt");
      const created = await actions.newDocument(cwd, name);
      const createdPath = normalizePath(created);
      setSelectedPaths(new Set([createdPath]));
      setInlineRenamePath(createdPath);
      setInlineRenameInitial(name);
      setActionError(null);
    } catch (e) {
      setActionError(e instanceof Error ? e.message : String(e));
    }
  }, [actions, cwd, entries]);

  const submitNetworkConnection = useCallback(
    async (event: FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      try {
        await connectNetworkLocation({
          protocol: networkForm.protocol,
          host: networkForm.host,
          share: networkForm.share,
          username: networkForm.username || undefined,
          password: networkForm.password || undefined,
        });
        setActionError(null);
        setNetworkConnectOpen(false);
        const items = await listNetworkLocations();
        setVirtualEntries(items);
        setVirtualError(null);
      } catch (e) {
        setActionError(e instanceof Error ? e.message : String(e));
      }
    },
    [networkForm],
  );

  const pathsForAction = useCallback(() => {
    if (selectedPaths.size > 0) return [...selectedPaths];
    return modalTarget ? [modalTarget] : [];
  }, [selectedPaths, modalTarget]);

  const startInlineRename = useCallback((entry: FileEntry) => {
    setInlineRenamePath(entry.path);
    setInlineRenameInitial(entry.name);
    setSelectedPaths(new Set([entry.path]));
    setLastSelectedPath(entry.path);
    setActionError(null);
  }, []);

  const commitInlineRename = useCallback(
    async (entry: FileEntry, name: string) => {
      try {
        await actions.rename(entry.path, name, cwd);
        setSelectedPaths(new Set());
        setInlineRenamePath(null);
        setInlineRenameInitial(null);
        setActionError(null);
        return true;
      } catch (e) {
        setActionError(e instanceof Error ? e.message : String(e));
        return false;
      }
    },
    [actions, cwd],
  );

  const cancelInlineRename = useCallback(() => {
    setInlineRenamePath(null);
    setInlineRenameInitial(null);
  }, []);

  const moveSelectionToTrash = useCallback(
    async (paths: string[]) => {
      try {
        closeMenu();
        await actions.trash(paths, cwd);
        setSelectedPaths(new Set());
        setActionError(null);
      } catch (e) {
        setActionError(e instanceof Error ? e.message : String(e));
      }
    },
    [actions, closeMenu, cwd],
  );

  const requestPermanentDelete = useCallback(
    (paths: string[]) => {
      closeMenu();
      setModalTarget(paths[0] ?? null);
      setSelectedPaths(new Set(paths));
      setModal("confirmPermanentDelete");
    },
    [closeMenu],
  );

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (
        target?.closest("input, textarea, select, [contenteditable='true']")
      ) {
        return;
      }
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "q") {
        event.preventDefault();
        void quitApp();
        return;
      }
      if (event.key === "Escape") {
        closeMenu();
        setSelectedPaths(new Set());
        setLastSelectedPath(null);
        return;
      }
      if (event.key === "Enter") {
        const primaryPath = lastSelectedPath ?? [...selectedPaths][0];
        const entry = displayedEntries.find(
          (item) => item.path === primaryPath,
        );
        if (entry) {
          event.preventDefault();
          void handleOpen(entry);
        }
        return;
      }
      if (event.key !== "Delete" || selectedPaths.size === 0) return;
      event.preventDefault();
      const paths = [...selectedPaths];
      if (currentIsTrash) requestPermanentDelete(paths);
      else void moveSelectionToTrash(paths);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [
    closeMenu,
    currentIsTrash,
    displayedEntries,
    handleOpen,
    lastSelectedPath,
    moveSelectionToTrash,
    requestPermanentDelete,
    selectedPaths,
  ]);

  const handleDragStart = useCallback(
    (entry: FileEntry, event: React.DragEvent<HTMLElement>) => {
      const paths = selectedPaths.has(entry.path)
        ? [...selectedPaths]
        : [entry.path];
      if (!selectedPaths.has(entry.path)) {
        setSelectedPaths(new Set([entry.path]));
        setLastSelectedPath(entry.path);
      }
      draggingPathsRef.current = paths;
      dragHandledRef.current = false;
      setDraggingPaths(new Set(paths));
      setDropTargetPath(null);
      event.dataTransfer.effectAllowed = "copyMove";
      event.dataTransfer.setData("text/plain", paths.join("\n"));
      event.dataTransfer.setData(
        "text/uri-list",
        paths.map(fileUri).join("\n"),
      );
      event.dataTransfer.setData(
        "application/x-nodalix-file-paths",
        JSON.stringify(paths),
      );

      const preview = document.createElement("div");
      const icon = event.currentTarget.querySelector("svg")?.cloneNode(true);
      if (icon instanceof SVGElement) {
        icon.setAttribute("class", "h-14 w-14 drop-shadow-md");
        preview.appendChild(icon);
      }
      if (paths.length > 1) {
        const badge = document.createElement("span");
        badge.textContent = String(paths.length);
        badge.className =
          "absolute -right-1 -top-1 rounded-full bg-[rgba(24,24,37,0.96)] px-1.5 py-0.5 text-[10px] font-semibold text-nodalix-text shadow-lg";
        preview.appendChild(badge);
      }
      preview.className =
        "fixed -left-[9999px] top-0 flex h-16 w-16 items-center justify-center rounded-xl border border-nodalix-border bg-[rgba(24,24,37,0.9)] p-1.5 shadow-2xl";
      document.body.appendChild(preview);
      event.dataTransfer.setDragImage(preview, 28, 28);
      window.setTimeout(() => preview.remove(), 0);
    },
    [selectedPaths],
  );

  const canDropOnFolder = useCallback(
    (entry: FileEntry, paths: string[]) =>
      entry.is_dir &&
      paths.length > 0 &&
      paths.every((path) => !isSameOrDescendantPath(path, entry.path)),
    [],
  );

  const getDraggedPaths = useCallback(
    (event: React.DragEvent<HTMLElement>) => {
      if (draggingPathsRef.current.length > 0) return draggingPathsRef.current;
      if (draggingPaths.size > 0) return [...draggingPaths];
      return dragPathsFromTransfer(event.dataTransfer);
    },
    [draggingPaths],
  );

  const getDraggedPathsFromTransfer = useCallback(
    (dataTransfer: DataTransfer) => {
      if (draggingPathsRef.current.length > 0) return draggingPathsRef.current;
      if (draggingPaths.size > 0) return [...draggingPaths];
      return dragPathsFromTransfer(dataTransfer);
    },
    [draggingPaths],
  );

  const entryForDragTarget = useCallback(
    (path: string) =>
      visibleEntriesForSelection.find((entry) => entry.path === path) ??
      displayedEntries.find((entry) => entry.path === path),
    [displayedEntries, visibleEntriesForSelection],
  );

  const clearDragState = useCallback(() => {
    if (hoverNavigateTimerRef.current) {
      window.clearTimeout(hoverNavigateTimerRef.current);
      hoverNavigateTimerRef.current = null;
    }
    hoverNavigateTargetRef.current = null;
    hoverNavigatedDropDirRef.current = null;
    draggingPathsRef.current = [];
    setDraggingPaths(new Set());
    setDropTargetPath(null);
  }, []);

  const copyPath = useCallback(async (path: string) => {
    try {
      await copyTextToClipboard(path);
      setActionError(null);
    } catch (e) {
      setActionError(e instanceof Error ? e.message : `No se pudo copiar la ruta: ${path}`);
    }
  }, []);

  const showPathContextMenu = useCallback(
    (path: string, x: number, y: number) => {
      openMenu(
        x,
        y,
        <>
          <ContextMenuItem
            label="Copiar ruta"
            onClick={() => {
              closeMenu();
              void copyPath(path);
            }}
          />
        </>,
      );
    },
    [closeMenu, copyPath, openMenu],
  );

  const moveDraggedPaths = useCallback(
    async (paths: string[], targetDir: string) => {
      await actions.move(paths, targetDir, cwdRef.current || cwd);
      setSelectedPaths(new Set());
      setLastSelectedPath(null);
      setActionError(null);
    },
    [actions, cwd],
  );

  const canDropOnPath = useCallback(
    (targetPath: string, paths: string[]) =>
      !targetPath.startsWith("nodalix://") &&
      paths.length > 0 &&
      paths.every((path) => !isSameOrDescendantPath(path, targetPath)),
    [],
  );

  const scheduleHoverNavigate = useCallback(
    (targetPath: string) => {
      if (
        !draggingPathsRef.current.length ||
        targetPath.startsWith("nodalix://")
      ) {
        return;
      }
      if (
        hoverNavigateTargetRef.current === targetPath &&
        hoverNavigateTimerRef.current
      ) {
        return;
      }
      if (hoverNavigateTimerRef.current) {
        window.clearTimeout(hoverNavigateTimerRef.current);
      }
      hoverNavigateTargetRef.current = targetPath;
      hoverNavigateTimerRef.current = window.setTimeout(() => {
        const paths = draggingPathsRef.current;
        if (!paths.length || !canDropOnPath(targetPath, paths)) return;
        hoverNavigateTimerRef.current = null;
        hoverNavigateTargetRef.current = null;
        hoverNavigatedDropDirRef.current = targetPath;
        setDropTargetPath(null);
        goTo(targetPath);
      }, 900);
    },
    [canDropOnPath, goTo],
  );

  const cancelHoverNavigate = useCallback((targetPath?: string) => {
    if (targetPath && hoverNavigateTargetRef.current !== targetPath) return;
    if (!hoverNavigateTimerRef.current) return;
    window.clearTimeout(hoverNavigateTimerRef.current);
    hoverNavigateTimerRef.current = null;
    hoverNavigateTargetRef.current = null;
  }, []);

  const handleDropOnPath = useCallback(
    async (targetPath: string, dataTransfer: DataTransfer) => {
      const paths = getDraggedPathsFromTransfer(dataTransfer);
      if (!canDropOnPath(targetPath, paths)) return;
      dragHandledRef.current = true;
      try {
        await moveDraggedPaths(paths, targetPath);
      } catch (e) {
        setActionError(e instanceof Error ? e.message : String(e));
      } finally {
        clearDragState();
      }
    },
    [canDropOnPath, clearDragState, getDraggedPathsFromTransfer, moveDraggedPaths],
  );

  const handleDropToCurrentDirectory = useCallback(
    async (event: React.DragEvent<HTMLElement>) => {
      event.preventDefault();
      event.stopPropagation();
      const paths = getDraggedPaths(event);
      const targetDir = cwdRef.current || cwd;
      if (!canDropOnPath(targetDir, paths)) return;
      dragHandledRef.current = true;
      try {
        await moveDraggedPaths(paths, targetDir);
      } catch (e) {
        setActionError(e instanceof Error ? e.message : String(e));
      } finally {
        clearDragState();
      }
    },
    [canDropOnPath, clearDragState, cwd, getDraggedPaths, moveDraggedPaths],
  );

  const pinDraggedPaths = useCallback(
    async (paths: string[]) => {
      for (const path of paths) {
        await pin(path, pathLabel(path));
      }
      setActionError(null);
    },
    [pin],
  );

  const handleDragOverFolder = useCallback(
    (entry: FileEntry, event: React.DragEvent<HTMLElement>) => {
      const paths = getDraggedPaths(event);
      if (!canDropOnFolder(entry, paths)) {
        event.dataTransfer.dropEffect = "none";
        return;
      }
      event.preventDefault();
      event.dataTransfer.dropEffect = "move";
      if (dropTargetPath !== entry.path) setDropTargetPath(entry.path);
      scheduleHoverNavigate(entry.path);
    },
    [canDropOnFolder, dropTargetPath, getDraggedPaths, scheduleHoverNavigate],
  );

  const handleDragEnterFolder = useCallback(
    (entry: FileEntry, event: React.DragEvent<HTMLElement>) => {
      const paths = getDraggedPaths(event);
      if (!canDropOnFolder(entry, paths)) return;
      event.preventDefault();
      setDropTargetPath(entry.path);
      scheduleHoverNavigate(entry.path);
    },
    [canDropOnFolder, getDraggedPaths, scheduleHoverNavigate],
  );

  const handleDragLeaveFolder = useCallback(
    (entry: FileEntry, event: React.DragEvent<HTMLElement>) => {
      if (
        event.currentTarget.contains(event.relatedTarget as Node | null)
      ) {
        return;
      }
      cancelHoverNavigate(entry.path);
      setDropTargetPath((path) => (path === entry.path ? null : path));
    },
    [cancelHoverNavigate],
  );

  const handleDropOnFolder = useCallback(
    async (entry: FileEntry, event: React.DragEvent<HTMLElement>) => {
      event.preventDefault();
      event.stopPropagation();
      const paths = getDraggedPaths(event);
      if (!canDropOnFolder(entry, paths)) {
        setDropTargetPath(null);
        return;
      }
      dragHandledRef.current = true;
      try {
        await moveDraggedPaths(paths, entry.path);
      } catch (e) {
        setActionError(e instanceof Error ? e.message : String(e));
      } finally {
        clearDragState();
      }
    },
    [canDropOnFolder, clearDragState, getDraggedPaths, moveDraggedPaths],
  );

  const sendDraggedPathsToLocalSendDevice = useCallback(
    async (paths: string[], device: LocalSendDevice) => {
      if (paths.length === 0) return;
      dragHandledRef.current = true;
      setLocalsendSendingId(device.id);
      try {
        await sendToLocalSendDevice(paths, device);
        setActionError(null);
      } catch (e) {
        setActionError(e instanceof Error ? e.message : String(e));
      } finally {
        setLocalsendSendingId(null);
      }
    },
    [],
  );

  const handleDragEnd = useCallback(
    async (event: React.DragEvent<HTMLElement>) => {
      const paths = getDraggedPaths(event);
      try {
        if (!dragHandledRef.current && paths.length > 0) {
          const target = document.elementFromPoint(
            event.clientX,
            event.clientY,
          ) as HTMLElement | null;
          const item = target?.closest<HTMLElement>("[data-file-item='true']");
          const localsendTarget = target?.closest<HTMLElement>(
            "[data-localsend-device-id]",
          );
          const localsendDeviceId = localsendTarget?.dataset.localsendDeviceId;
          const localsendDevice = localsendDeviceId
            ? localsendDevices.find((device) => device.id === localsendDeviceId)
            : undefined;
          if (localsendDevice) {
            await sendDraggedPathsToLocalSendDevice(paths, localsendDevice);
            return;
          }
          const path = item?.dataset.filePath;
          const entry = path ? entryForDragTarget(path) : undefined;
          if (entry?.is_dir && canDropOnFolder(entry, paths)) {
            await moveDraggedPaths(paths, entry.path);
            return;
          }
          const fileArea = target?.closest<HTMLElement>("[data-file-area='true']");
          const targetDir = hoverNavigatedDropDirRef.current ?? cwdRef.current;
          if (fileArea && targetDir && canDropOnPath(targetDir, paths)) {
            await moveDraggedPaths(paths, targetDir);
          }
        }
      } catch (e) {
        setActionError(e instanceof Error ? e.message : String(e));
      } finally {
        dragHandledRef.current = false;
        clearDragState();
      }
    },
    [
      canDropOnFolder,
      clearDragState,
      entryForDragTarget,
      getDraggedPaths,
      localsendDevices,
      moveDraggedPaths,
      sendDraggedPathsToLocalSendDevice,
    ],
  );

  const handleSidebarDrop = useCallback(
    async (dataTransfer: DataTransfer) => {
      const paths = draggingPathsRef.current.length
        ? draggingPathsRef.current
        : dragPathsFromTransfer(dataTransfer);
      if (paths.length === 0) return;
      dragHandledRef.current = true;
      try {
        await pinDraggedPaths(paths);
      } catch (e) {
        setActionError(e instanceof Error ? e.message : String(e));
      } finally {
        clearDragState();
      }
    },
    [clearDragState, pinDraggedPaths],
  );

  const handleSidebarItemDrop = useCallback(
    (item: SidebarItem, dataTransfer: DataTransfer) => {
      void handleDropOnPath(item.path, dataTransfer);
    },
    [handleDropOnPath],
  );

  const handleLocalSendDeviceDrop = useCallback(
    async (device: LocalSendDevice, dataTransfer: DataTransfer) => {
      const paths = draggingPathsRef.current.length
        ? draggingPathsRef.current
        : dragPathsFromTransfer(dataTransfer);
      await sendDraggedPathsToLocalSendDevice(paths, device);
      clearDragState();
    },
    [clearDragState, sendDraggedPathsToLocalSendDevice],
  );

  useEffect(() => {
    const clear = () => {
      dragHandledRef.current = false;
      clearDragState();
    };
    const onVisibilityChange = () => {
      if (document.visibilityState !== "visible") clear();
    };
    window.addEventListener("dragend", clear);
    window.addEventListener("drop", clear);
    window.addEventListener("blur", clear);
    document.addEventListener("visibilitychange", onVisibilityChange);
    return () => {
      window.removeEventListener("dragend", clear);
      window.removeEventListener("drop", clear);
      window.removeEventListener("blur", clear);
      document.removeEventListener("visibilitychange", onVisibilityChange);
    };
  }, [clearDragState]);

  useEffect(() => {
    if (draggingPaths.size === 0) return;
    const onWheel = (event: WheelEvent) => {
      const target = document.elementFromPoint(
        event.clientX,
        event.clientY,
      ) as HTMLElement | null;
      const scroller =
        target?.closest<HTMLElement>(".nodalix-content") ??
        document.querySelector<HTMLElement>(".nodalix-content");
      if (!scroller) return;
      event.preventDefault();
      scroller.scrollBy({
        top: event.deltaY,
        left: event.shiftKey ? event.deltaY : event.deltaX,
        behavior: "auto",
      });
    };
    window.addEventListener("wheel", onWheel, { passive: false });
    return () => window.removeEventListener("wheel", onWheel);
  }, [draggingPaths.size]);

  const showFileContextMenu = useCallback(
    (entry: FileEntry, x: number, y: number) => {
      const targets = selectedPaths.has(entry.path)
        ? [...selectedPaths]
        : [entry.path];
      setModalTarget(entry.path);
      if (!selectedPaths.has(entry.path)) {
        setSelectedPaths(new Set([entry.path]));
        setLastSelectedPath(entry.path);
      }

      openMenu(
        x,
        y,
        <>
          {targets.length > 1 && (
            <ContextMenuItem
              label={`${targets.length} elementos seleccionados`}
              disabled
            />
          )}
          {targets.length > 1 && <ContextMenuSeparator />}
          <ContextMenuItem
            label={targets.length > 1 ? `Abrir (${targets.length})` : "Abrir"}
            onClick={() => {
              closeMenu();
              void handleOpen(entry);
            }}
          />
          <ContextMenuSubmenu label="Abrir con…">
            <ContextMenuItem
              label="Elegir aplicación…"
              onClick={() => {
                closeMenu();
                setModal("openWith");
              }}
            />
          </ContextMenuSubmenu>
          <ContextMenuSeparator />
          <ContextMenuItem
            label="Renombrar"
            onClick={() => {
              closeMenu();
              startInlineRename(entry);
            }}
          />
          <ContextMenuItem
            label="Copiar"
            onClick={() => {
              closeMenu();
              void actions.copy(targets);
            }}
          />
          <ContextMenuItem
            label="Cortar"
            onClick={() => {
              closeMenu();
              void actions.cut(targets);
            }}
          />
          <ContextMenuItem
            label="Pegar"
            disabled={!actions.clipboardActive}
            onClick={() => {
              closeMenu();
              void actions.paste(cwd);
            }}
          />
          <ContextMenuItem
            label="Mover a…"
            onClick={() => {
              closeMenu();
              setModal("move");
            }}
          />
          <ContextMenuSeparator />
          {currentIsTrash ? (
            <ContextMenuDangerItem
              label="Eliminar permanentemente"
              onClick={() => requestPermanentDelete(targets)}
            />
          ) : (
            <ContextMenuItem
              label="Mover a la papelera"
              onClick={() => void moveSelectionToTrash(targets)}
            />
          )}
          {localsendAvailable && (
            <ContextMenuItem
              label="Enviar con LocalSend"
              onClick={() => {
                closeMenu();
                void actions.localSend(targets);
              }}
            />
          )}
          <ContextMenuItem
            label="Propiedades"
            onClick={() => {
              closeMenu();
              setModal("properties");
            }}
          />
          {entry.is_dir && (
            <>
              <ContextMenuSeparator />
              <ContextMenuSubmenu label="Apariencia">
                <ContextMenuItem
                  label="Cambiar color…"
                  onClick={() => {
                    closeMenu();
                    setModal("folderColor");
                  }}
                />
                <ContextMenuItem
                  label="Cambiar icono…"
                  onClick={() => {
                    closeMenu();
                    setModal("folderIcon");
                  }}
                />
              </ContextMenuSubmenu>
              <ContextMenuItem
                label="Fijar en la barra lateral"
                onClick={() => {
                  closeMenu();
                  void pin(entry.path, entry.name);
                }}
              />
              <ContextMenuItem
                label="Abrir terminal aquí"
                onClick={() => {
                  closeMenu();
                  void actions.terminalHere(entry.path);
                }}
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
      currentIsTrash,
      requestPermanentDelete,
      moveSelectionToTrash,
      startInlineRename,
    ],
  );

  const showSidebarContextMenu = useCallback(
    (item: SidebarItem, x: number, y: number) => {
      if (item.kind === "trash") {
        openMenu(
          x,
          y,
          <>
            <ContextMenuItem
              label="Abrir"
              onClick={() => {
                closeMenu();
                goTo(item.path);
              }}
            />
            <ContextMenuItem
              label="Propiedades"
              onClick={() => {
                closeMenu();
                setModalTarget(item.path);
                setModal("properties");
              }}
            />
          </>,
        );
        return;
      }

      openMenu(
        x,
        y,
        <>
          <ContextMenuItem label="Abrir" onClick={() => goTo(item.path)} />
          <ContextMenuItem
            label="Abrir en una ventana nueva"
            disabled
            title="Pendiente: soporte multi-ventana"
          />
          <ContextMenuSeparator />
          {item.pinned ? (
            <ContextMenuItem
              label="Quitar de la barra lateral"
              onClick={() =>
                void (item.custom ? unpin(item.path) : hideBuiltin(item.id))
              }
            />
          ) : (
            <ContextMenuItem
              label="Fijar en la barra lateral"
              onClick={() => void pin(item.path, item.label)}
            />
          )}
          {item.can_rename && (
            <ContextMenuItem
              label="Renombrar acceso"
              onClick={() => {
                closeMenu();
                setRenameInitial(item.label);
                setModalTarget(item.id);
                setModal("rename");
              }}
            />
          )}
          {item.custom && (
            <ContextMenuSubmenu label="Apariencia">
              <ContextMenuItem
                label="Cambiar icono…"
                onClick={() => {
                  closeMenu();
                  setModalTarget(item.id);
                  setModal("sidebarIcon");
                }}
              />
            </ContextMenuSubmenu>
          )}
          {item.can_remove && (
            <ContextMenuItem
              label="Eliminar de la barra lateral"
              onClick={() => void removeAccess(item.id)}
            />
          )}
          <ContextMenuSeparator />
          <ContextMenuItem
            label="Mostrar en carpeta"
            onClick={() => {
              const parent = parentPath(item.path);
              if (parent) goTo(parent);
            }}
          />
          <ContextMenuItem
            label="Propiedades"
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
          {currentIsTrash && entries.length > 0 && (
            <>
              <ContextMenuItem
                label="Vaciar papelera"
                onClick={() =>
                  requestPermanentDelete(entries.map((entry) => entry.path))
                }
              />
              <ContextMenuSeparator />
            </>
          )}
          <ContextMenuItem
            label="Seleccionar todo"
            disabled={displayedEntries.length === 0}
            onClick={() => {
              closeMenu();
              const paths = displayedEntries.map((entry) => entry.path);
              setSelectedPaths(new Set(paths));
              setLastSelectedPath(paths.at(-1) ?? null);
            }}
          />
          <ContextMenuSeparator />
          <ContextMenuItem
            label="Nueva carpeta"
            onClick={() => {
              closeMenu();
              void createNewFolder();
            }}
          />
          <ContextMenuItem
            label="Nuevo documento"
            onClick={() => {
              closeMenu();
              void createNewDocument();
            }}
          />
          <ContextMenuSeparator />
          <ContextMenuItem
            label="Pegar"
            disabled={!actions.clipboardActive}
            onClick={() => {
              closeMenu();
              void actions.paste(cwd);
            }}
          />
          <ContextMenuItem
            label="Abrir terminal aquí"
            onClick={() => {
              closeMenu();
              void actions.terminalHere(cwd);
            }}
          />
          <ContextMenuItem
            label="Actualizar"
            onClick={() => {
              closeMenu();
              void refreshDir(cwd);
            }}
          />
          <ContextMenuSubmenu label="Opciones de vista">
            <ContextMenuItem
              label="Cuadrícula"
              onClick={() => {
                closeMenu();
                setViewMode("grid");
              }}
            />
            <ContextMenuItem
              label="Árbol"
              onClick={() => {
                closeMenu();
                setViewMode("tree");
              }}
            />
          </ContextMenuSubmenu>
        </>,
      );
    },
    [
      openMenu,
      closeMenu,
      createNewFolder,
      createNewDocument,
      actions,
      cwd,
      currentIsTrash,
      displayedEntries,
      entries,
      refreshDir,
      requestPermanentDelete,
    ],
  );

  useEffect(() => {
    const preventNativeContextMenu = (event: MouseEvent) => {
      event.preventDefault();
    };
    document.addEventListener("contextmenu", preventNativeContextMenu, {
      capture: true,
    });
    return () => {
      document.removeEventListener("contextmenu", preventNativeContextMenu, {
        capture: true,
      });
    };
  }, []);

  if (!ready || !platform || !activeTab) {
    return (
      <div className="flex h-full flex-col bg-[#0b0b12]">
        <div className="flex flex-1 flex-col items-center justify-center gap-3">
          <div className="h-10 w-10 animate-pulse rounded-2xl bg-nodalix-accent-dim" />
          <p className="text-sm text-nodalix-muted">
            {error ?? "Cargando Nodalix Files…"}
          </p>
        </div>
      </div>
    );
  }

  return (
    <div
      className="nodalix-shell flex h-full flex-col overflow-hidden"
      style={appStyle}
      onContextMenuCapture={(e) => e.preventDefault()}
    >
      <div className="flex min-h-0 flex-1">
        <Sidebar
          items={sidebarItems}
          activePath={activeTab.path}
          collapsed={sidebarCollapsed}
          onToggleCollapsed={() => setSidebarCollapsed((value) => !value)}
          onNavigate={goTo}
          onContextMenu={showSidebarContextMenu}
          onDropPaths={handleSidebarDrop}
          onDropToItem={handleSidebarItemDrop}
          onPreloadItem={(item) => {
            if (
              item.is_dir &&
              !item.path.startsWith("nodalix://") &&
              item.kind !== "trash"
            ) {
              void preload(item.path);
            }
          }}
          localsendAvailable={localsendAvailable}
          localsendDevices={localsendDevices}
          localsendScanning={localsendScanning}
          localsendSendingId={localsendSendingId}
          onRefreshLocalSend={refreshLocalSendDevices}
          onDropToLocalSendDevice={handleLocalSendDeviceDrop}
          onHoverItem={(item) => scheduleHoverNavigate(item.path)}
          onLeaveHoverItem={(item) => cancelHoverNavigate(item.path)}
        />
        <main className="flex min-w-0 flex-1 flex-col">
          <Toolbar
            tabs={tabs}
            activeTabId={activeTabId}
            canGoBack={activeTab.historyIndex > 0}
            canGoForward={activeTab.historyIndex < activeTab.history.length - 1}
            searchQuery={searchQuery}
            accentName={accentName}
            accentOptions={accentOptions}
            pathLabels={pathLabels}
            onBack={() => updateActiveTab((t) => tabGoBack(t))}
            onForward={() => updateActiveTab((t) => tabGoForward(t))}
            onBreadcrumb={goTo}
            onCopyPath={copyPath}
            onPathContextMenu={showPathContextMenu}
            onBreadcrumbDrop={(path, dataTransfer) =>
              void handleDropOnPath(path, dataTransfer)
            }
            onPathHover={scheduleHoverNavigate}
            onPathLeave={cancelHoverNavigate}
            onTabSelect={setActiveTabId}
            onTabClose={(id) => {
              setTabs((prev) => {
                if (prev.length <= 1) return prev;
                const next = prev.filter((t) => t.id !== id);
                if (activeTabId === id) setActiveTabId(next[0].id);
                return next;
              });
            }}
            onTabReorder={(draggedTabId, targetTabId) => {
              setTabs((prev) => {
                const from = prev.findIndex((tab) => tab.id === draggedTabId);
                const to = prev.findIndex((tab) => tab.id === targetTabId);
                if (from === -1 || to === -1 || from === to) return prev;
                const next = [...prev];
                const [dragged] = next.splice(from, 1);
                next.splice(to, 0, dragged);
                return next;
              });
            }}
            onNewTab={() => {
              const t = createTab(home);
              setTabs((p) => [...p, t]);
              setActiveTabId(t.id);
            }}
            onSearchChange={setSearchQuery}
            onAccentChange={(value) =>
              setAccentName(value as typeof accentName)
            }
          />
          {actionError && (
            <div className="mx-3 mt-1 rounded-lg border border-nodalix-danger/30 bg-nodalix-danger/10 px-3 py-1.5 text-xs text-nodalix-danger">
              {actionError}
              <button
                type="button"
                className="ml-2 underline"
                onClick={() => setActionError(null)}
              >
                cerrar
              </button>
            </div>
          )}
          <div className="nodalix-file-area relative flex min-h-0 flex-1 flex-col">
            {isNetworkView && (
              <>
                <div className="nodalix-network-connect-control">
                  <button
                    ref={networkConnectButtonRef}
                    type="button"
                    aria-label="Conectar disco de red"
                    title="Conectar disco de red"
                    aria-expanded={networkConnectOpen}
                    onClick={() => setNetworkConnectOpen((open) => !open)}
                    className={[
                      "nodalix-floating-button",
                      networkConnectOpen && "nodalix-floating-button-active",
                    ]
                      .filter(Boolean)
                      .join(" ")}
                  >
                    <NetworkAddIcon />
                  </button>
                </div>
                {networkConnectOpen && (
                  <div
                    ref={networkConnectRef}
                    className="nodalix-network-connect-popover nodalix-animate-in"
                    role="dialog"
                    aria-label="Conectar disco de red"
                    onPointerDown={(e) => e.stopPropagation()}
                    onClick={(e) => e.stopPropagation()}
                  >
                    <form
                      className="nodalix-network-form"
                      onSubmit={(event) => void submitNetworkConnection(event)}
                    >
                      <p className="nodalix-settings-title">Conectar disco</p>
                      <div className="nodalix-network-field">
                        <span>Protocolo</span>
                        <div className="nodalix-network-protocols">
                          {["smb", "sftp", "ftp"].map((protocol) => (
                            <button
                              key={protocol}
                              type="button"
                              className={[
                                "nodalix-filter-choice",
                                networkForm.protocol === protocol &&
                                  "nodalix-filter-choice-active",
                              ]
                                .filter(Boolean)
                                .join(" ")}
                              onClick={() =>
                                setNetworkForm((form) => ({
                                  ...form,
                                  protocol,
                                }))
                              }
                            >
                              {protocol.toUpperCase()}
                            </button>
                          ))}
                        </div>
                      </div>
                      <label>
                        <span>Servidor</span>
                        <input
                          value={networkForm.host}
                          onChange={(event) =>
                            setNetworkForm((form) => ({
                              ...form,
                              host: event.target.value,
                            }))
                          }
                          placeholder="192.168.1.20"
                          autoComplete="off"
                          required
                        />
                      </label>
                      <label>
                        <span>Recurso compartido</span>
                        <input
                          value={networkForm.share}
                          onChange={(event) =>
                            setNetworkForm((form) => ({
                              ...form,
                              share: event.target.value,
                            }))
                          }
                          placeholder="Documentos"
                          autoComplete="off"
                        />
                      </label>
                      <label>
                        <span>Usuario</span>
                        <input
                          value={networkForm.username}
                          onChange={(event) =>
                            setNetworkForm((form) => ({
                              ...form,
                              username: event.target.value,
                            }))
                          }
                          placeholder="opcional"
                          autoComplete="username"
                        />
                      </label>
                      <label>
                        <span>Contraseña</span>
                        <input
                          value={networkForm.password}
                          onChange={(event) =>
                            setNetworkForm((form) => ({
                              ...form,
                              password: event.target.value,
                            }))
                          }
                          type="password"
                          placeholder="opcional"
                          autoComplete="current-password"
                        />
                      </label>
                      <button type="submit" className="nodalix-network-submit">
                        Conectar
                      </button>
                    </form>
                  </div>
                )}
              </>
            )}
            <div className="nodalix-floating-controls">
              <button
                ref={filtersButtonRef}
                type="button"
                aria-label="Filtros"
                title="Filtros"
                aria-expanded={filtersOpen}
                onClick={() => setFiltersOpen((open) => !open)}
                className={[
                  "nodalix-floating-button",
                  filtersOpen && "nodalix-floating-button-active",
                ]
                  .filter(Boolean)
                  .join(" ")}
              >
                <FilterIcon />
              </button>
              <button
                type="button"
                aria-label="Vista de árbol"
                title="Vista de árbol"
                onClick={() => setViewMode("tree")}
                className={[
                  "nodalix-floating-button",
                  viewMode === "tree" && "nodalix-floating-button-active",
                ]
                  .filter(Boolean)
                  .join(" ")}
              >
                <TreeIcon />
              </button>
              <button
                type="button"
                aria-label="Vista de cuadrícula"
                title="Vista de cuadrícula"
                onClick={() => setViewMode("grid")}
                className={[
                  "nodalix-floating-button",
                  viewMode === "grid" && "nodalix-floating-button-active",
                ]
                  .filter(Boolean)
                  .join(" ")}
              >
                <GridIcon />
              </button>
            </div>
            {filtersOpen && (
              <div
                ref={filtersRef}
                className="nodalix-filter-popover nodalix-animate-in"
                role="dialog"
                aria-label="Filtros"
                onPointerDown={(e) => e.stopPropagation()}
                onClick={(e) => e.stopPropagation()}
              >
                <FilterGroup label="Tipo">
                  <FilterChoice
                    active={filterType === "all"}
                    label="Todo"
                    onClick={() => setFilterType("all")}
                  />
                  <FilterChoice
                    active={filterType === "folders"}
                    label="Carpetas"
                    onClick={() => setFilterType("folders")}
                  />
                  <FilterChoice
                    active={filterType === "files"}
                    label="Archivos"
                    onClick={() => setFilterType("files")}
                  />
                </FilterGroup>
                <FilterGroup label="Ordenar por">
                  <FilterChoice
                    active={sortKey === "name"}
                    label="Nombre"
                    onClick={() => setSortKey("name")}
                  />
                  <FilterChoice
                    active={sortKey === "modified"}
                    label="Modificado"
                    onClick={() => setSortKey("modified")}
                  />
                  <FilterChoice
                    active={sortKey === "size"}
                    label="Tamaño"
                    onClick={() => setSortKey("size")}
                  />
                  <FilterChoice
                    active={sortKey === "type"}
                    label="Tipo"
                    onClick={() => setSortKey("type")}
                  />
                </FilterGroup>
                <FilterGroup label="Dirección">
                  <FilterChoice
                    active={sortDirection === "asc"}
                    label="Ascendente"
                    onClick={() => setSortDirection("asc")}
                  />
                  <FilterChoice
                    active={sortDirection === "desc"}
                    label="Descendente"
                    onClick={() => setSortDirection("desc")}
                  />
                </FilterGroup>
                <FilterGroup label="Opciones">
                  <button
                    type="button"
                    role="switch"
                    aria-checked={showHiddenFiles}
                    className={[
                      "nodalix-filter-toggle",
                      showHiddenFiles && "nodalix-filter-toggle-active",
                    ]
                      .filter(Boolean)
                      .join(" ")}
                    onClick={() => setShowHiddenFiles((value) => !value)}
                  >
                    <span>Mostrar ocultos</span>
                    <span className="nodalix-filter-switch" />
                  </button>
                </FilterGroup>
              </div>
            )}
            <FileGrid
            entries={displayedEntries}
            loading={isVirtualView ? virtualEntries === null : loading}
            error={isVirtualView ? virtualError : error}
            selectedPaths={selectedPaths}
            folderStyles={folderStyles}
            defaultFolderColor={accent.accent}
            specialDirs={specialDirs}
            viewMode={viewMode}
            zoom={zoom}
            renamePath={inlineRenamePath}
            renameInitialName={inlineRenameInitial}
            onZoomChange={setZoom}
            showHiddenFiles={showHiddenFiles}
            emptyMessage={
              debouncedSearchQuery.trim()
                ? "No se encontraron archivos"
                : isDisksView
                  ? "No se encontraron discos montados"
                  : isNetworkView
                    ? "No hay ubicaciones de red montadas"
                    : currentIsTrash
                      ? "La papelera está vacía"
                      : "Esta carpeta está vacía"
            }
            emptyIcon={
              !debouncedSearchQuery.trim() && currentIsTrash ? (
                <FileTypeIcon kind="trash" className="h-24 w-24 opacity-90" />
              ) : !debouncedSearchQuery.trim() && isNetworkView ? (
                <FileTypeIcon kind="network" className="h-24 w-24 opacity-90" />
              ) : undefined
            }
            draggingPaths={draggingPaths}
            dropTargetPath={dropTargetPath}
            onSelect={handleSelect}
            onBoxSelect={handleBoxSelect}
            onBackgroundClick={handleBackgroundClick}
            onOpen={(e) => void handleOpen(e)}
            onPreload={preloadEntry}
            onContextMenu={showFileContextMenu}
            onRenameCommit={commitInlineRename}
            onRenameCancel={cancelInlineRename}
            onBackgroundContext={showBackgroundMenu}
            onDragStart={handleDragStart}
            onDragOver={handleDragOverFolder}
            onDragEnter={handleDragEnterFolder}
            onDragLeave={handleDragLeaveFolder}
            onDrop={handleDropOnFolder}
            onDropToCurrentDirectory={handleDropToCurrentDirectory}
            onDragEnd={handleDragEnd}
            onVisibleEntriesChange={setVisibleEntriesForSelection}
          />
          </div>
        </main>
      </div>

      <ContextMenu open={menu.open} x={menu.x} y={menu.y} onClose={closeMenu}>
        {menu.items}
      </ContextMenu>

      <RenameModal
        open={modal === "rename"}
        initialName={renameInitial}
        onClose={() => setModal(null)}
        error={actionError}
        onConfirm={async (name) => {
          try {
            if (modalTarget && sidebarItems.some((s) => s.id === modalTarget)) {
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
        open={
          modal === "folderColor" ||
          modal === "folderIcon" ||
          modal === "sidebarIcon"
        }
        mode={modal === "folderColor" ? "color" : "icon"}
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
          } else if (modal === "folderIcon") {
            await setFolderIcon(p, value || null);
            setFolderStyles((s) => ({
              ...s,
              [p]: { ...s[p], icon: value || null },
            }));
          } else if (modal === "sidebarIcon") {
            await setSidebarItemIcon(p, value || null);
          }
          setModal(null);
        }}
      />

      <ConfirmModal
        open={modal === "confirmPermanentDelete"}
        title="Eliminar permanentemente"
        message={
          pathsForAction().length > 1
            ? `¿Eliminar ${pathsForAction().length} elementos permanentemente? Esta acción no se puede deshacer.`
            : "¿Eliminar permanentemente? Esta acción no se puede deshacer."
        }
        confirmLabel="Eliminar permanentemente"
        onClose={() => setModal(null)}
        onConfirm={async () => {
          await actions.deletePermanently(pathsForAction(), cwd);
          setSelectedPaths(new Set());
          setModal(null);
        }}
      />
    </div>
  );
}

function FilterGroup({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <section className="nodalix-filter-section">
      <p>{label}</p>
      <div>{children}</div>
    </section>
  );
}

function FilterChoice({
  active,
  label,
  onClick,
}: {
  active: boolean;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className={["nodalix-filter-choice", active && "nodalix-filter-choice-active"]
        .filter(Boolean)
        .join(" ")}
      onClick={onClick}
    >
      {label}
    </button>
  );
}

function GridIcon() {
  return (
    <svg className="h-4 w-4" viewBox="0 0 24 24" fill="currentColor" aria-hidden>
      <path d="M4 4h6.5v6.5H4V4zm9.5 0H20v6.5h-6.5V4zM4 13.5h6.5V20H4v-6.5zm9.5 0H20V20h-6.5v-6.5z" />
    </svg>
  );
}

function TreeIcon() {
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
      <path d="M6 4v14" />
      <path d="M6 8h5" />
      <path d="M6 14h5" />
      <path d="M13 6h7v4h-7zM13 12h7v4h-7zM13 18h7v2h-7z" />
    </svg>
  );
}

function FilterIcon() {
  return (
    <svg
      className="h-4 w-4"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      aria-hidden
    >
      <path d="M4 7h16M7 12h10M10 17h4" />
    </svg>
  );
}

function NetworkAddIcon() {
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
      <path d="M12 5a7 7 0 017 7M12 5a7 7 0 00-7 7M12 5v14M5 12h14M7 17h7" />
      <path d="M18 16v6M15 19h6" />
    </svg>
  );
}
