import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { isHiddenEntry, listDirectory } from "../lib/files";
import type { FileEntry, SpecialDirs } from "../lib/files";
import {
  getFolderStyle,
  type FolderStyle,
} from "../lib/folderCustomization";
import FileItem from "./FileItem";

export type ViewMode = "grid" | "tree";
export type ZoomLevel = "small" | "medium" | "large" | "xlarge";

const GAP = 12;
const OVERSCAN = 2;
const DRAG_THRESHOLD = 4;

const zoomMetrics: Record<ZoomLevel, { itemW: number; itemH: number }> = {
  small: { itemW: 96, itemH: 92 },
  medium: { itemW: 136, itemH: 128 },
  large: { itemW: 176, itemH: 162 },
  xlarge: { itemW: 224, itemH: 206 },
};

const zoomLevels: ZoomLevel[] = ["small", "medium", "large", "xlarge"];

interface FileGridProps {
  entries: FileEntry[];
  loading: boolean;
  error: string | null;
  selectedPaths: Set<string>;
  draggingPaths: Set<string>;
  dropTargetPath: string | null;
  folderStyles: Record<string, FolderStyle>;
  defaultFolderColor?: string | null;
  specialDirs?: SpecialDirs | null;
  viewMode: ViewMode;
  zoom: ZoomLevel;
  renamePath?: string | null;
  renameInitialName?: string | null;
  onZoomChange?: (zoom: ZoomLevel) => void;
  showHiddenFiles: boolean;
  emptyMessage?: string;
  emptyIcon?: React.ReactNode;
  onSelect: (entry: FileEntry, additive: boolean, range: boolean) => void;
  onBoxSelect: (paths: string[], additive: boolean) => void;
  onBackgroundClick: (preserveSelection: boolean) => void;
  onOpen: (entry: FileEntry) => void;
  onPreload?: (entry: FileEntry) => void;
  onContextMenu: (entry: FileEntry, x: number, y: number) => void;
  onRenameCommit?: (entry: FileEntry, name: string) => Promise<boolean>;
  onRenameCancel?: () => void;
  onBackgroundContext: (x: number, y: number) => void;
  onDragStart: (entry: FileEntry, event: React.DragEvent<HTMLElement>) => void;
  onDragOver: (entry: FileEntry, event: React.DragEvent<HTMLElement>) => void;
  onDragEnter: (entry: FileEntry, event: React.DragEvent<HTMLElement>) => void;
  onDragLeave: (entry: FileEntry, event: React.DragEvent<HTMLElement>) => void;
  onDrop: (entry: FileEntry, event: React.DragEvent<HTMLElement>) => void;
  onDropToCurrentDirectory?: (event: React.DragEvent<HTMLElement>) => void;
  onDragEnd: (event: React.DragEvent<HTMLElement>) => void;
  onVisibleEntriesChange?: (entries: FileEntry[]) => void;
}

interface SelectionBoxState {
  active: boolean;
  left: number;
  top: number;
  width: number;
  height: number;
}

interface TreeChildrenState {
  loading: boolean;
  entries: FileEntry[] | null;
  error: string | null;
}

interface TreeRow {
  entry: FileEntry;
  depth: number;
  placeholder?: "loading" | "empty" | "error";
}

function intersects(a: DOMRect, b: DOMRect) {
  return (
    a.left <= b.right &&
    a.right >= b.left &&
    a.top <= b.bottom &&
    a.bottom >= b.top
  );
}

function samePaths(a: string[], b: string[]) {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i += 1) if (a[i] !== b[i]) return false;
  return true;
}

function hasPossibleInternalDrag(event: React.DragEvent<HTMLElement>) {
  return Array.from(event.dataTransfer.types).some(
    (type) =>
      type === "application/x-nodalix-file-paths" ||
      type === "text/plain" ||
      type === "text/uri-list",
  );
}

function FileGridInner({
  entries,
  loading,
  error,
  selectedPaths,
  draggingPaths,
  dropTargetPath,
  folderStyles,
  defaultFolderColor,
  specialDirs,
  viewMode,
  zoom,
  renamePath,
  renameInitialName,
  onZoomChange,
  showHiddenFiles,
  emptyMessage = "Esta carpeta está vacía",
  emptyIcon,
  onSelect,
  onBoxSelect,
  onBackgroundClick,
  onOpen,
  onPreload,
  onContextMenu,
  onRenameCommit,
  onRenameCancel,
  onBackgroundContext,
  onDragStart,
  onDragOver,
  onDragEnter,
  onDragLeave,
  onDrop,
  onDropToCurrentDirectory,
  onDragEnd,
  onVisibleEntriesChange,
}: FileGridProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const selectionRef = useRef({
    startX: 0,
    startY: 0,
    additive: false,
    dragging: false,
    raf: 0,
    lastPaths: [] as string[],
  });
  const metrics = zoomMetrics[zoom];
  const [cols, setCols] = useState(6);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportH, setViewportH] = useState(600);
  const [selectionBox, setSelectionBox] = useState<SelectionBoxState>({
    active: false,
    left: 0,
    top: 0,
    width: 0,
    height: 0,
  });
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set());
  const [treeChildren, setTreeChildren] = useState<
    Record<string, TreeChildrenState>
  >({});

  useEffect(() => {
    setTreeChildren({});
    setExpandedPaths(new Set());
  }, [showHiddenFiles]);

  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => {
      const w = el.clientWidth - 24;
      setCols(Math.max(1, Math.floor((w + GAP) / (metrics.itemW + GAP))));
      setViewportH(el.clientHeight);
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, [metrics.itemW]);

  const rows = viewMode === "grid" ? Math.ceil(entries.length / cols) : 0;
  const totalH = rows * (metrics.itemH + GAP) + GAP;
  const startRow = Math.max(
    0,
    Math.floor(scrollTop / (metrics.itemH + GAP)) - OVERSCAN,
  );
  const endRow = Math.min(
    rows,
    Math.ceil((scrollTop + viewportH) / (metrics.itemH + GAP)) + OVERSCAN,
  );

  const visible = useMemo(() => {
    const items: { entry: FileEntry; row: number; col: number }[] = [];
    if (viewMode !== "grid") return items;
    for (let r = startRow; r < endRow; r += 1) {
      for (let c = 0; c < cols; c += 1) {
        const idx = r * cols + c;
        if (idx >= entries.length) break;
        items.push({ entry: entries[idx], row: r, col: c });
      }
    }
    return items;
  }, [entries, cols, startRow, endRow, viewMode]);

  useEffect(() => {
    if (!renamePath || !scrollRef.current) return;
    if (viewMode === "grid") {
      const index = entries.findIndex((entry) => entry.path === renamePath);
      if (index === -1) return;
      const row = Math.floor(index / cols);
      scrollRef.current.scrollTop = Math.max(0, row * (metrics.itemH + GAP) - GAP);
      return;
    }
    requestAnimationFrame(() => {
      scrollRef.current
        ?.querySelector<HTMLElement>(`[data-file-path="${CSS.escape(renamePath)}"]`)
        ?.scrollIntoView({ block: "nearest" });
    });
  }, [cols, entries, metrics.itemH, renamePath, viewMode]);

  const treeRows = useMemo(() => {
    const rows: TreeRow[] = [];
    const append = (items: FileEntry[], depth: number) => {
      for (const entry of items) {
        rows.push({ entry, depth });
        if (!entry.is_dir || !expandedPaths.has(entry.path)) continue;
        const children = treeChildren[entry.path];
        if (!children || children.loading) {
          rows.push({ entry, depth: depth + 1, placeholder: "loading" });
        } else if (children.error) {
          rows.push({ entry, depth: depth + 1, placeholder: "error" });
        } else if (!children.entries || children.entries.length === 0) {
          rows.push({ entry, depth: depth + 1, placeholder: "empty" });
        } else {
          append(children.entries, depth + 1);
        }
      }
    };
    append(entries, 0);
    return rows;
  }, [entries, expandedPaths, treeChildren]);

  useEffect(() => {
    if (!onVisibleEntriesChange) return;
    onVisibleEntriesChange(
      viewMode === "tree"
        ? treeRows.filter((row) => !row.placeholder).map((row) => row.entry)
        : entries,
    );
  }, [entries, onVisibleEntriesChange, treeRows, viewMode]);

  const toggleTreeExpansion = useCallback(
    async (entry: FileEntry) => {
      if (!entry.is_dir) return;
      let shouldLoad = false;
      setExpandedPaths((prev) => {
        const next = new Set(prev);
        if (next.has(entry.path)) next.delete(entry.path);
        else {
          next.add(entry.path);
          shouldLoad = !treeChildren[entry.path];
        }
        return next;
      });
      if (!shouldLoad) return;
      setTreeChildren((prev) => ({
        ...prev,
        [entry.path]: { loading: true, entries: null, error: null },
      }));
      try {
        const children = await listDirectory(entry.path);
        const visibleChildren = showHiddenFiles
          ? children
          : children.filter((child) => !isHiddenEntry(child));
        setTreeChildren((prev) => ({
          ...prev,
          [entry.path]: {
            loading: false,
            entries: visibleChildren,
            error: null,
          },
        }));
      } catch (e) {
        setTreeChildren((prev) => ({
          ...prev,
          [entry.path]: {
            loading: false,
            entries: null,
            error: e instanceof Error ? e.message : String(e),
          },
        }));
      }
    },
    [showHiddenFiles, treeChildren],
  );

  const folderPreferences = useMemo(
    () => ({ defaultFolderColor }),
    [defaultFolderColor],
  );

  const resolveFolderStyle = useCallback(
    (entry: FileEntry) =>
      getFolderStyle(entry, folderPreferences, folderStyles, specialDirs),
    [folderPreferences, folderStyles, specialDirs],
  );

  const entryByPath = useMemo(() => {
    const rows: TreeRow[] =
      viewMode === "tree"
        ? treeRows
        : entries.map((entry) => ({ entry, depth: 0 }));
    return new Map(
      rows
        .filter((row) => !row.placeholder)
        .map((row) => [row.entry.path, row.entry]),
    );
  }, [entries, treeRows, viewMode]);

  const fileItemFromEvent = useCallback(
    (event: React.DragEvent<HTMLElement>) => {
      const target = event.target as HTMLElement;
      const item = target.closest<HTMLElement>("[data-file-item='true']");
      const path = item?.dataset.filePath;
      return path ? entryByPath.get(path) : undefined;
    },
    [entryByPath],
  );

  const onScroll = useCallback(() => {
    if (scrollRef.current) setScrollTop(scrollRef.current.scrollTop);
  }, []);

  const handleWheel = useCallback(
    (event: React.WheelEvent<HTMLDivElement>) => {
      if (viewMode !== "grid" || !event.shiftKey || !onZoomChange) return;
      event.preventDefault();
      const current = zoomLevels.indexOf(zoom);
      const delta =
        Math.abs(event.deltaX) > Math.abs(event.deltaY)
          ? event.deltaX
          : event.deltaY;
      if (delta === 0) return;
      const direction = delta < 0 ? 1 : -1;
      const next = Math.min(
        zoomLevels.length - 1,
        Math.max(0, current + direction),
      );
      if (next !== current) onZoomChange(zoomLevels[next]);
    },
    [onZoomChange, viewMode, zoom],
  );

  const handleBackgroundContext = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      const target = e.target as HTMLElement;
      if (target.closest("[data-file-item='true']")) return;
      e.preventDefault();
      e.stopPropagation();
      onBackgroundContext(e.clientX, e.clientY);
    },
    [onBackgroundContext],
  );

  const updateSelectionBox = useCallback(
    (clientX: number, clientY: number) => {
      const state = selectionRef.current;
      const left = Math.min(state.startX, clientX);
      const top = Math.min(state.startY, clientY);
      const width = Math.abs(clientX - state.startX);
      const height = Math.abs(clientY - state.startY);
      const boxRect = new DOMRect(left, top, width, height);
      const nodes =
        scrollRef.current?.querySelectorAll<HTMLElement>("[data-file-path]") ??
        [];
      const paths: string[] = [];
      nodes.forEach((node) => {
        if (intersects(node.getBoundingClientRect(), boxRect)) {
          const path = node.dataset.filePath;
          if (path) paths.push(path);
        }
      });

      setSelectionBox({ active: true, left, top, width, height });
      if (!samePaths(paths, state.lastPaths)) {
        state.lastPaths = paths;
        onBoxSelect(paths, state.additive);
      }
    },
    [onBoxSelect],
  );

  const handleMouseDown = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      if (e.button !== 0) return;
      const target = e.target as HTMLElement;
      if (
        target.closest(
          "[data-file-item='true'], input, textarea, select, [contenteditable='true']",
        )
      ) {
        return;
      }
      const state = selectionRef.current;
      state.startX = e.clientX;
      state.startY = e.clientY;
      state.additive = e.metaKey || e.ctrlKey;
      state.dragging = false;
      state.lastPaths = ["__selection-start__"];

      const onMove = (event: MouseEvent) => {
        const moved =
          Math.abs(event.clientX - state.startX) > DRAG_THRESHOLD ||
          Math.abs(event.clientY - state.startY) > DRAG_THRESHOLD;
        if (!moved && !state.dragging) return;
        state.dragging = true;
        event.preventDefault();
        if (state.raf) cancelAnimationFrame(state.raf);
        state.raf = requestAnimationFrame(() =>
          updateSelectionBox(event.clientX, event.clientY),
        );
      };

      const onUp = (event: MouseEvent) => {
        window.removeEventListener("mousemove", onMove);
        window.removeEventListener("mouseup", onUp);
        if (state.raf) cancelAnimationFrame(state.raf);
        if (!state.dragging) onBackgroundClick(event.metaKey || event.ctrlKey);
        setSelectionBox((box) => ({ ...box, active: false }));
        state.dragging = false;
        state.lastPaths = [];
      };

      window.addEventListener("mousemove", onMove);
      window.addEventListener("mouseup", onUp);
    },
    [onBackgroundClick, updateSelectionBox],
  );

  if (error) {
    return (
      <div className="flex flex-1 items-center justify-center px-6 text-center text-sm text-nodalix-danger">
        {error}
      </div>
    );
  }

  return (
    <div
      ref={scrollRef}
      data-file-area="true"
      className="nodalix-content relative flex-1 select-none overflow-auto px-3 pb-2 pt-12"
      onScroll={onScroll}
      onWheel={handleWheel}
      onMouseDown={handleMouseDown}
      onContextMenu={handleBackgroundContext}
      onDragOver={(e) => {
        const entry = fileItemFromEvent(e);
        if (entry?.is_dir) {
          onDragOver(entry, e);
          return;
        }
        if (
          onDropToCurrentDirectory &&
          (draggingPaths.size > 0 || hasPossibleInternalDrag(e))
        ) {
          e.preventDefault();
          e.dataTransfer.dropEffect = "move";
        }
      }}
      onDrop={(e) => {
        const entry = fileItemFromEvent(e);
        if (entry?.is_dir) {
          onDrop(entry, e);
          return;
        }
        if (onDropToCurrentDirectory) onDropToCurrentDirectory(e);
      }}
    >
      {loading && entries.length === 0 && (
        <div className="grid grid-cols-[repeat(auto-fill,104px)] gap-3 p-1">
          {Array.from({ length: 12 }).map((_, i) => (
            <div key={i} className="nodalix-skeleton h-[88px] w-[104px]" />
          ))}
        </div>
      )}

      {viewMode === "grid" ? (
        <div className="relative" style={{ height: totalH }}>
          {visible.map(({ entry, row, col }) => (
            <div
              key={entry.path}
              className="absolute"
              style={{
                left: col * (metrics.itemW + GAP) + GAP,
                top: row * (metrics.itemH + GAP) + GAP,
                width: metrics.itemW,
                minHeight: metrics.itemH,
              }}
            >
              <FileItem
                entry={entry}
                selected={selectedPaths.has(entry.path)}
                dragging={draggingPaths.has(entry.path)}
                dropTarget={dropTargetPath === entry.path}
                folderStyle={resolveFolderStyle(entry)}
                specialDirs={specialDirs}
                viewMode={viewMode}
                zoom={zoom}
                renameInitialName={
                  renamePath === entry.path ? renameInitialName : null
                }
                onSelect={onSelect}
                onOpen={onOpen}
                onPreload={onPreload}
                onContextMenu={onContextMenu}
                onRenameCommit={onRenameCommit}
                onRenameCancel={onRenameCancel}
                onDragStart={onDragStart}
                onDragOver={onDragOver}
                onDragEnter={onDragEnter}
                onDragLeave={onDragLeave}
                onDrop={onDrop}
                onDragEnd={onDragEnd}
              />
            </div>
          ))}
        </div>
      ) : (
        <div className="min-w-[640px] py-1">
          <div className="grid grid-cols-[minmax(220px,1fr)_150px_110px_90px] gap-3 px-2 pb-1 text-[11px] font-semibold uppercase tracking-wide text-nodalix-muted">
            <span>Nombre</span>
            <span>Fecha de modificación</span>
            <span>Tipo</span>
            <span className="text-right">Tamaño</span>
          </div>
          <div className="space-y-0.5">
            {treeRows.map(
              ({ entry, depth, placeholder }, index) =>
                placeholder ? (
                  <div
                    key={`${entry.path}-${placeholder}-${index}`}
                    className="rounded-lg px-2 py-1.5 text-[12px] text-nodalix-muted"
                    style={{ paddingLeft: depth * 16 + 34 }}
                  >
                    {placeholder === "loading" && "Cargando…"}
                    {placeholder === "empty" && "Vacío"}
                    {placeholder === "error" && "No se pudo cargar la carpeta"}
                  </div>
                ) : (
                  <FileItem
                    key={entry.path}
                    entry={entry}
                    selected={selectedPaths.has(entry.path)}
                    dragging={draggingPaths.has(entry.path)}
                    dropTarget={dropTargetPath === entry.path}
                    folderStyle={resolveFolderStyle(entry)}
                    specialDirs={specialDirs}
                    viewMode={viewMode}
                    zoom={zoom}
                    renameInitialName={
                      renamePath === entry.path ? renameInitialName : null
                    }
                    depth={depth}
                    treeExpanded={expandedPaths.has(entry.path)}
                    treeLoading={treeChildren[entry.path]?.loading ?? false}
                    onToggleTree={toggleTreeExpansion}
                    onSelect={onSelect}
                    onOpen={onOpen}
                    onPreload={onPreload}
                    onContextMenu={onContextMenu}
                    onRenameCommit={onRenameCommit}
                    onRenameCancel={onRenameCancel}
                    onDragStart={onDragStart}
                    onDragOver={onDragOver}
                    onDragEnter={onDragEnter}
                    onDragLeave={onDragLeave}
                    onDrop={onDrop}
                    onDragEnd={onDragEnd}
                  />
                ),
            )}
          </div>
        </div>
      )}

      {!loading && entries.length === 0 && (
        <div className="flex flex-col items-center justify-center gap-3 py-16 text-center text-sm text-nodalix-muted">
          {emptyIcon}
          <p>{emptyMessage}</p>
        </div>
      )}

      {selectionBox.active && (
        <div
          className="pointer-events-none fixed z-[190] rounded-lg border border-nodalix-accent/55 bg-nodalix-accent/18 shadow-[0_0_0_1px_rgba(203,166,247,0.12)]"
          style={{
            left: selectionBox.left,
            top: selectionBox.top,
            width: selectionBox.width,
            height: selectionBox.height,
          }}
        />
      )}
    </div>
  );
}

export default memo(FileGridInner);
