import { memo, useCallback } from "react";
import type { FileEntry } from "../lib/files";
import { getFileKind } from "../lib/files";
import type { FolderStyle } from "../lib/folderCustomization";
import { FileTypeIcon } from "../lib/icons";

interface FileItemProps {
  entry: FileEntry;
  selected: boolean;
  folderStyle?: FolderStyle;
  onSelect: (entry: FileEntry, additive: boolean) => void;
  onOpen: (entry: FileEntry) => void;
  onContextMenu: (entry: FileEntry, x: number, y: number) => void;
}

function FileItemInner({
  entry,
  selected,
  folderStyle,
  onSelect,
  onOpen,
  onContextMenu,
}: FileItemProps) {
  const kind = getFileKind(entry);

  const handleContext = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      onContextMenu(entry, e.clientX, e.clientY);
    },
    [entry, onContextMenu],
  );

  return (
    <button
      type="button"
      className={[
        "group flex w-[104px] flex-col items-center gap-1.5 rounded-xl px-2 py-2.5 text-center transition-all duration-150",
        "border border-transparent bg-transparent",
        "hover:bg-nodalix-accent-dim/80 hover:shadow-md hover:shadow-nodalix-accent/8",
        selected &&
          "bg-nodalix-accent-strong/90 shadow-lg shadow-nodalix-accent/15 ring-1 ring-nodalix-accent/50",
      ]
        .filter(Boolean)
        .join(" ")}
      onClick={(e) => onSelect(entry, e.metaKey || e.ctrlKey)}
      onDoubleClick={() => onOpen(entry)}
      onContextMenu={handleContext}
    >
      <FileTypeIcon kind={kind} style={folderStyle} />
      <span
        className="line-clamp-2 w-full px-0.5 text-[11px] font-medium leading-tight text-nodalix-text"
        title={entry.name}
      >
        {entry.name}
      </span>
    </button>
  );
}

export default memo(FileItemInner);
