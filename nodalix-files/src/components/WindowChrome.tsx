import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ReactNode } from "react";
import type { PlatformInfo } from "../lib/platform";

interface WindowChromeProps {
  platform: PlatformInfo;
}

export default function WindowChrome({ platform }: WindowChromeProps) {
  const win = getCurrentWindow();
  const { window_controls: c } = platform;

  return (
    <div
      data-tauri-drag-region
      className="flex h-7 shrink-0 items-center justify-end border-b border-nodalix-border/70 bg-transparent px-2"
    >
      <div className="flex items-center gap-1">
        {c.show_minimize && (
          <ChromeBtn label="Minimize" onClick={() => void win.minimize()}>
            <span className="mb-2 block h-0.5 w-3 bg-current" />
          </ChromeBtn>
        )}
        {c.show_maximize && (
          <ChromeBtn label="Maximize" onClick={() => void win.toggleMaximize()}>
            <span className="block h-2.5 w-3 border border-current" />
          </ChromeBtn>
        )}
        {c.show_close && (
          <ChromeBtn label="Close" danger onClick={() => void win.close()}>
            <span className="text-sm leading-none">×</span>
          </ChromeBtn>
        )}
      </div>
    </div>
  );
}

function ChromeBtn({
  children,
  label,
  onClick,
  danger,
}: {
  children: ReactNode;
  label: string;
  onClick: () => void;
  danger?: boolean;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      onClick={onClick}
      className={[
        "flex h-6 w-7 items-center justify-center rounded-md text-nodalix-muted transition",
        danger
          ? "hover:bg-nodalix-danger/25 hover:text-nodalix-danger"
          : "hover:bg-white/10 hover:text-nodalix-text",
      ].join(" ")}
    >
      {children}
    </button>
  );
}
