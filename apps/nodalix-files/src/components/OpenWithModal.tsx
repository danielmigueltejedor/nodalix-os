import { useEffect, useState } from "react";
import type { OpenWithApp } from "../lib/files";
import { getOpenWithApps } from "../lib/files";
import Modal, { ModalButton } from "./Modal";

interface OpenWithModalProps {
  open: boolean;
  path: string | null;
  onClose: () => void;
  onSelect: (appId: string) => void | Promise<void>;
}

export default function OpenWithModal({
  open,
  path,
  onClose,
  onSelect,
}: OpenWithModalProps) {
  const [apps, setApps] = useState<OpenWithApp[]>([]);

  useEffect(() => {
    if (!open || !path) return;
    getOpenWithApps(path).then(setApps);
  }, [open, path]);

  return (
    <Modal
      open={open}
      title="Abrir con"
      onClose={onClose}
      footer={<ModalButton onClick={onClose}>Cancelar</ModalButton>}
    >
      <div className="flex flex-col gap-1">
        {apps.map((app) => (
          <button
            key={app.id}
            type="button"
            className="rounded-lg px-3 py-2 text-left text-sm hover:bg-nodalix-accent-dim hover:text-nodalix-accent"
            onClick={() => {
              void onSelect(app.id);
              onClose();
            }}
          >
            {app.name}
          </button>
        ))}
      </div>
      <p className="mt-3 text-xs text-nodalix-muted">
        Pendiente: escanear archivos .desktop para detectar más aplicaciones.
      </p>
    </Modal>
  );
}
