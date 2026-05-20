import Modal, { ModalButton } from "./Modal";
import { FOLDER_COLORS, FOLDER_ICONS } from "../lib/folderCustomization";

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
        <div className="flex flex-wrap gap-2">
          {FOLDER_ICONS.map((icon) => (
            <button
              key={icon}
              type="button"
              className="rounded-lg border border-nodalix-border px-3 py-2 text-sm capitalize hover:bg-nodalix-accent-dim"
              onClick={() => void onPick(icon === "default" ? "" : icon)}
            >
              {icon}
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
