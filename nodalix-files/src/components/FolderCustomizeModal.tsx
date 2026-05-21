import Modal, { ModalButton } from "./Modal";
import { FOLDER_COLORS, FOLDER_ICONS } from "../lib/folderCustomization";

const ICON_LABELS: Record<string, string> = {
  default: "Default",
  star: "Star",
  work: "Work",
  code: "Code",
  terminal: "Terminal",
  projects: "Projects",
  icloud: "iCloud",
  cloud: "Cloud",
  server: "Server",
  shared: "Shared",
  download: "Download",
  photo: "Photos",
  camera: "Camera",
  music: "Music",
  video: "Video",
  documents: "Documents",
  notes: "Notes",
  books: "Books",
  design: "Design",
  archive: "Archive",
  backup: "Backup",
  locked: "Locked",
  shield: "Secure",
  private: "Private",
  apps: "Apps",
  finance: "Finance",
  favorite: "Favorite",
  data: "Data",
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
      title={mode === "color" ? "Folder color" : "Folder icon"}
      onClose={onClose}
      footer={<ModalButton onClick={onClose}>Cancel</ModalButton>}
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
            Reset
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
        Customization is stored in ~/.config/nodalix-files/ and shown only in Nodalix Files.
      </p>
    </Modal>
  );
}
