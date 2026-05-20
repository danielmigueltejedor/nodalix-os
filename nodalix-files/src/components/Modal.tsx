import { useEffect, type ReactNode } from "react";
import { createPortal } from "react-dom";

interface ModalProps {
  open: boolean;
  title: string;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
  width?: "sm" | "md" | "lg";
}

export default function Modal({
  open,
  title,
  onClose,
  children,
  footer,
  width = "md",
}: ModalProps) {
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  const w =
    width === "sm" ? "max-w-sm" : width === "lg" ? "max-w-xl" : "max-w-md";

  return createPortal(
    <div className="fixed inset-0 z-[300] flex items-center justify-center p-4">
      <div
        className="absolute inset-0 bg-black/55 backdrop-blur-sm"
        onClick={onClose}
      />
      <div
        className={`nodalix-animate-in relative w-full ${w} rounded-2xl border border-nodalix-border bg-[rgba(24,24,37,0.96)] shadow-2xl shadow-black/60`}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="border-b border-nodalix-border px-4 py-3">
          <h2 className="text-sm font-semibold text-nodalix-text">{title}</h2>
        </div>
        <div className="px-4 py-4">{children}</div>
        {footer && (
          <div className="flex justify-end gap-2 border-t border-nodalix-border px-4 py-3">
            {footer}
          </div>
        )}
      </div>
    </div>,
    document.body,
  );
}

export function ModalButton({
  children,
  onClick,
  variant = "default",
  disabled,
}: {
  children: ReactNode;
  onClick: () => void;
  variant?: "default" | "primary" | "danger";
  disabled?: boolean;
}) {
  const cls =
    variant === "primary"
      ? "bg-nodalix-accent-dim text-nodalix-accent border-nodalix-accent/40 hover:bg-nodalix-accent-strong"
      : variant === "danger"
        ? "text-nodalix-danger border-nodalix-danger/30 hover:bg-nodalix-danger/15"
        : "text-nodalix-muted border-nodalix-border hover:bg-white/5 hover:text-nodalix-text";
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className={`rounded-lg border px-3 py-1.5 text-xs transition ${cls} disabled:opacity-40`}
    >
      {children}
    </button>
  );
}
