import Modal, { ModalButton } from "./Modal";
import { FOLDER_COLORS, FOLDER_ICONS } from "../lib/folderCustomization";

const ICON_LABELS: Record<string, string> = {
  default: "Predeterminado",
  star: "Estrella",
  work: "Trabajo",
  code: "Código",
  terminal: "Terminal",
  projects: "Proyectos",
  icloud: "iCloud",
  cloud: "Nube",
  server: "Servidor",
  shared: "Compartido",
  download: "Descargas",
  photo: "Fotos",
  camera: "Cámara",
  music: "Música",
  video: "Vídeo",
  documents: "Documentos",
  notes: "Notas",
  books: "Libros",
  design: "Diseño",
  archive: "Archivo",
  backup: "Copia de seguridad",
  locked: "Bloqueado",
  shield: "Seguro",
  private: "Privado",
  apps: "Apps",
  finance: "Finanzas",
  favorite: "Favorito",
  data: "Datos",
};

interface FolderCustomizeModalProps {
  open: boolean;
  mode: "color" | "icon";
  onClose: () => void;
  onPick: (value: string) => void | Promise<void>;
}

export default function FolderCustomizeModal({
  open,
  mode,
  onClose,
  onPick,
}: FolderCustomizeModalProps) {
  return (
    <Modal
      open={open}
      title={mode === "color" ? "Color de carpeta" : "Icono de carpeta"}
      onClose={onClose}
      footer={<ModalButton onClick={onClose}>Cancelar</ModalButton>}
    >
      {mode === "color" ? (
        <div className="flex flex-wrap gap-2">
          {FOLDER_COLORS.map((c) => (
            <button
              key={c}
              type="button"
              className="h-9 w-9 rounded-full border-2 border-white/10 transition hover:scale-110"
              style={{ background: c }}
              onClick={() => void onPick(c)}
            />
          ))}
          <button
            type="button"
            className="rounded-lg border border-nodalix-border px-3 py-1 text-xs text-nodalix-muted"
            onClick={() => void onPick("")}
          >
            Restablecer
          </button>
        </div>
      ) : (
        <div className="grid max-h-[360px] grid-cols-3 gap-2 overflow-auto pr-1">
          {FOLDER_ICONS.map((icon) => (
            <button
              key={icon}
              type="button"
              className="rounded-xl border border-nodalix-border px-3 py-2 text-sm hover:bg-nodalix-accent-dim"
              onClick={() => void onPick(icon === "default" ? "" : icon)}
            >
              {ICON_LABELS[icon] ?? icon}
            </button>
          ))}
        </div>
      )}
      <p className="mt-3 text-xs text-nodalix-muted">
        La personalización se guarda en ~/.config/nodalix-files/ y solo se muestra en Nodalix Files.
      </p>
    </Modal>
  );
}
