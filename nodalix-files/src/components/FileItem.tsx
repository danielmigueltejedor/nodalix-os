import { convertFileSrc } from "@tauri-apps/api/core";
import { memo, useCallback, useMemo, useState } from "react";
import type { FileEntry, SpecialDirs } from "../lib/files";
import { formatSize, formatTimestamp, getFileKind } from "../lib/files";
import type { FolderStyle } from "../lib/folderCustomization";
import { FileTypeIcon } from "../lib/icons";
import type { ViewMode, ZoomLevel } from "./FileGrid";

interface FileItemProps {
  entry: FileEntry;
  selected: boolean;
  dragging: boolean;
  dropTarget: boolean;
  folderStyle?: FolderStyle;
  specialDirs?: SpecialDirs | null;
  viewMode: ViewMode;
  zoom: ZoomLevel;
  depth?: number;
  treeExpanded?: boolean;
  treeLoading?: boolean;
  onToggleTree?: (entry: FileEntry) => void;
  onSelect: (entry: FileEntry, additive: boolean, range: boolean) => void;
  onOpen: (entry: FileEntry) => void;
  onContextMenu: (entry: FileEntry, x: number, y: number) => void;
  onDragStart: (entry: FileEntry, event: React.DragEvent<HTMLElement>) => void;
  onDragOver: (entry: FileEntry, event: React.DragEvent<HTMLElement>) => void;
  onDragEnter: (entry: FileEntry, event: React.DragEvent<HTMLElement>) => void;
  onDragLeave: (entry: FileEntry, event: React.DragEvent<HTMLElement>) => void;
  onDrop: (entry: FileEntry, event: React.DragEvent<HTMLElement>) => void;
  onDragEnd: (event: React.DragEvent<HTMLElement>) => void;
}

const zoomClasses: Record<
  ZoomLevel,
  { width: string; height: string; icon: string; text: string }
> = {
  small: {
    width: "w-[96px]",
    height: "min-h-[88px]",
    icon: "h-11 w-11",
    text: "text-[11px]",
  },
  medium: {
    width: "w-[136px]",
    height: "min-h-[122px]",
    icon: "h-16 w-16",
    text: "text-[12px]",
  },
  large: {
    width: "w-[176px]",
    height: "min-h-[156px]",
    icon: "h-[5.5rem] w-[5.5rem]",
    text: "text-sm",
  },
  xlarge: {
    width: "w-[224px]",
    height: "min-h-[198px]",
    icon: "h-32 w-32",
    text: "text-base",
  },
};

function typeLabel(entry: FileEntry) {
  if (entry.is_dir) return "Folder";
  const ext = entry.name.includes(".")
    ? entry.name.split(".").pop()?.toUpperCase()
    : null;
  return ext ? `${ext} file` : "File";
}

function isImageKind(kind: ReturnType<typeof getFileKind>) {
  return (
    kind === "image" ||
    kind === "image-jpg" ||
    kind === "image-png" ||
    kind === "image-svg"
  );
}

function FileItemInner({
  entry,
  selected,
  dragging,
  dropTarget,
  folderStyle,
  specialDirs,
  viewMode,
  zoom,
  depth = 0,
  treeExpanded,
  treeLoading,
  onToggleTree,
  onSelect,
  onOpen,
  onContextMenu,
  onDragStart,
  onDragOver,
  onDragEnter,
  onDragLeave,
  onDrop,
  onDragEnd,
}: FileItemProps) {
  const kind = getFileKind(entry, specialDirs);
  const z = zoomClasses[zoom];
  const [thumbnailFailed, setThumbnailFailed] = useState(false);
  const thumbnailSrc = useMemo(
    () =>
      !entry.is_dir && isImageKind(kind) && !thumbnailFailed
        ? convertFileSrc(entry.path)
        : null,
    [entry.is_dir, entry.path, kind, thumbnailFailed],
  );

  const handleContext = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      onContextMenu(entry, e.clientX, e.clientY);
    },
    [entry, onContextMenu],
  );

  const commonProps = {
    type: "button" as const,
    draggable: true,
    "data-file-item": "true",
    "data-file-path": entry.path,
    onClick: (e: React.MouseEvent<HTMLButtonElement>) =>
      onSelect(entry, e.metaKey || e.ctrlKey, e.shiftKey),
    onDoubleClick: () => onOpen(entry),
    onContextMenu: handleContext,
    onDragStart: (e: React.DragEvent<HTMLButtonElement>) =>
      onDragStart(entry, e),
    onDragOver: (e: React.DragEvent<HTMLButtonElement>) => onDragOver(entry, e),
    onDragEnter: (e: React.DragEvent<HTMLButtonElement>) =>
      onDragEnter(entry, e),
    onDragLeave: (e: React.DragEvent<HTMLButtonElement>) =>
      onDragLeave(entry, e),
    onDrop: (e: React.DragEvent<HTMLButtonElement>) => onDrop(entry, e),
    onDragEnd: (e: React.DragEvent<HTMLButtonElement>) => onDragEnd(e),
  };

  if (viewMode === "tree") {
    return (
      <button
        {...commonProps}
        className={[
          "grid w-full grid-cols-[minmax(220px,1fr)_150px_110px_90px] items-center gap-3 rounded-lg px-2 py-1.5 text-left text-[12px] transition",
          "border border-transparent hover:bg-white/[0.045]",
          selected && "nodalix-file-selected",
          selected &&
            "border-nodalix-accent/45 bg-white/[0.045] text-nodalix-text ring-1 ring-nodalix-accent/35",
          dragging && "opacity-60",
          dropTarget &&
            "border-nodalix-accent/70 bg-nodalix-accent-dim ring-1 ring-nodalix-accent/50",
        ]
          .filter(Boolean)
          .join(" ")}
      >
        <span
          className="flex min-w-0 items-center gap-2"
          style={{ paddingLeft: depth * 16 }}
        >
          {viewMode === "tree" &&
            (entry.is_dir ? (
              <span
                role="button"
                tabIndex={0}
                className="flex h-5 w-5 shrink-0 items-center justify-center rounded-md text-nodalix-muted transition hover:bg-nodalix-accent-dim hover:text-nodalix-text"
                onClick={(e) => {
                  e.stopPropagation();
                  onToggleTree?.(entry);
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    e.stopPropagation();
                    onToggleTree?.(entry);
                  }
                }}
              >
                {treeLoading ? "…" : treeExpanded ? "▾" : "▸"}
              </span>
            ) : (
              <span className="h-5 w-5 shrink-0" />
            ))}
          {thumbnailSrc ? (
            <img
              src={thumbnailSrc}
              alt=""
              loading="lazy"
              decoding="async"
              draggable={false}
              className="h-7 w-7 shrink-0 rounded-md object-cover shadow-sm ring-1 ring-white/10"
              onError={() => setThumbnailFailed(true)}
            />
          ) : (
            <FileTypeIcon
              kind={kind}
              style={folderStyle}
              className="h-6 w-6 shrink-0 drop-shadow-sm"
            />
          )}
          <span
            className="truncate font-medium text-nodalix-text"
            title={entry.name}
          >
            {entry.name}
          </span>
        </span>
        <span className="truncate text-nodalix-muted">
          {formatTimestamp(entry.modified)}
        </span>
        <span className="truncate text-nodalix-muted">{typeLabel(entry)}</span>
        <span className="truncate text-right text-nodalix-muted">
          {entry.is_dir ? "—" : formatSize(entry.size)}
        </span>
      </button>
    );
  }

  return (
    <button
      {...commonProps}
      className={[
        "group flex flex-col items-center gap-1 rounded-xl px-1.5 py-2 text-center transition-all duration-150",
        "border border-transparent bg-transparent",
        selected && "nodalix-file-selected",
        z.width,
        z.height,
        "hover:bg-nodalix-accent-dim/65",
        selected &&
          "bg-nodalix-accent-dim/65 ring-1 ring-nodalix-accent/50",
        dragging && "scale-[0.98] opacity-60",
        dropTarget &&
          "bg-nodalix-accent-dim ring-2 ring-nodalix-accent/70 shadow-[0_0_0_4px_rgba(203,166,247,0.08)]",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      {thumbnailSrc ? (
        <img
          src={thumbnailSrc}
          alt=""
          loading="lazy"
          decoding="async"
          draggable={false}
          className={`nodalix-image-thumbnail ${z.icon}`}
          onError={() => setThumbnailFailed(true)}
        />
      ) : (
        <FileTypeIcon
          kind={kind}
          style={folderStyle}
          className={`${z.icon} drop-shadow-md`}
        />
      )}
      <span
        className={`line-clamp-2 w-full px-0.5 font-medium leading-tight text-nodalix-text ${z.text}`}
        title={entry.name}
      >
        {entry.name}
      </span>
    </button>
  );
}

export default memo(FileItemInner);
