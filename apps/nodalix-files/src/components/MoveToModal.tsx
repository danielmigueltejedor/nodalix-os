import { useEffect, useState } from "react";
import { listDirectory } from "../lib/files";
import { basename, parentPath } from "../lib/files";
import Modal, { ModalButton } from "./Modal";

interface MoveToModalProps {
  open: boolean;
  startPath: string;
  onClose: () => void;
  onConfirm: (targetDir: string) => void | Promise<void>;
}

export default function MoveToModal({
  open,
  startPath,
  onClose,
  onConfirm,
}: MoveToModalProps) {
  const [cwd, setCwd] = useState(startPath);
  const [dirs, setDirs] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!open) return;
    setCwd(startPath);
  }, [open, startPath]);

  useEffect(() => {
    if (!open) return;
    setLoading(true);
    listDirectory(cwd)
      .then((entries) =>
        setDirs(entries.filter((e) => e.is_dir).map((e) => e.name)),
      )
      .finally(() => setLoading(false));
  }, [open, cwd]);

  return (
    <Modal
      open={open}
      title="Mover a"
      onClose={onClose}
      width="md"
      footer={
        <>
          <ModalButton onClick={onClose}>Cancelar</ModalButton>
          <ModalButton variant="primary" onClick={() => void onConfirm(cwd)}>
            Mover aquí
          </ModalButton>
        </>
      }
    >
      <div className="mb-2 flex items-center gap-2 text-xs text-nodalix-muted">
        {parentPath(cwd) && (
          <button
            type="button"
            className="rounded-md px-2 py-1 hover:bg-nodalix-accent-dim hover:text-nodalix-accent"
            onClick={() => {
              const p = parentPath(cwd);
              if (p) setCwd(p);
            }}
          >
            ↑ Subir
          </button>
        )}
        <span className="truncate">{cwd}</span>
      </div>
      <div className="max-h-56 overflow-auto rounded-lg border border-nodalix-border bg-black/15">
        {loading && <p className="p-3 text-sm text-nodalix-muted">Cargando…</p>}
        {!loading &&
          dirs.map((d) => (
            <button
              key={d}
              type="button"
              className="flex w-full items-center gap-2 px-3 py-2 text-left text-sm hover:bg-nodalix-accent-dim"
              onClick={() => setCwd(`${cwd.replace(/\/$/, "")}/${d}`)}
            >
              <span>📁</span>
              <span>{d}</span>
            </button>
          ))}
        {!loading && dirs.length === 0 && (
          <p className="p-3 text-sm text-nodalix-muted">No hay subcarpetas</p>
        )}
      </div>
      <p className="mt-2 text-xs text-nodalix-muted">
        Destino: {basename(cwd) || cwd}
      </p>
    </Modal>
  );
}
