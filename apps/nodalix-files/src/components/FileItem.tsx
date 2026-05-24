import { convertFileSrc } from "@tauri-apps/api/core";
import { memo, useCallback, useEffect, useRef, useState } from "react";
import type { FileEntry, SpecialDirs } from "../lib/files";
import {
  formatSize,
  formatTimestamp,
  getFileKind,
  getPdfThumbnail,
  getThumbnailDataUrl,
} from "../lib/files";
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
  renameInitialName?: string | null;
  onToggleTree?: (entry: FileEntry) => void;
  onSelect: (entry: FileEntry, additive: boolean, range: boolean) => void;
  onOpen: (entry: FileEntry) => void;
  onContextMenu: (entry: FileEntry, x: number, y: number) => void;
  onRenameCommit?: (entry: FileEntry, name: string) => Promise<boolean>;
  onRenameCancel?: () => void;
  onPreload?: (entry: FileEntry) => void;
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
  if (entry.is_dir) return "Carpeta";
  const ext = entry.name.includes(".")
    ? entry.name.split(".").pop()?.toUpperCase()
    : null;
  return ext ? `Archivo ${ext}` : "Archivo";
}

function isImageKind(kind: ReturnType<typeof getFileKind>) {
  return (
    kind === "image" ||
    kind === "image-jpg" ||
    kind === "image-png" ||
    kind === "image-svg"
  );
}

const pdfThumbnailRequests = new Map<string, Promise<string | null>>();
const dataThumbnailRequests = new Map<string, Promise<string | null>>();

function cachedPdfThumbnail(path: string) {
  let request = pdfThumbnailRequests.get(path);
  if (!request) {
    request = getPdfThumbnail(path).finally(() => pdfThumbnailRequests.delete(path));
    pdfThumbnailRequests.set(path, request);
  }
  return request;
}

function cachedDataThumbnail(path: string) {
  let request = dataThumbnailRequests.get(path);
  if (!request) {
    request = getThumbnailDataUrl(path).finally(() =>
      dataThumbnailRequests.delete(path),
    );
    dataThumbnailRequests.set(path, request);
  }
  return request;
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
  renameInitialName,
  onToggleTree,
  onSelect,
  onOpen,
  onContextMenu,
  onRenameCommit,
  onRenameCancel,
  onPreload,
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
  const [thumbnailSrc, setThumbnailSrc] = useState<string | null>(null);
  const [thumbnailSource, setThumbnailSource] = useState<"asset" | "data" | null>(
    null,
  );
  const [draftName, setDraftName] = useState(renameInitialName ?? entry.name);
  const [renaming, setRenaming] = useState(false);
  const renameInputRef = useRef<HTMLInputElement>(null);
  const renameCommittedRef = useRef(false);
  const isRenaming = renameInitialName != null;

  useEffect(() => {
    if (!isRenaming) {
      setRenaming(false);
      setDraftName(entry.name);
      renameCommittedRef.current = false;
      return;
    }
    setDraftName(renameInitialName || entry.name);
    renameCommittedRef.current = false;
    requestAnimationFrame(() => {
      const input = renameInputRef.current;
      input?.focus();
      input?.select();
    });
  }, [entry.name, isRenaming, renameInitialName]);
  useEffect(() => {
    setThumbnailFailed(false);
    setThumbnailSrc(null);
    setThumbnailSource(null);
    if (entry.is_dir || (!isImageKind(kind) && kind !== "document-pdf")) return;
    let cancelled = false;

    if (isImageKind(kind)) {
      setThumbnailSrc(convertFileSrc(entry.path));
      setThumbnailSource("asset");
      return () => {
        cancelled = true;
      };
    }

    cachedPdfThumbnail(entry.path)
      .then((thumbnailPath) => {
        if (cancelled) return;
        if (thumbnailPath) {
          setThumbnailSrc(convertFileSrc(thumbnailPath));
          setThumbnailSource("asset");
        }
      })
      .catch(() => {
        if (!cancelled) setThumbnailFailed(true);
      });
    return () => {
      cancelled = true;
    };
  }, [entry.is_dir, entry.path, kind]);

  const handleThumbnailError = useCallback(() => {
    if (thumbnailSource === "data") {
      setThumbnailFailed(true);
      setThumbnailSrc(null);
      return;
    }
    cachedDataThumbnail(entry.path)
      .then((thumbnail) => {
        if (thumbnail) {
          setThumbnailSrc(thumbnail);
          setThumbnailSource("data");
          setThumbnailFailed(false);
        } else {
          setThumbnailFailed(true);
          setThumbnailSrc(null);
        }
      })
      .catch(() => {
        setThumbnailFailed(true);
        setThumbnailSrc(null);
      });
  }, [entry.path, thumbnailSource]);

  const handleContext = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      onContextMenu(entry, e.clientX, e.clientY);
    },
    [entry, onContextMenu],
  );

  const commonProps = {
    role: "button" as const,
    tabIndex: 0,
    draggable: true,
    "data-file-item": "true",
    "data-file-path": entry.path,
    onClick: (e: React.MouseEvent<HTMLElement>) => {
      if (isRenaming) return;
      if (entry.is_dir && e.detail >= 2) {
        e.preventDefault();
        e.stopPropagation();
        onOpen(entry);
        return;
      }
      onSelect(entry, e.metaKey || e.ctrlKey, e.shiftKey);
    },
    onDoubleClick: () => {
      if (!isRenaming && !entry.is_dir) onOpen(entry);
    },
    onMouseEnter: () => onPreload?.(entry),
    onFocus: () => onPreload?.(entry),
    onContextMenu: handleContext,
    onDragStart: (e: React.DragEvent<HTMLElement>) => {
      if (isRenaming) {
        e.preventDefault();
        return;
      }
      onDragStart(entry, e);
    },
    onDragOver: (e: React.DragEvent<HTMLElement>) => onDragOver(entry, e),
    onDragEnter: (e: React.DragEvent<HTMLElement>) =>
      onDragEnter(entry, e),
    onDragLeave: (e: React.DragEvent<HTMLElement>) =>
      onDragLeave(entry, e),
    onDrop: (e: React.DragEvent<HTMLElement>) => onDrop(entry, e),
    onDragEnd: (e: React.DragEvent<HTMLElement>) => onDragEnd(e),
    onKeyDown: (e: React.KeyboardEvent<HTMLElement>) => {
      if (isRenaming) return;
      if (e.key === "Enter") onOpen(entry);
    },
  };

  const commitRename = useCallback(async () => {
    if (!isRenaming || renameCommittedRef.current) return;
    const nextName = draftName.trim();
    if (!nextName || nextName === entry.name) {
      renameCommittedRef.current = true;
      onRenameCancel?.();
      return;
    }
    setRenaming(true);
    const ok = await onRenameCommit?.(entry, nextName);
    setRenaming(false);
    if (ok) {
      renameCommittedRef.current = true;
      onRenameCancel?.();
    }
  }, [draftName, entry, isRenaming, onRenameCancel, onRenameCommit]);

  const renameInput = (
    <input
      ref={renameInputRef}
      value={draftName}
      disabled={renaming}
      className="nodalix-inline-rename"
      draggable={false}
      onClick={(e) => e.stopPropagation()}
      onDoubleClick={(e) => e.stopPropagation()}
      onPointerDown={(e) => e.stopPropagation()}
      onContextMenu={(e) => e.stopPropagation()}
      onChange={(e) => setDraftName(e.target.value)}
      onBlur={() => void commitRename()}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          void commitRename();
        }
        if (e.key === "Escape") {
          e.preventDefault();
          renameCommittedRef.current = true;
          onRenameCancel?.();
        }
      }}
    />
  );

  if (viewMode === "tree") {
    return (
      <div
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
          {thumbnailSrc && !thumbnailFailed ? (
            <img
              src={thumbnailSrc}
              alt=""
              loading="lazy"
              decoding="async"
              draggable={false}
              className={[
                "h-7 w-7 shrink-0 rounded-md shadow-sm ring-1 ring-white/10",
                kind === "document-pdf" ? "object-contain bg-white" : "object-cover",
              ].join(" ")}
              onError={handleThumbnailError}
            />
          ) : (
            <FileTypeIcon
              kind={kind}
              style={folderStyle}
              className="h-6 w-6 shrink-0 drop-shadow-sm"
            />
          )}
          {isRenaming ? (
            renameInput
          ) : (
            <span
              className="nodalix-file-name truncate font-medium text-nodalix-text"
              title={entry.name}
            >
              {entry.name}
            </span>
          )}
        </span>
        <span className="truncate text-nodalix-muted">
          {formatTimestamp(entry.modified)}
        </span>
        <span className="truncate text-nodalix-muted">{typeLabel(entry)}</span>
        <span className="truncate text-right text-nodalix-muted">
          {entry.is_dir ? "—" : formatSize(entry.size)}
        </span>
      </div>
    );
  }

  return (
    <div
      {...commonProps}
      className={[
        "nodalix-file-item group flex flex-col items-center gap-1 rounded-xl px-1.5 py-2 text-center transition-all duration-150",
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
      {thumbnailSrc && !thumbnailFailed ? (
        <img
          src={thumbnailSrc}
          alt=""
          loading="lazy"
          decoding="async"
          draggable={false}
          className={`nodalix-image-thumbnail ${z.icon}`}
          data-thumbnail-kind={kind === "document-pdf" ? "pdf" : "image"}
          onError={handleThumbnailError}
        />
      ) : (
        <FileTypeIcon
          kind={kind}
          style={folderStyle}
          className={`${z.icon} drop-shadow-md`}
        />
      )}
      {isRenaming ? (
        renameInput
      ) : (
        <span
          className={`nodalix-file-name line-clamp-2 w-full px-0.5 font-medium leading-tight text-nodalix-text ${z.text}`}
          title={entry.name}
        >
          {entry.name}
        </span>
      )}
    </div>
  );
}

export default memo(FileItemInner);
