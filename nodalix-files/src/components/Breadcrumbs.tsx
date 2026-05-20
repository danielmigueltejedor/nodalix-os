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
  onNavigate: (path: string) => void;
}

export default function Breadcrumbs({ path, onNavigate }: BreadcrumbsProps) {
  const crumbs = buildCrumbs(path);
  return (
    <nav className="flex min-w-0 flex-1 flex-wrap items-center gap-0.5 text-sm">
      {crumbs.map((crumb, i) => (
        <span key={crumb.path} className="flex min-w-0 items-center">
          {i > 0 && <span className="mx-0.5 text-nodalix-muted/60">/</span>}
          <button
            type="button"
            className="truncate rounded-md px-1.5 py-0.5 text-nodalix-muted transition hover:bg-nodalix-accent-dim hover:text-nodalix-accent"
            onClick={() => onNavigate(crumb.path)}
          >
            {crumb.label}
          </button>
        </span>
      ))}
    </nav>
  );
}
