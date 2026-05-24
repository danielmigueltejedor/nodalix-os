function buildCrumbs(path: string): { label: string; path: string }[] {
  if (path === "/") return [{ label: "/", path: "/" }];
  const parts = path.split("/").filter(Boolean);
  const crumbs = [{ label: "/", path: "/" }];
  let acc = "";
  for (const part of parts) {
    acc += `/${part}`;
    crumbs.push({ label: part, path: acc });
  }
  return crumbs;
}

interface BreadcrumbsProps {
  path: string;
  pathLabels?: Record<string, string>;
  className?: string;
  compact?: boolean;
  onNavigate: (path: string) => void;
  onCopyPath?: (path: string) => void;
  onPathContextMenu?: (path: string, x: number, y: number) => void;
  onDropPath?: (path: string, dataTransfer: DataTransfer) => void;
  onHoverPath?: (path: string) => void;
  onLeaveHoverPath?: (path: string) => void;
}

export default function Breadcrumbs({
  path,
  pathLabels = {},
  className = "",
  compact = false,
  onNavigate,
  onCopyPath,
  onPathContextMenu,
  onDropPath,
  onHoverPath,
  onLeaveHoverPath,
}: BreadcrumbsProps) {
  const crumbs = buildCrumbs(path);
  return (
    <nav
      className={[
        "nodalix-pathbar flex min-w-0 flex-1 items-center gap-0.5 overflow-hidden text-sm",
        compact && "nodalix-pathbar-compact",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
    >
      {crumbs.map((crumb, i) => {
        const current = i === crumbs.length - 1;
        return (
          <span key={crumb.path} className="flex min-w-0 items-center">
            {i > 0 && <span className="mx-0.5 text-nodalix-muted/50">/</span>}
            <button
              type="button"
              aria-current={current ? "page" : undefined}
              onDragOver={(event) => {
                if (!onDropPath) return;
                event.preventDefault();
                event.dataTransfer.dropEffect = "move";
                onHoverPath?.(crumb.path);
              }}
              onDragEnter={() => onHoverPath?.(crumb.path)}
              onDragLeave={() => onLeaveHoverPath?.(crumb.path)}
              onDrop={(event) => {
                if (!onDropPath) return;
                event.preventDefault();
                event.stopPropagation();
                onDropPath(crumb.path, event.dataTransfer);
              }}
              onContextMenu={(event) => {
                if (!onPathContextMenu && !onCopyPath) return;
                event.preventDefault();
                event.stopPropagation();
                if (onPathContextMenu) {
                  onPathContextMenu(crumb.path, event.clientX, event.clientY);
                } else {
                  onCopyPath?.(crumb.path);
                }
              }}
              className={[
                "truncate rounded-full px-2 py-1 transition",
                current
                  ? "bg-white/[0.045] text-nodalix-text"
                  : "text-nodalix-muted hover:bg-nodalix-accent-dim hover:text-nodalix-accent",
              ].join(" ")}
              onClick={() => {
                onNavigate(crumb.path);
              }}
            >
              {pathLabels[crumb.path] ?? (i === 0 ? "Inicio" : crumb.label)}
            </button>
          </span>
        );
      })}
    </nav>
  );
}
