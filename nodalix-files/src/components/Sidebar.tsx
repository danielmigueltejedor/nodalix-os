import type { SpecialDirs } from "../lib/files";

export interface SidebarLocation {
  id: string;
  label: string;
  path: string;
  icon: "home" | "desktop" | "downloads" | "documents" | "pictures" | "videos" | "music" | "data";
}

interface SidebarProps {
  locations: SidebarLocation[];
  activePath: string;
  onNavigate: (path: string) => void;
}

function SidebarIcon({ icon }: { icon: SidebarLocation["icon"] }) {
  const cls = "h-4 w-4 shrink-0 opacity-90";
  switch (icon) {
    case "home":
      return (
        <svg className={cls} viewBox="0 0 24 24" fill="currentColor" aria-hidden>
          <path d="M12 3l9 8v10h-6v-6H9v6H3V11l9-8z" />
        </svg>
      );
    case "desktop":
      return (
        <svg className={cls} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
          <rect x="3" y="4" width="18" height="13" rx="2" />
          <path d="M8 20h8M12 17v3" />
        </svg>
      );
    case "downloads":
      return (
        <svg className={cls} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
          <path d="M12 3v12m0 0l4-4m-4 4l-4-4M4 17v3h16v-3" />
        </svg>
      );
    case "documents":
      return (
        <svg className={cls} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
          <path d="M7 4h7l5 5v11H7V4z" />
          <path d="M14 4v5h5" />
        </svg>
      );
    case "pictures":
      return (
        <svg className={cls} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
          <rect x="3" y="5" width="18" height="14" rx="2" />
          <circle cx="9" cy="10" r="1.5" fill="currentColor" />
          <path d="M3 16l5-4 4 3 3-2 6 5" />
        </svg>
      );
    case "videos":
      return (
        <svg className={cls} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
          <rect x="2" y="6" width="20" height="12" rx="2" />
          <path d="M10 9l6 3-6 3V9z" fill="currentColor" stroke="none" />
        </svg>
      );
    case "music":
      return (
        <svg className={cls} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
          <circle cx="7" cy="17" r="2" />
          <circle cx="17" cy="15" r="2" />
          <path d="M9 17V7l10-2v10" />
        </svg>
      );
    case "data":
      return (
        <svg className={cls} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
          <ellipse cx="12" cy="6" rx="8" ry="3" />
          <path d="M4 6v6c0 1.7 3.6 3 8 3s8-1.3 8-3V6M4 12v6c0 1.7 3.6 3 8 3s8-1.3 8-3v-6" />
        </svg>
      );
  }
}

export function buildSidebarLocations(dirs: SpecialDirs): SidebarLocation[] {
  const items: SidebarLocation[] = [
    { id: "home", label: "Home", path: dirs.home, icon: "home" },
  ];

  const optional: Array<[string, string | null, SidebarLocation["icon"]]> = [
    ["desktop", dirs.desktop, "desktop"],
    ["downloads", dirs.downloads, "downloads"],
    ["documents", dirs.documents, "documents"],
    ["pictures", dirs.pictures, "pictures"],
    ["videos", dirs.videos, "videos"],
    ["music", dirs.music, "music"],
    ["data", dirs.data, "data"],
  ];

  for (const [id, path, icon] of optional) {
    if (path) {
      items.push({
        id,
        label: id.charAt(0).toUpperCase() + id.slice(1),
        path,
        icon,
      });
    }
  }

  return items;
}

export default function Sidebar({ locations, activePath, onNavigate }: SidebarProps) {
  return (
    <aside className="flex w-52 shrink-0 flex-col gap-1 border-r border-nodalix-border bg-nodalix-surface/80 p-3 backdrop-blur-xl">
      <div className="mb-3 px-2">
        <h1 className="text-sm font-semibold tracking-wide text-nodalix-accent">Nodalix Files</h1>
        <p className="text-[10px] text-nodalix-muted">Wayland file browser</p>
      </div>
      <nav className="flex flex-col gap-0.5">
        {locations.map((loc) => {
          const active = activePath === loc.path || activePath.startsWith(`${loc.path}/`);
          return (
            <button
              key={loc.id}
              type="button"
              onClick={() => onNavigate(loc.path)}
              className={[
                "flex items-center gap-2.5 rounded-xl px-3 py-2 text-left text-sm transition-all duration-150",
                active
                  ? "bg-nodalix-accent-dim text-nodalix-text shadow-sm shadow-nodalix-accent/10"
                  : "text-nodalix-muted hover:bg-white/5 hover:text-nodalix-text",
              ].join(" ")}
            >
              <SidebarIcon icon={loc.icon} />
              <span className="truncate">{loc.label}</span>
            </button>
          );
        })}
      </nav>
    </aside>
  );
}
