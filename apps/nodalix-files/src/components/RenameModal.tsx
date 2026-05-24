import { useEffect, useState } from "react";
import Modal, { ModalButton } from "./Modal";

interface RenameModalProps {
  open: boolean;
  initialName: string;
  onClose: () => void;
  onConfirm: (name: string) => void | Promise<void>;
  error?: string | null;
}

export default function RenameModal({
  open,
  initialName,
  onClose,
  onConfirm,
  error,
}: RenameModalProps) {
  const [name, setName] = useState(initialName);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (open) setName(initialName);
  }, [open, initialName]);

  const submit = async () => {
    if (!name.trim()) return;
    setBusy(true);
    try {
      await onConfirm(name.trim());
      onClose();
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      open={open}
      title="Renombrar"
      onClose={onClose}
      footer={
        <>
          <ModalButton onClick={onClose}>Cancelar</ModalButton>
          <ModalButton variant="primary" onClick={() => void submit()} disabled={busy}>
            Renombrar
          </ModalButton>
        </>
      }
    >
      <input
        autoFocus
        value={name}
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && void submit()}
        className="w-full rounded-lg border border-nodalix-border bg-black/20 px-3 py-2 text-sm text-nodalix-text outline-none focus:border-nodalix-accent/50"
      />
      {error && <p className="mt-2 text-xs text-nodalix-danger">{error}</p>}
    </Modal>
  );
}
