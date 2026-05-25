import { useCallback, useState } from "react";
import {
  convertFileToPath,
  previewRenameWithConversion,
  type RenameConversionPreview,
} from "../lib/conversion";
import { deletePathsPermanently, renamePath } from "../lib/files";

type PendingAction = { kind: "rename" } | { kind: "convert" };

export interface RenameOverwriteState {
  preview: RenameConversionPreview;
  oldPath: string;
  cwd: string;
  action: PendingAction;
}

export function useRenameWithConversion(
  onRefresh: (path: string) => Promise<unknown>,
) {
  const [convertModal, setConvertModal] = useState<RenameConversionPreview | null>(
    null,
  );
  const [convertContext, setConvertContext] = useState<{
    oldPath: string;
    cwd: string;
  } | null>(null);
  const [overwriteModal, setOverwriteModal] = useState<RenameOverwriteState | null>(
    null,
  );
  const [converting, setConverting] = useState(false);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);

  const closeModals = useCallback(() => {
    setConvertModal(null);
    setConvertContext(null);
    setOverwriteModal(null);
    setConverting(false);
    setStatusMessage(null);
  }, []);

  const performRename = useCallback(
    async (oldPath: string, newName: string, cwd: string) => {
      await renamePath(oldPath, newName);
      await onRefresh(cwd);
    },
    [onRefresh],
  );

  const performConvert = useCallback(async (preview: RenameConversionPreview) => {
    setConverting(true);
    setStatusMessage("Convirtiendo…");
    try {
      await convertFileToPath(preview.old_path, preview.dest_path);
      setStatusMessage("Conversión completada");
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setStatusMessage(`Error al convertir: ${msg}`);
      throw e;
    } finally {
      setConverting(false);
    }
  }, []);

  const executeAction = useCallback(
    async (action: PendingAction, preview: RenameConversionPreview, oldPath: string, cwd: string) => {
      if (preview.dest_exists) {
        await deletePathsPermanently([preview.dest_path]);
      }
      if (action.kind === "rename") {
        await performRename(oldPath, preview.new_name, cwd);
      } else {
        await performConvert(preview);
        await onRefresh(cwd);
      }
      closeModals();
    },
    [closeModals, onRefresh, performConvert, performRename],
  );

  const openOverwriteConfirm = useCallback(
    (
      preview: RenameConversionPreview,
      oldPath: string,
      cwd: string,
      action: PendingAction,
    ) => {
      setOverwriteModal({ preview, oldPath, cwd, action });
    },
    [],
  );

  const commitRename = useCallback(
    async (oldPath: string, newName: string, cwd: string): Promise<boolean> => {
      const trimmed = newName.trim();
      if (!trimmed) return false;

      const preview = await previewRenameWithConversion(oldPath, trimmed);

      if (!preview.extension_changed || preview.scenario === "no_extension_change") {
        if (preview.dest_exists) {
          openOverwriteConfirm(preview, oldPath, cwd, { kind: "rename" });
          return false;
        }
        await performRename(oldPath, trimmed, cwd);
        return true;
      }

      if (
        preview.scenario === "convert_available" &&
        !preview.ask_on_extension_change
      ) {
        if (preview.dest_exists) {
          openOverwriteConfirm(preview, oldPath, cwd, { kind: "rename" });
          return false;
        }
        await performRename(oldPath, trimmed, cwd);
        return true;
      }

      setConvertContext({ oldPath, cwd });
      setConvertModal(preview);
      setStatusMessage(null);
      return false;
    },
    [openOverwriteConfirm, performRename],
  );

  const handleRenameOnly = useCallback(async () => {
    if (!convertModal || !convertContext) return;
    const { oldPath, cwd } = convertContext;
    if (convertModal.dest_exists) {
      openOverwriteConfirm(convertModal, oldPath, cwd, { kind: "rename" });
      return;
    }
    await performRename(oldPath, convertModal.new_name, cwd);
    closeModals();
  }, [
    closeModals,
    convertContext,
    convertModal,
    openOverwriteConfirm,
    performRename,
  ]);

  const handleConvert = useCallback(async () => {
    if (!convertModal || !convertContext) return;
    const { oldPath, cwd } = convertContext;
    if (convertModal.dest_exists) {
      openOverwriteConfirm(convertModal, oldPath, cwd, { kind: "convert" });
      return;
    }
    try {
      await performConvert(convertModal);
      await onRefresh(cwd);
      closeModals();
    } catch {
      // Mantener el modal abierto con el mensaje de error.
    }
  }, [
    closeModals,
    convertContext,
    convertModal,
    onRefresh,
    openOverwriteConfirm,
    performConvert,
  ]);

  const confirmOverwrite = useCallback(async () => {
    if (!overwriteModal) return;
    const { preview, oldPath, cwd, action } = overwriteModal;
    try {
      await executeAction(action, preview, oldPath, cwd);
    } catch (e) {
      throw e;
    }
  }, [executeAction, overwriteModal]);

  const cancelOverwrite = useCallback(() => {
    setOverwriteModal(null);
  }, []);

  return {
    commitRename,
    convertModalOpen: convertModal !== null,
    convertModal,
    convertBusy: converting,
    convertStatusMessage: statusMessage,
    closeConvertModal: closeModals,
    handleRenameOnly,
    handleConvert,
    overwriteOpen: overwriteModal !== null,
    overwriteTargetName: overwriteModal?.preview.new_name ?? "",
    confirmOverwrite,
    cancelOverwrite,
  };
}
