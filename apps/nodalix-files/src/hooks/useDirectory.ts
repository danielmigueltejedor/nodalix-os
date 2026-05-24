import { useCallback, useRef, useState } from "react";
import type { FileEntry } from "../lib/files";
import { listDirectory, normalizePath } from "../lib/files";
import { perfMark } from "./usePerformanceMarks";

const CACHE_TTL_MS = 30_000;
const CACHE_MAX = 80;
const cache = new Map<string, { at: number; entries: FileEntry[] }>();
const inFlight = new Map<string, Promise<FileEntry[]>>();

function writeCache(key: string, entries: FileEntry[]) {
  if (cache.has(key)) cache.delete(key);
  cache.set(key, { at: Date.now(), entries });
  while (cache.size > CACHE_MAX) {
    const oldest = cache.keys().next().value;
    if (!oldest) break;
    cache.delete(oldest);
  }
}

export function useDirectory() {
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const requestId = useRef(0);

  const load = useCallback(async (path: string, opts?: { silent?: boolean }) => {
    const id = ++requestId.current;
    const key = normalizePath(path);
    const cached = cache.get(key);
    if (cached) {
      setEntries(cached.entries);
      setError(null);
      setLoading(false);
      const fresh = Date.now() - cached.at < CACHE_TTL_MS;
      perfMark(`${fresh ? "list_cache_hit" : "list_cache_stale"} ${key}`);
      if (fresh) return cached.entries;
    }

    if (!opts?.silent && !cached) setLoading(true);
    setError(null);
    perfMark(`list_start ${key}`);

    try {
      let request = inFlight.get(key);
      if (!request) {
        request = listDirectory(key).finally(() => inFlight.delete(key));
        inFlight.set(key, request);
      }
      const list = await request;
      if (id !== requestId.current) return list;
      writeCache(key, list);
      setEntries(list);
      perfMark(`list_done ${key} (${list.length})`);
      return list;
    } catch (e) {
      if (id !== requestId.current) return [];
      if (cached) return cached.entries;
      const msg = e instanceof Error ? e.message : String(e);
      setEntries([]);
      setError(msg);
      perfMark(`list_error ${key}`);
      return [];
    } finally {
      if (id === requestId.current && !opts?.silent) setLoading(false);
    }
  }, []);

  const preload = useCallback(async (path: string) => {
    const key = normalizePath(path);
    const cached = cache.get(key);
    if (cached && Date.now() - cached.at < CACHE_TTL_MS) return cached.entries;
    try {
      let request = inFlight.get(key);
      if (!request) {
        request = listDirectory(key).finally(() => inFlight.delete(key));
        inFlight.set(key, request);
      }
      const list = await request;
      writeCache(key, list);
      perfMark(`list_preload_done ${key} (${list.length})`);
      return list;
    } catch {
      return cached?.entries ?? [];
    }
  }, []);

  const invalidate = useCallback((path?: string) => {
    if (path) cache.delete(normalizePath(path));
    else cache.clear();
  }, []);

  return { entries, loading, error, load, preload, invalidate, setError };
}
