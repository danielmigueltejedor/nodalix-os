import { useEffect, useState } from "react";
import type { PathProperties } from "../lib/files";
import { formatTimestamp, getProperties } from "../lib/files";
import { FileTypeIcon } from "../lib/icons";
import Modal, { ModalButton } from "./Modal";

interface PropertiesModalProps {
  open: boolean;
  path: string | null;
  onClose: () => void;
}

export default function PropertiesModal({ open, path, onClose }: PropertiesModalProps) {
  const [props, setProps] = useState<PathProperties | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!open || !path) return;
    setLoading(true);
    setError(null);
    getProperties(path)
      .then(setProps)
      .catch((e) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setLoading(false));
  }, [open, path]);

  return (
    <Modal
      open={open}
      title="Properties"
      onClose={onClose}
      width="md"
      footer={<ModalButton onClick={onClose}>Close</ModalButton>}
    >
      {loading && <p className="text-sm text-nodalix-muted">Loading…</p>}
      {error && <p className="text-sm text-nodalix-danger">{error}</p>}
      {props && (
        <div className="flex gap-4">
          <FileTypeIcon
            kind={props.kind === "Folder" ? "folder" : "generic"}
            className="h-16 w-16 shrink-0 drop-shadow-md"
          />
          <dl className="grid flex-1 grid-cols-[100px_1fr] gap-y-2 text-sm">
            <dt className="text-nodalix-muted">Name</dt>
            <dd className="truncate text-nodalix-text">{props.name}</dd>
            <dt className="text-nodalix-muted">Type</dt>
            <dd>{props.kind}</dd>
            <dt className="text-nodalix-muted">Size</dt>
            <dd>{props.size_display}</dd>
            <dt className="text-nodalix-muted">Modified</dt>
            <dd>{formatTimestamp(props.modified)}</dd>
            <dt className="text-nodalix-muted">Created</dt>
            <dd>{formatTimestamp(props.created)}</dd>
            <dt className="text-nodalix-muted">Permissions</dt>
            <dd>{props.permissions}</dd>
            {props.item_count != null && (
              <>
                <dt className="text-nodalix-muted">Items</dt>
                <dd>{props.item_count >= 5000 ? "5000+" : props.item_count}</dd>
              </>
            )}
            <dt className="text-nodalix-muted">Path</dt>
            <dd className="break-all text-xs text-nodalix-muted">{props.path}</dd>
          </dl>
        </div>
      )}
    </Modal>
  );
}
