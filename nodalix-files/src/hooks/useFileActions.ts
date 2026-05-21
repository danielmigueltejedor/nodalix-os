import { useCallback, useState } from "react";
import type { FileEntry } from "../lib/files";
import {
  copyPaths,
  createDocument,
  createFolder,
  cutPaths,
  deletePathsPermanently,
  moveTo,
  openPath,
  openTerminalHere,
  openWith,
  pasteInto,
  renamePath,
  trashPaths,
} from "../lib/files";
import { sendWithLocalSend } from "../lib/localsend";

export function useFileActions(onRefresh: (path: string) => Promise<unknown>) {
  const [clipboardActive, setClipboardActive] = useState(false);

  const open = useCallback(async (entry: FileEntry) => {
    if (!entry.is_dir) await openPath(entry.path);
  }, []);

  const openWithApp = useCallback(async (path: string, appId: string) => {
    await openWith(path, appId);
  }, []);

  const rename = useCallback(
    async (oldPath: string, newName: string, cwd: string) => {
      await renamePath(oldPath, newName);
      await onRefresh(cwd);
    },
    [onRefresh],
  );

  const trash = useCallback(
    async (paths: string[], cwd: string) => {
      await trashPaths(paths);
      await onRefresh(cwd);
    },
    [onRefresh],
  );

  const deletePermanently = useCallback(
    async (paths: string[], cwd: string) => {
      await deletePathsPermanently(paths);
      await onRefresh(cwd);
    },
    [onRefresh],
  );

  const copy = useCallback(async (paths: string[]) => {
    await copyPaths(paths);
    setClipboardActive(true);
  }, []);

  const cut = useCallback(async (paths: string[]) => {
    await cutPaths(paths);
    setClipboardActive(true);
  }, []);

  const paste = useCallback(
    async (targetDir: string) => {
      await pasteInto(targetDir);
      setClipboardActive(false);
      await onRefresh(targetDir);
    },
    [onRefresh],
  );

  const move = useCallback(
    async (paths: string[], targetDir: string, cwd: string) => {
      await moveTo(paths, targetDir);
      await onRefresh(cwd);
      if (targetDir !== cwd) await onRefresh(targetDir);
    },
    [onRefresh],
  );

  const newFolder = useCallback(
    async (parent: string, name: string) => {
      const created = await createFolder(parent, name);
      await onRefresh(parent);
      return created;
    },
    [onRefresh],
  );

  const newDocument = useCallback(
    async (parent: string, name: string) => {
      const created = await createDocument(parent, name);
      await onRefresh(parent);
      return created;
    },
    [onRefresh],
  );

  const terminalHere = useCallback(async (path: string) => {
    await openTerminalHere(path);
  }, []);

  const localSend = useCallback(async (paths: string[]) => {
    await sendWithLocalSend(paths);
  }, []);

  return {
    clipboardActive,
    setClipboardActive,
    open,
    openWithApp,
    rename,
    trash,
    deletePermanently,
    copy,
    cut,
    paste,
    move,
    newFolder,
    newDocument,
    terminalHere,
    localSend,
  };
}
