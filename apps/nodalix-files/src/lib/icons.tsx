import { useId, type CSSProperties } from "react";
import type { FileKind } from "./files";
import type { FolderStyle } from "./folderCustomization";

interface FolderTokens {
  body: string;
  highlight: string;
  icon: string;
  accent: string;
}

const DEFAULT_FOLDER_TOKENS: FolderTokens = {
  body: "rgba(154, 132, 194, 0.9)",
  highlight: "rgba(229, 206, 255, 0.42)",
  icon: "rgba(76, 58, 104, 0.98)",
  accent: "rgba(203, 166, 247, 0.64)",
};

const FOLDER_TOKEN_MAP: Record<string, FolderTokens> = {
  "#89b4fa": {
    body: "rgba(120, 154, 206, 0.9)",
    highlight: "rgba(196, 220, 255, 0.42)",
    icon: "rgba(52, 68, 96, 0.98)",
    accent: "rgba(137, 180, 250, 0.66)",
  },
  "#74c7ec": {
    body: "rgba(108, 164, 196, 0.9)",
    highlight: "rgba(184, 232, 248, 0.42)",
    icon: "rgba(50, 74, 94, 0.98)",
    accent: "rgba(116, 199, 236, 0.64)",
  },
  "#89dceb": {
    body: "rgba(112, 172, 188, 0.9)",
    highlight: "rgba(190, 238, 246, 0.42)",
    icon: "rgba(48, 78, 92, 0.98)",
    accent: "rgba(137, 220, 235, 0.64)",
  },
  "#a6e3a1": {
    body: "rgba(130, 182, 132, 0.9)",
    highlight: "rgba(206, 246, 198, 0.4)",
    icon: "rgba(50, 82, 66, 0.98)",
    accent: "rgba(166, 227, 161, 0.62)",
  },
  "#94e2d5": {
    body: "rgba(120, 182, 172, 0.9)",
    highlight: "rgba(198, 246, 238, 0.4)",
    icon: "rgba(48, 82, 78, 0.98)",
    accent: "rgba(148, 226, 213, 0.62)",
  },
  "#cba6f7": {
    body: "rgba(154, 132, 194, 0.9)",
    highlight: "rgba(229, 206, 255, 0.42)",
    icon: "rgba(72, 60, 96, 0.98)",
    accent: "rgba(203, 166, 247, 0.64)",
  },
  "#b4befe": {
    body: "rgba(142, 150, 206, 0.9)",
    highlight: "rgba(214, 220, 255, 0.42)",
    icon: "rgba(62, 66, 96, 0.98)",
    accent: "rgba(180, 190, 254, 0.64)",
  },
  "#fab387": {
    body: "rgba(198, 142, 98, 0.9)",
    highlight: "rgba(255, 206, 164, 0.4)",
    icon: "rgba(92, 68, 46, 0.98)",
    accent: "rgba(250, 179, 135, 0.64)",
  },
  "#f9e2af": {
    body: "rgba(198, 172, 112, 0.9)",
    highlight: "rgba(255, 236, 184, 0.38)",
    icon: "rgba(88, 78, 48, 0.98)",
    accent: "rgba(249, 226, 175, 0.6)",
  },
  "#f38ba8": {
    body: "rgba(196, 118, 148, 0.9)",
    highlight: "rgba(255, 190, 216, 0.4)",
    icon: "rgba(92, 56, 82, 0.98)",
    accent: "rgba(243, 139, 168, 0.64)",
  },
  "#f5c2e7": {
    body: "rgba(202, 142, 184, 0.9)",
    highlight: "rgba(255, 216, 246, 0.4)",
    icon: "rgba(92, 56, 82, 0.98)",
    accent: "rgba(245, 194, 231, 0.64)",
  },
  "#9399b2": {
    body: "rgba(146, 152, 172, 0.88)",
    highlight: "rgba(198, 204, 222, 0.36)",
    icon: "rgba(62, 66, 78, 0.98)",
    accent: "rgba(166, 173, 200, 0.58)",
  },
};

const folderPalette: Partial<Record<FileKind, { fill: string }>> = {
  "folder-home": { fill: "#cba6f7" },
  "folder-desktop": { fill: "#89b4fa" },
  "folder-downloads": { fill: "#74c7ec" },
  "folder-documents": { fill: "#a6e3a1" },
  "folder-pictures": { fill: "#89dceb" },
  "folder-videos": { fill: "#f38ba8" },
  "folder-music": { fill: "#fab387" },
  trash: { fill: "#9399b2" },
};

const FOLDER_OVERLAY_BOX = {
  x: 15.25,
  y: 21.75,
  width: 21,
  height: 18,
} as const;

const FOLDER_OVERLAY_STROKE = 2.35;

function folderTokens(color?: string | null): FolderTokens {
  if (!color) return DEFAULT_FOLDER_TOKENS;
  return FOLDER_TOKEN_MAP[color.toLowerCase()] ?? DEFAULT_FOLDER_TOKENS;
}

function folderTokenStyle(tokens: FolderTokens): CSSProperties {
  return {
    "--nx-folder-body": tokens.body,
    "--nx-folder-body-highlight": tokens.highlight,
    "--nx-folder-icon-color": tokens.icon,
    "--nx-folder-accent": tokens.accent,
  } as CSSProperties;
}

function SpecialGlyph({ kind, color }: { kind: FileKind; color: string }) {
  switch (kind) {
    case "folder-home":
      return (
        <path
          d="M5 11l7-5.6 7 5.6v8h-4.3v-5.2H9.3V19H5v-8z"
          fill={color}
          fillOpacity="0.9"
        />
      );
    case "folder-desktop":
      return (
        <path
          d="M4 6h16v10H4zM9 20h6M12 16v4"
          stroke={color}
          strokeWidth={FOLDER_OVERLAY_STROKE}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "folder-downloads":
      return (
        <path
          d="M12 4v12M7 11l5 5 5-5M6 20h12"
          stroke={color}
          strokeWidth="2.8"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "folder-documents":
      return (
        <path
          d="M6 3h9l4 4v14H6V3zM15 3v5h4M9 12h7M9 16h7"
          stroke={color}
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "folder-pictures":
      return (
        <path
          d="M4 6h16v13H4zM7 16l4-4 3 3 2-2 4 4M8 9.5h.1"
          stroke={color}
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "folder-videos":
      return (
        <path
          d="M4 7h12v10H4zM16 10l5-3v10l-5-3"
          fill={color}
          fillOpacity="0.9"
        />
      );
    case "folder-music":
      return (
        <path
          d="M7 18.5a3 3 0 100-6 3 3 0 000 6zM17 16a3 3 0 100-6 3 3 0 000 6zM10 15.5V6l10-2v9"
          stroke={color}
          strokeWidth={FOLDER_OVERLAY_STROKE}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    default:
      return null;
  }
}

function CustomFolderGlyph({
  icon,
  color,
}: {
  icon?: string | null;
  color: string;
}) {
  switch (icon) {
    case "star":
      return (
        <path
          d="M12 3.5l2.8 5.5 6.1.9-4.4 4.3 1 6.1-5.5-2.9-5.5 2.9 1-6.1-4.4-4.3 6.1-.9L12 3.5z"
          fill={color}
          fillOpacity="0.92"
        />
      );
    case "work":
      return (
        <path
          d="M4 7h16v12H4zM8.5 7V4h7v3M4 12h16"
          stroke={color}
          strokeWidth={FOLDER_OVERLAY_STROKE}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "code":
      return (
        <path
          d="M8 8l-5 4 5 4M16 8l5 4-5 4M14 6l-4 12"
          stroke={color}
          strokeWidth={FOLDER_OVERLAY_STROKE}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "terminal":
      return (
        <path
          d="M4 6h16v13H4zM7 10l3 2.5L7 15M12 16h5"
          stroke={color}
          strokeWidth={FOLDER_OVERLAY_STROKE}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "projects":
      return (
        <path
          d="M5 7h14v12H5zM8 7V4h8v3M9 11h6M9 15h4"
          stroke={color}
          strokeWidth={FOLDER_OVERLAY_STROKE}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "download":
      return (
        <path
          d="M12 4v12M7 11l5 5 5-5M6 20h12"
          stroke={color}
          strokeWidth="2.8"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "icloud":
      return (
        <path
          d="M5.2 17.8h13.2a4.8 4.8 0 00.5-9.6 6.9 6.9 0 00-13.3-2A5.8 5.8 0 005.2 17.8z"
          fill={color}
          fillOpacity="0.9"
        />
      );
    case "cloud":
      return (
        <path
          d="M4 18h15a4.8 4.8 0 10-.5-9.6A6.6 6.6 0 005.7 6.5 5.8 5.8 0 004 18z"
          stroke={color}
          strokeWidth={FOLDER_OVERLAY_STROKE}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "server":
      return (
        <path
          d="M5 5h14v5H5zM5 14h14v5H5zM8 7.5h.1M8 16.5h.1M12 7.5h4M12 16.5h4"
          stroke={color}
          strokeWidth={FOLDER_OVERLAY_STROKE}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "shared":
      return (
        <path
          d="M8 12a3 3 0 100-6 3 3 0 000 6zM17 10a2.5 2.5 0 100-5 2.5 2.5 0 000 5zM4 20a4 4 0 018 0M13.5 19a3.5 3.5 0 017 0"
          stroke={color}
          strokeWidth="2.1"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "photo":
      return (
        <path
          d="M4 7h16v12H4zM7 16l4-4 3 3 2-2 4 4M8 10h.1"
          stroke={color}
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "camera":
      return (
        <path
          d="M4 8h4l1.5-3h5L16 8h4v11H4zM12 16a3.5 3.5 0 100-7 3.5 3.5 0 000 7z"
          stroke={color}
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "video":
      return (
        <path
          d="M4 7h12v10H4zM16 10l5-3v10l-5-3"
          fill={color}
          fillOpacity="0.9"
        />
      );
    case "documents":
      return (
        <path
          d="M6 4h10l3 3v13H6V4zM16 4v4h3M9 12h7M9 16h7"
          stroke={color}
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "notes":
      return (
        <path
          d="M6 4h12v16H6zM9 8h6M9 12h6M9 16h4"
          stroke={color}
          strokeWidth={FOLDER_OVERLAY_STROKE}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "books":
      return (
        <path
          d="M5 5h5v15H5zM10 5h5v15h-5zM15 6l4-1 3 14-4 1z"
          stroke={color}
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "design":
      return (
        <path
          d="M6 18l4-12 4 12M8 14h4M15 7l3-3 3 3-3 3zM16 16h5"
          stroke={color}
          strokeWidth={FOLDER_OVERLAY_STROKE}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "music":
      return (
        <path
          d="M7 19a2.8 2.8 0 100-5.6A2.8 2.8 0 007 19zM17 16.5a2.8 2.8 0 100-5.6 2.8 2.8 0 000 5.6zM9.8 16.2V6.5l9.2-2v9"
          stroke={color}
          strokeWidth={FOLDER_OVERLAY_STROKE}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "archive":
      return (
        <path
          d="M4 8h16v11H4zM7 8V5h10v3M9 12h6M9 16h6"
          stroke={color}
          strokeWidth={FOLDER_OVERLAY_STROKE}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "backup":
      return (
        <path
          d="M12 5a7 7 0 11-6.3 4M5 5v4h4M12 9v4l3 2"
          stroke={color}
          strokeWidth={FOLDER_OVERLAY_STROKE}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "locked":
      return (
        <path
          d="M5 10h14v10H5zM8 10V7a4 4 0 018 0v3M12 14.5V17"
          stroke={color}
          strokeWidth={FOLDER_OVERLAY_STROKE}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "shield":
      return (
        <path
          d="M12 3l8 3.4v6.2c0 4.4-3.1 7.9-8 9.4-4.9-1.5-8-5-8-9.4V6.4L12 3z"
          fill={color}
          fillOpacity="0.9"
        />
      );
    case "private":
      return (
        <path
          d="M5 11h14v9H5zM8 11V8a4 4 0 018 0v3M10 16h4"
          stroke={color}
          strokeWidth={FOLDER_OVERLAY_STROKE}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "apps":
      return (
        <path
          d="M5 5h5v5H5zM14 5h5v5h-5zM5 14h5v5H5zM14 14h5v5h-5z"
          fill={color}
          fillOpacity="0.9"
        />
      );
    case "finance":
      return (
        <path
          d="M12 4v16M16 7.5c-.8-1-2-1.5-4-1.5-2.4 0-4 1.1-4 2.8 0 4.2 8 1.8 8 6.2 0 1.8-1.8 3-4.2 3-2 0-3.4-.6-4.4-1.8"
          stroke={color}
          strokeWidth="2.2"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "favorite":
      return (
        <path
          d="M12 20s-8-4.7-8-10.1c0-2.4 1.8-4.3 4.1-4.3 1.4 0 2.8.8 3.9 2.2 1.1-1.4 2.5-2.2 3.9-2.2 2.3 0 4.1 1.9 4.1 4.3C20 15.3 12 20 12 20z"
          fill={color}
          fillOpacity="0.9"
        />
      );
    case "data":
      return (
        <path
          d="M4 7c0-2 3.6-3.5 8-3.5S20 5 20 7s-3.6 3.5-8 3.5S4 9 4 7zM4 7v8c0 2 3.6 3.5 8 3.5s8-1.5 8-3.5V7"
          stroke={color}
          strokeWidth={FOLDER_OVERLAY_STROKE}
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    default:
      return null;
  }
}

function FolderIcon({
  kind,
  style,
  className,
}: {
  kind: FileKind;
  style?: FolderStyle;
  className: string;
}) {
  const gradientId = useId();

  if (kind === "trash") {
    return (
      <svg className={className} viewBox="0 0 48 48" fill="none" aria-hidden>
        <path
          d="M15 16h18l-1.3 25a3 3 0 01-3 2.8h-9.4a3 3 0 01-3-2.8L15 16z"
          fill="#9399b2"
          fillOpacity="0.82"
        />
        <path
          d="M13 14h22M20 14l1.5-4h5L28 14M20 21l.7 16M28 21l-.7 16"
          stroke="#cdd6f4"
          strokeWidth="2"
          strokeLinecap="round"
        />
      </svg>
    );
  }

  const palette = folderPalette[kind];
  const sourceColor = style?.color ?? palette?.fill ?? null;
  const tokens = folderTokens(sourceColor);
  const glyphColor = "var(--nx-folder-icon-color, rgba(58, 62, 72, 0.98))";

  return (
    <svg
      className={className}
      style={folderTokenStyle(tokens)}
      viewBox="0 0 48 48"
      fill="none"
      aria-hidden
    >
      <path
        d="M6 14a4 4 0 014-4h10l4 4h18a4 4 0 014 4v20a4 4 0 01-4 4H10a4 4 0 01-4-4V14z"
        fill="var(--nx-folder-body, rgba(128, 134, 148, 0.72))"
      />
      <path
        d="M8 16.5c0-1.4 1.1-2.5 2.5-2.5h11.2l3.4 3.6H42c1.1 0 2 .9 2 2v1.2H6v-2.3c0-1.1.9-2 2-2z"
        fill="var(--nx-folder-body-highlight, rgba(170, 176, 190, 0.28))"
      />
      <path
        d="M7 22h38v14.8A4.2 4.2 0 0140.8 41H10.2A4.2 4.2 0 016 36.8V23a1 1 0 011-1z"
        fill={`url(#${gradientId})`}
        fillOpacity="0.46"
      />
      <path d="M7 20h37" stroke="rgba(255,255,255,0.22)" strokeWidth="1" />
      <svg
        x={FOLDER_OVERLAY_BOX.x}
        y={FOLDER_OVERLAY_BOX.y}
        width={FOLDER_OVERLAY_BOX.width}
        height={FOLDER_OVERLAY_BOX.height}
        viewBox="0 0 24 24"
        overflow="visible"
        preserveAspectRatio="xMidYMid meet"
      >
        <g transform="translate(0 1)">
          <SpecialGlyph kind={kind} color={glyphColor} />
          <CustomFolderGlyph icon={style?.icon} color={glyphColor} />
        </g>
      </svg>
      <defs>
        <linearGradient
          id={gradientId}
          x1="10"
          y1="22"
          x2="38"
          y2="43"
          gradientUnits="userSpaceOnUse"
        >
          <stop stopColor="rgba(255,255,255,0.22)" />
          <stop
            offset="1"
            stopColor="var(--nx-folder-accent, rgba(150,156,170,0.32))"
            stopOpacity="0.42"
          />
        </linearGradient>
      </defs>
    </svg>
  );
}

export function FileTypeIcon({
  kind,
  style,
  className = "h-11 w-11 drop-shadow-md",
}: {
  kind: FileKind;
  style?: FolderStyle;
  className?: string;
}) {
  if (kind === "folder" || kind.startsWith("folder-") || kind === "trash") {
    return <FolderIcon kind={kind} style={style} className={className} />;
  }

  switch (kind) {
    case "disk":
      return (
        <svg className={className} viewBox="0 0 48 48" fill="none" aria-hidden>
          <rect x="8" y="9" width="32" height="30" rx="5" fill="#89b4fa" fillOpacity="0.8" />
          <rect x="12" y="14" width="24" height="15" rx="3" fill="#1e1e2e" fillOpacity="0.32" />
          <path d="M14 35h20M32 33h.1" stroke="#cdd6f4" strokeWidth="2.4" strokeLinecap="round" />
          <path d="M15 19h18" stroke="#cdd6f4" strokeOpacity="0.45" strokeWidth="2" strokeLinecap="round" />
        </svg>
      );
    case "network":
      return (
        <svg className={className} viewBox="0 0 48 48" fill="none" aria-hidden>
          <circle cx="24" cy="24" r="15" fill="#94e2d5" fillOpacity="0.72" />
          <path d="M24 9c5 4 7 9 7 15s-2 11-7 15M24 9c-5 4-7 9-7 15s2 11 7 15M10 24h28M14 16h20M14 32h20" stroke="#1e1e2e" strokeOpacity="0.62" strokeWidth="2" strokeLinecap="round" />
        </svg>
      );
    case "image-jpg":
    case "image-png":
    case "image-svg":
    case "image":
      return (
        <svg className={className} viewBox="0 0 48 48" fill="none" aria-hidden>
          <rect
            x="8"
            y="10"
            width="32"
            height="28"
            rx="4"
            fill="#89dceb"
            fillOpacity="0.75"
          />
          <circle cx="17" cy="19" r="3" fill="#f9e2af" />
          <path d="M8 32l10-8 8 6 6-5 8 9H8z" fill="#74c7ec" />
          {kind !== "image" && (
            <text
              x="24"
              y="43"
              textAnchor="middle"
              fill="#11111b"
              fillOpacity="0.72"
              fontSize="7"
              fontWeight="800"
            >
              {kind === "image-jpg" ? "JPG" : kind === "image-png" ? "PNG" : "SVG"}
            </text>
          )}
        </svg>
      );
    case "video":
      return (
        <svg className={className} viewBox="0 0 48 48" fill="none" aria-hidden>
          <rect
            x="8"
            y="12"
            width="32"
            height="24"
            rx="4"
            fill="#f38ba8"
            fillOpacity="0.75"
          />
          <path d="M22 18l12 6-12 6V18z" fill="#11111b" fillOpacity="0.6" />
        </svg>
      );
    case "document-text":
      return (
        <svg className={className} viewBox="0 0 48 48" fill="none" aria-hidden>
          <path d="M14 6h14l10 10v26a2 2 0 01-2 2H14a2 2 0 01-2-2V8a2 2 0 012-2z" fill="#cdd6f4" fillOpacity="0.82" />
          <path d="M28 6v10h10" fill="#a6adc8" fillOpacity="0.55" />
          <path d="M18 22h15M18 27h12M18 32h15M18 37h9" stroke="#45475a" strokeWidth="2" strokeLinecap="round" />
        </svg>
      );
    case "document-pdf":
      return (
        <svg className={className} viewBox="0 0 48 48" fill="none" aria-hidden>
          <path d="M14 6h14l10 10v26a2 2 0 01-2 2H14a2 2 0 01-2-2V8a2 2 0 012-2z" fill="#f38ba8" fillOpacity="0.86" />
          <path d="M28 6v10h10" fill="#f5c2e7" fillOpacity="0.42" />
          <text x="24" y="34" textAnchor="middle" fill="#11111b" fillOpacity="0.75" fontSize="10" fontWeight="900">PDF</text>
        </svg>
      );
    case "document-markdown":
      return (
        <svg className={className} viewBox="0 0 48 48" fill="none" aria-hidden>
          <path d="M14 6h14l10 10v26a2 2 0 01-2 2H14a2 2 0 01-2-2V8a2 2 0 012-2z" fill="#89b4fa" fillOpacity="0.82" />
          <path d="M28 6v10h10" fill="#b4befe" fillOpacity="0.45" />
          <path d="M18 32V21l5 6 5-6v11M32 21v11M29 29l3 3 3-3" stroke="#1e1e2e" strokeOpacity="0.68" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      );
    case "document-spreadsheet":
      return (
        <svg className={className} viewBox="0 0 48 48" fill="none" aria-hidden>
          <path d="M14 6h14l10 10v26a2 2 0 01-2 2H14a2 2 0 01-2-2V8a2 2 0 012-2z" fill="#a6e3a1" fillOpacity="0.86" />
          <path d="M28 6v10h10" fill="#94e2d5" fillOpacity="0.45" />
          <path d="M17 22h18M17 29h18M17 36h18M23 20v18M29 20v18" stroke="#1e1e2e" strokeOpacity="0.55" strokeWidth="1.7" />
        </svg>
      );
    case "document-presentation":
      return (
        <svg className={className} viewBox="0 0 48 48" fill="none" aria-hidden>
          <path d="M14 6h14l10 10v26a2 2 0 01-2 2H14a2 2 0 01-2-2V8a2 2 0 012-2z" fill="#fab387" fillOpacity="0.86" />
          <path d="M28 6v10h10" fill="#f9e2af" fillOpacity="0.42" />
          <rect x="17" y="23" width="15" height="10" rx="1.5" stroke="#1e1e2e" strokeOpacity="0.62" strokeWidth="2" />
          <path d="M24.5 33v5M20 38h9" stroke="#1e1e2e" strokeOpacity="0.62" strokeWidth="2" strokeLinecap="round" />
        </svg>
      );
    case "cad-dwg":
    case "cad-dxf":
    case "cad":
      return (
        <svg className={className} viewBox="0 0 48 48" fill="none" aria-hidden>
          <path d="M12 7h24a3 3 0 013 3v28a3 3 0 01-3 3H12a3 3 0 01-3-3V10a3 3 0 013-3z" fill="#74c7ec" fillOpacity="0.82" />
          <path d="M14 15h20M14 22h20M14 29h20M18 11v26M27 11v26" stroke="#1e1e2e" strokeOpacity="0.22" strokeWidth="1" />
          <path d="M17 32l8-14 6 10h-5l-2 4h-7z" stroke="#1e1e2e" strokeOpacity="0.68" strokeWidth="2" strokeLinejoin="round" />
          <text x="24" y="44" textAnchor="middle" fill="#11111b" fillOpacity="0.76" fontSize="7" fontWeight="900">
            {kind === "cad-dwg" ? "DWG" : kind === "cad-dxf" ? "DXF" : "CAD"}
          </text>
        </svg>
      );
    case "model-3d":
      return (
        <svg className={className} viewBox="0 0 48 48" fill="none" aria-hidden>
          <path d="M24 6l15 8.5v18L24 41 9 32.5v-18L24 6z" fill="#b4befe" fillOpacity="0.78" />
          <path d="M24 6v17M9 14.5l15 8.5 15-8.5M16 19v9l8 4.5 8-4.5v-9" stroke="#1e1e2e" strokeOpacity="0.58" strokeWidth="2" strokeLinejoin="round" />
        </svg>
      );
    case "document":
      return (
        <svg className={className} viewBox="0 0 48 48" fill="none" aria-hidden>
          <path
            d="M14 6h14l10 10v26a2 2 0 01-2 2H14a2 2 0 01-2-2V8a2 2 0 012-2z"
            fill="#a6e3a1"
            fillOpacity="0.85"
          />
          <path d="M28 6v10h10" fill="#94e2d5" fillOpacity="0.45" />
        </svg>
      );
    case "audio-mp3":
    case "music":
      return (
        <svg className={className} viewBox="0 0 48 48" fill="none" aria-hidden>
          <circle cx="16" cy="34" r="5" fill="#fab387" />
          <circle cx="34" cy="30" r="5" fill="#fab387" />
          <path
            d="M21 34V14l18-4v20"
            stroke="#fab387"
            strokeWidth="3"
            strokeLinecap="round"
          />
          {kind === "audio-mp3" && (
            <text x="25" y="43" textAnchor="middle" fill="#11111b" fillOpacity="0.7" fontSize="7" fontWeight="900">MP3</text>
          )}
        </svg>
      );
    case "archive":
      return (
        <svg className={className} viewBox="0 0 48 48" fill="none" aria-hidden>
          <rect
            x="10"
            y="8"
            width="28"
            height="32"
            rx="3"
            fill="#f9e2af"
            fillOpacity="0.8"
          />
          <path
            d="M10 16h28M10 24h28M10 32h28"
            stroke="#e6c07b"
            strokeWidth="2"
          />
        </svg>
      );
    case "code-python":
    case "code-c":
    case "code-js":
    case "code-rust":
    case "code-shell":
    case "code-generic":
      return (
        <svg className={className} viewBox="0 0 48 48" fill="none" aria-hidden>
          <path d="M14 6h14l10 10v26a2 2 0 01-2 2H14a2 2 0 01-2-2V8a2 2 0 012-2z" fill="#b4befe" fillOpacity="0.82" />
          <path d="M28 6v10h10" fill="#cba6f7" fillOpacity="0.42" />
          <path d="M21 24l-4 4 4 4M27 24l4 4-4 4" stroke="#1e1e2e" strokeOpacity="0.68" strokeWidth="2.1" strokeLinecap="round" strokeLinejoin="round" />
          <text x="24" y="41" textAnchor="middle" fill="#11111b" fillOpacity="0.75" fontSize="7" fontWeight="900">
            {kind === "code-python"
              ? "PY"
              : kind === "code-c"
                ? "C"
                : kind === "code-js"
                  ? "JS"
                  : kind === "code-rust"
                    ? "RS"
                    : kind === "code-shell"
                      ? "SH"
                      : "CODE"}
          </text>
        </svg>
      );
    default:
      return (
        <svg className={className} viewBox="0 0 48 48" fill="none" aria-hidden>
          <path
            d="M14 6h14l10 10v26a2 2 0 01-2 2H14a2 2 0 01-2-2V8a2 2 0 012-2z"
            fill="#9399b2"
            fillOpacity="0.7"
          />
          <path d="M28 6v10h10" fill="#7f849c" fillOpacity="0.4" />
        </svg>
      );
  }
}
