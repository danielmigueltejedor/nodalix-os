import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { FileEntry } from "../lib/files";
import type { FolderStyle } from "../lib/folderCustomization";
import FileItem from "./FileItem";

const ITEM_W = 104;
const ITEM_H = 96;
const GAP = 12;
const OVERSCAN = 2;

interface FileGridProps {
  entries: FileEntry[];
  loading: boolean;
  error: string | null;
  selectedPaths: Set<string>;
  folderStyles: Record<string, FolderStyle>;
  onSelect: (entry: FileEntry, additive: boolean) => void;
  onOpen: (entry: FileEntry) => void;
  onContextMenu: (entry: FileEntry, x: number, y: number) => void;
  onBackgroundContext: (x: number, y: number) => void;
}

function FileGridInner({
  entries,
  loading,
  error,
  selectedPaths,
  folderStyles,
  onSelect,
  onOpen,
  onContextMenu,
  onBackgroundContext,
}: FileGridProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [cols, setCols] = useState(6);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportH, setViewportH] = useState(600);

  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => {
      const w = el.clientWidth - 24;
      setCols(Math.max(1, Math.floor((w + GAP) / (ITEM_W + GAP))));
      setViewportH(el.clientHeight);
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const rows = Math.ceil(entries.length / cols);
  const totalH = rows * (ITEM_H + GAP) + GAP;
  const startRow = Math.max(0, Math.floor(scrollTop / (ITEM_H + GAP)) - OVERSCAN);
  const endRow = Math.min(
    rows,
    Math.ceil((scrollTop + viewportH) / (ITEM_H + GAP)) + OVERSCAN,
  );

  const visible = useMemo(() => {
    const items: { entry: FileEntry; row: number; col: number }[] = [];
    for (let r = startRow; r < endRow; r++) {
      for (let c = 0; c < cols; c++) {
        const idx = r * cols + c;
        if (idx >= entries.length) break;
        items.push({ entry: entries[idx], row: r, col: c });
      }
    }
    return items;
  }, [entries, cols, startRow, endRow]);

  const onScroll = useCallback(() => {
    if (scrollRef.current) setScrollTop(scrollRef.current.scrollTop);
  }, []);

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
      className="flex-1 overflow-auto px-3 py-2"
      onScroll={onScroll}
      onContextMenu={(e) => {
        if (e.target === e.currentTarget) {
          e.preventDefault();
          onBackgroundContext(e.clientX, e.clientY);
        }
      }}
    >
      {loading && entries.length === 0 && (
        <div className="grid grid-cols-[repeat(auto-fill,104px)] gap-3 p-1">
          {Array.from({ length: 12 }).map((_, i) => (
            <div key={i} className="nodalix-skeleton h-[88px] w-[104px]" />
          ))}
        </div>
      )}
      <div className="relative" style={{ height: totalH }}>
        {visible.map(({ entry, row, col }) => (
          <div
            key={entry.path}
            className="absolute"
            style={{
              left: col * (ITEM_W + GAP) + GAP,
              top: row * (ITEM_H + GAP) + GAP,
              width: ITEM_W,
              height: ITEM_H,
            }}
          >
            <FileItem
              entry={entry}
              selected={selectedPaths.has(entry.path)}
              folderStyle={entry.is_dir ? folderStyles[entry.path] : undefined}
              onSelect={onSelect}
              onOpen={onOpen}
              onContextMenu={onContextMenu}
            />
          </div>
        ))}
      </div>
      {!loading && entries.length === 0 && (
        <p className="py-16 text-center text-sm text-nodalix-muted">This folder is empty</p>
      )}
    </div>
  );
}

export default memo(FileGridInner);
