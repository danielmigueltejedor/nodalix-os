import { useEffect, useState } from "react";
import type { PathProperties } from "../lib/files";
import { formatTimestamp, getFileKind, getProperties } from "../lib/files";
import { FileTypeIcon } from "../lib/icons";
import Modal, { ModalButton } from "./Modal";

interface PropertiesModalProps {
  open: boolean;
  path: string | null;
  onClose: () => void;
}

function yesNo(value: boolean) {
  return value ? "Sí" : "No";
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
      title="Propiedades"
      onClose={onClose}
      width="md"
      footer={<ModalButton onClick={onClose}>Cerrar</ModalButton>}
    >
      {loading && <p className="text-sm text-nodalix-muted">Cargando…</p>}
      {error && <p className="text-sm text-nodalix-danger">{error}</p>}
      {props && (
        <div className="flex gap-4">
          <FileTypeIcon
            kind={getFileKind({
              name: props.name,
              path: props.path,
              is_dir: props.kind === "Folder",
              size: props.size,
              modified: props.modified,
              created: props.created,
              icon_kind: null,
            })}
            className="h-16 w-16 shrink-0 drop-shadow-md"
          />
          <dl className="grid flex-1 grid-cols-[112px_1fr] gap-y-2 text-sm">
            <dt className="text-nodalix-muted">Nombre</dt>
            <dd className="truncate text-nodalix-text">{props.name}</dd>
            <dt className="text-nodalix-muted">Tipo</dt>
            <dd>{props.kind_label}</dd>
            {props.extension && (
              <>
                <dt className="text-nodalix-muted">Extensión</dt>
                <dd>.{props.extension}</dd>
              </>
            )}
            {props.mime_type && (
              <>
                <dt className="text-nodalix-muted">MIME</dt>
                <dd className="break-all text-xs text-nodalix-muted">{props.mime_type}</dd>
              </>
            )}
            <dt className="text-nodalix-muted">Tamaño</dt>
            <dd>{props.size_display}</dd>
            <dt className="text-nodalix-muted">Modificado</dt>
            <dd>{formatTimestamp(props.modified)}</dd>
            <dt className="text-nodalix-muted">Creado</dt>
            <dd>{formatTimestamp(props.created)}</dd>
            <dt className="text-nodalix-muted">Accedido</dt>
            <dd>{formatTimestamp(props.accessed)}</dd>
            <dt className="text-nodalix-muted">Permisos</dt>
            <dd>{props.permissions}</dd>
            <dt className="text-nodalix-muted">Solo lectura</dt>
            <dd>{yesNo(props.readonly)}</dd>
            {props.owner != null && (
              <>
                <dt className="text-nodalix-muted">Propietario</dt>
                <dd>{props.owner}</dd>
              </>
            )}
            {props.group != null && (
              <>
                <dt className="text-nodalix-muted">Grupo</dt>
                <dd>{props.group}</dd>
              </>
            )}
            {props.item_count != null && (
              <>
                <dt className="text-nodalix-muted">Elementos</dt>
                <dd>{props.item_count >= 5000 ? "5000+" : props.item_count}</dd>
              </>
            )}
            {props.symlink_target && (
              <>
                <dt className="text-nodalix-muted">Destino</dt>
                <dd className="break-all text-xs text-nodalix-muted">{props.symlink_target}</dd>
              </>
            )}
            <dt className="text-nodalix-muted">Ruta</dt>
            <dd className="break-all text-xs text-nodalix-muted">{props.path}</dd>
          </dl>
        </div>
      )}
    </Modal>
  );
}
