import Modal, { ModalButton } from "./Modal";

interface ConfirmModalProps {
  open: boolean;
  title: string;
  message: string;
  confirmLabel?: string;
  onClose: () => void;
  onConfirm: () => void | Promise<void>;
}

export default function ConfirmModal({
  open,
  title,
  message,
  confirmLabel = "Confirm",
  onClose,
  onConfirm,
}: ConfirmModalProps) {
  return (
    <Modal
      open={open}
      title={title}
      onClose={onClose}
      footer={
        <>
          <ModalButton onClick={onClose}>Cancel</ModalButton>
          <ModalButton variant="danger" onClick={() => void onConfirm()}>
            {confirmLabel}
          </ModalButton>
        </>
      }
    >
      <p className="text-sm text-nodalix-muted">{message}</p>
    </Modal>
  );
}
