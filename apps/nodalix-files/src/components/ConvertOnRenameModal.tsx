import Modal, { ModalButton } from "./Modal";
import type {
  ConversionCapability,
  RenameConversionPreview,
  RenameConversionScenario,
} from "../lib/conversion";

interface ConvertOnRenameModalProps {
  open: boolean;
  preview: RenameConversionPreview | null;
  busy?: boolean;
  statusMessage?: string | null;
  onClose: () => void;
  onConvert?: () => void | Promise<void>;
  onRenameOnly: () => void | Promise<void>;
}

function scenarioTitle(scenario: RenameConversionScenario): string {
  switch (scenario) {
    case "convert_available":
      return "Convertir archivo";
    case "incompatible":
    case "unavailable":
    case "category_unavailable":
      return "Cambio de extensión";
    default:
      return "Renombrar";
  }
}

function scenarioIntro(preview: RenameConversionPreview): string {
  const from = preview.from_ext ? `.${preview.from_ext}` : "(sin extensión)";
  const to = preview.to_ext ? `.${preview.to_ext}` : "(sin extensión)";
  if (preview.scenario === "convert_available") {
    return `Has cambiado la extensión de ${from} a ${to}. LixFiles puede convertir el archivo para que el formato interno coincida.`;
  }
  if (preview.message) return preview.message;
  return `Has cambiado la extensión de ${from} a ${to}.`;
}

export default function ConvertOnRenameModal({
  open,
  preview,
  busy = false,
  statusMessage,
  onClose,
  onConvert,
  onRenameOnly,
}: ConvertOnRenameModalProps) {
  if (!preview) return null;

  const canConvert =
    preview.scenario === "convert_available" &&
    preview.capability?.available === true;

  const convertLabel =
    preview.capability?.to_ext === "jpg"
      ? "Convertir a JPEG"
      : preview.capability
        ? `Convertir a ${preview.capability.label.split("→").pop()?.trim() ?? preview.capability.to_ext.toUpperCase()}`
        : "Convertir";

  return (
    <Modal
      open={open}
      title={scenarioTitle(preview.scenario)}
      onClose={onClose}
      width="lg"
      footer={
        <>
          <ModalButton onClick={onClose} disabled={busy}>
            Cancelar
          </ModalButton>
          <ModalButton onClick={() => void onRenameOnly()} disabled={busy}>
            Solo renombrar
          </ModalButton>
          {canConvert && onConvert && (
            <ModalButton
              variant="primary"
              onClick={() => void onConvert()}
              disabled={busy}
            >
              {busy ? "Convirtiendo…" : convertLabel}
            </ModalButton>
          )}
        </>
      }
    >
      <p className="text-sm text-nodalix-muted">{scenarioIntro(preview)}</p>
      <dl className="mt-4 space-y-2 text-xs text-nodalix-text">
        <div className="flex gap-2">
          <dt className="w-20 shrink-0 text-nodalix-muted">Origen</dt>
          <dd className="min-w-0 break-all">{preview.old_name}</dd>
        </div>
        <div className="flex gap-2">
          <dt className="w-20 shrink-0 text-nodalix-muted">Destino</dt>
          <dd className="min-w-0 break-all">{preview.new_name}</dd>
        </div>
        {preview.capability && (
          <>
            <div className="flex gap-2">
              <dt className="w-20 shrink-0 text-nodalix-muted">Conversión</dt>
              <dd>{preview.capability.label}</dd>
            </div>
            <div className="flex gap-2">
              <dt className="w-20 shrink-0 text-nodalix-muted">Backend</dt>
              <dd>{preview.capability.backend}</dd>
            </div>
          </>
        )}
      </dl>
      {preview.capability && <CapabilityNotes capability={preview.capability} />}
      {preview.scenario === "unavailable" && preview.capability?.unavailable_reason && (
        <p className="mt-3 text-xs text-nodalix-danger">
          {preview.capability.unavailable_reason}
        </p>
      )}
      {statusMessage && (
        <p className="mt-3 text-xs text-nodalix-muted">{statusMessage}</p>
      )}
      <p className="mt-3 text-[11px] text-nodalix-muted/80">
        Conversiones compatibles: imágenes, texto simple y documentos cuando hay
        herramientas instaladas.
      </p>
    </Modal>
  );
}

function CapabilityNotes({ capability }: { capability: ConversionCapability }) {
  const notes: string[] = [...capability.warnings];
  if (capability.lossy && !notes.some((n) => n.includes("calidad"))) {
    notes.push("Puede perder calidad.");
  }
  if (capability.requires_external) {
    notes.push("Requiere una herramienta externa en el sistema.");
  }
  if (notes.length === 0) return null;
  return (
    <ul className="mt-3 list-disc space-y-1 pl-4 text-xs text-nodalix-muted">
      {notes.map((note) => (
        <li key={note}>{note}</li>
      ))}
    </ul>
  );
}
