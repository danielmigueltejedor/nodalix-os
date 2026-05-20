import type { FileEntry } from "../lib/files";
import { formatSize, getFileKind } from "../lib/files";

interface FileItemProps {
  entry: FileEntry;
  selected: boolean;
  onSelect: (entry: FileEntry, additive: boolean) => void;
  onOpen: (entry: FileEntry) => void;
}

function FileIcon({ kind }: { kind: ReturnType<typeof getFileKind> }) {
  const base = "h-10 w-10 drop-shadow-sm";
  switch (kind) {
    case "folder":
      return (
        <svg className={base} viewBox="0 0 48 48" fill="none" aria-hidden>
          <path
            d="M6 14a4 4 0 014-4h10l4 4h18a4 4 0 014 4v20a4 4 0 01-4 4H10a4 4 0 01-4-4V14z"
            fill="#cba6f7"
            fillOpacity="0.85"
          />
          <path d="M6 18h36v2H6z" fill="#f5e0ff" fillOpacity="0.35" />
        </svg>
      );
    case "image":
      return (
        <svg className={base} viewBox="0 0 48 48" fill="none" aria-hidden>
          <rect x="8" y="10" width="32" height="28" rx="4" fill="#89dceb" fillOpacity="0.7" />
          <circle cx="17" cy="19" r="3" fill="#f9e2af" />
          <path d="M8 32l10-8 8 6 6-5 8 9H8z" fill="#74c7ec" />
        </svg>
      );
    case "video":
      return (
        <svg className={base} viewBox="0 0 48 48" fill="none" aria-hidden>
          <rect x="8" y="12" width="32" height="24" rx="4" fill="#f38ba8" fillOpacity="0.75" />
          <path d="M22 18l12 6-12 6V18z" fill="#11111b" fillOpacity="0.65" />
        </svg>
      );
    case "document":
      return (
        <svg className={base} viewBox="0 0 48 48" fill="none" aria-hidden>
          <path d="M14 6h14l10 10v26a2 2 0 01-2 2H14a2 2 0 01-2-2V8a2 2 0 012-2z" fill="#a6e3a1" fillOpacity="0.8" />
          <path d="M28 6v10h10" fill="#94e2d5" fillOpacity="0.5" />
          <path d="M16 26h16M16 32h12" stroke="#1e1e2e" strokeWidth="2" strokeLinecap="round" />
        </svg>
      );
    case "music":
      return (
        <svg className={base} viewBox="0 0 48 48" fill="none" aria-hidden>
          <circle cx="16" cy="34" r="5" fill="#fab387" />
          <circle cx="34" cy="30" r="5" fill="#fab387" />
          <path
            d="M21 34V14l18-4v20"
            stroke="#fab387"
            strokeWidth="3"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      );
    case "archive":
      return (
        <svg className={base} viewBox="0 0 48 48" fill="none" aria-hidden>
          <rect x="10" y="8" width="28" height="32" rx="3" fill="#f9e2af" fillOpacity="0.75" />
          <path d="M10 16h28M10 24h28M10 32h28" stroke="#e6c07b" strokeWidth="2" />
        </svg>
      );
    default:
      return (
        <svg className={base} viewBox="0 0 48 48" fill="none" aria-hidden>
          <path d="M14 6h14l10 10v26a2 2 0 01-2 2H14a2 2 0 01-2-2V8a2 2 0 012-2z" fill="#9399b2" fillOpacity="0.65" />
          <path d="M28 6v10h10" fill="#7f849c" fillOpacity="0.45" />
        </svg>
      );
  }
}

export default function FileItem({ entry, selected, onSelect, onOpen }: FileItemProps) {
  const kind = getFileKind(entry);

  return (
    <button
      type="button"
      className={[
        "group flex flex-col items-center gap-2 rounded-2xl border p-3 text-center transition-all duration-200",
        "hover:-translate-y-0.5 hover:border-nodalix-accent/40 hover:bg-nodalix-accent-dim hover:shadow-lg hover:shadow-nodalix-accent/10",
        selected
          ? "border-nodalix-accent/60 bg-nodalix-accent-dim shadow-md shadow-nodalix-accent/15"
          : "border-transparent bg-nodalix-card/40",
      ].join(" ")}
      onClick={(e) => onSelect(entry, e.metaKey || e.ctrlKey)}
      onDoubleClick={() => onOpen(entry)}
    >
      <FileIcon kind={kind} />
      <span className="line-clamp-2 w-full text-xs font-medium text-nodalix-text" title={entry.name}>
        {entry.name}
      </span>
      {!entry.is_dir && (
        <span className="text-[10px] text-nodalix-muted">{formatSize(entry.size)}</span>
      )}
    </button>
  );
}
