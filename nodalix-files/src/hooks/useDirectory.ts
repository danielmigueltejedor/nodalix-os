import { useCallback, useRef, useState } from "react";
import type { FileEntry } from "../lib/files";
import { listDirectory } from "../lib/files";
import { perfMark } from "./usePerformanceMarks";

const CACHE_TTL_MS = 30_000;
const cache = new Map<string, { at: number; entries: FileEntry[] }>();

export function useDirectory() {
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const requestId = useRef(0);

  const load = useCallback(async (path: string, opts?: { silent?: boolean }) => {
    const id = ++requestId.current;
    const cached = cache.get(path);
    if (cached && Date.now() - cached.at < CACHE_TTL_MS) {
      setEntries(cached.entries);
      setError(null);
      setLoading(false);
      perfMark(`list_cache_hit ${path}`);
      return cached.entries;
    }

    if (!opts?.silent) setLoading(true);
    setError(null);
    perfMark(`list_start ${path}`);

    try {
      const list = await listDirectory(path);
      if (id !== requestId.current) return list;
      cache.set(path, { at: Date.now(), entries: list });
      setEntries(list);
      perfMark(`list_done ${path} (${list.length})`);
      return list;
    } catch (e) {
      if (id !== requestId.current) return [];
      const msg = e instanceof Error ? e.message : String(e);
      setEntries([]);
      setError(msg);
      perfMark(`list_error ${path}`);
      return [];
    } finally {
      if (id === requestId.current && !opts?.silent) setLoading(false);
    }
  }, []);

  const invalidate = useCallback((path?: string) => {
    if (path) cache.delete(path);
    else cache.clear();
  }, []);

  return { entries, loading, error, load, invalidate, setError };
}
