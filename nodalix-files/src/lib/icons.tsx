import type { FileKind } from "./files";
import type { FolderStyle } from "./folderCustomization";

export function FileTypeIcon({
  kind,
  style,
  className = "h-11 w-11 drop-shadow-md",
}: {
  kind: FileKind;
  style?: FolderStyle;
  className?: string;
}) {
  if (kind === "folder") {
    const fill = style?.color ?? "#cba6f7";
    return (
      <svg className={className} viewBox="0 0 48 48" fill="none" aria-hidden>
        <path
          d="M6 14a4 4 0 014-4h10l4 4h18a4 4 0 014 4v20a4 4 0 01-4 4H10a4 4 0 01-4-4V14z"
          fill={fill}
          fillOpacity="0.9"
        />
        <path d="M6 18h36v2H6z" fill="#fff" fillOpacity="0.2" />
        {style?.icon === "star" && (
          <path
            d="M24 26l2.5 5 5.5.8-4 3.9.9 5.5-4.9-2.6-4.9 2.6.9-5.5-4-3.9 5.5-.8L24 26z"
            fill="#f9e2af"
          />
        )}
      </svg>
    );
  }

  switch (kind) {
    case "image":
      return (
        <svg className={className} viewBox="0 0 48 48" fill="none" aria-hidden>
          <rect x="8" y="10" width="32" height="28" rx="4" fill="#89dceb" fillOpacity="0.75" />
          <circle cx="17" cy="19" r="3" fill="#f9e2af" />
          <path d="M8 32l10-8 8 6 6-5 8 9H8z" fill="#74c7ec" />
        </svg>
      );
    case "video":
      return (
        <svg className={className} viewBox="0 0 48 48" fill="none" aria-hidden>
          <rect x="8" y="12" width="32" height="24" rx="4" fill="#f38ba8" fillOpacity="0.75" />
          <path d="M22 18l12 6-12 6V18z" fill="#11111b" fillOpacity="0.6" />
        </svg>
      );
    case "document":
      return (
        <svg className={className} viewBox="0 0 48 48" fill="none" aria-hidden>
          <path d="M14 6h14l10 10v26a2 2 0 01-2 2H14a2 2 0 01-2-2V8a2 2 0 012-2z" fill="#a6e3a1" fillOpacity="0.85" />
          <path d="M28 6v10h10" fill="#94e2d5" fillOpacity="0.45" />
        </svg>
      );
    case "music":
      return (
        <svg className={className} viewBox="0 0 48 48" fill="none" aria-hidden>
          <circle cx="16" cy="34" r="5" fill="#fab387" />
          <circle cx="34" cy="30" r="5" fill="#fab387" />
          <path d="M21 34V14l18-4v20" stroke="#fab387" strokeWidth="3" strokeLinecap="round" />
        </svg>
      );
    case "archive":
      return (
        <svg className={className} viewBox="0 0 48 48" fill="none" aria-hidden>
          <rect x="10" y="8" width="28" height="32" rx="3" fill="#f9e2af" fillOpacity="0.8" />
          <path d="M10 16h28M10 24h28M10 32h28" stroke="#e6c07b" strokeWidth="2" />
        </svg>
      );
    default:
      return (
        <svg className={className} viewBox="0 0 48 48" fill="none" aria-hidden>
          <path d="M14 6h14l10 10v26a2 2 0 01-2 2H14a2 2 0 01-2-2V8a2 2 0 012-2z" fill="#9399b2" fillOpacity="0.7" />
          <path d="M28 6v10h10" fill="#7f849c" fillOpacity="0.4" />
        </svg>
      );
  }
}
