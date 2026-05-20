import type { FileEntry } from "../lib/files";
import FileItem from "./FileItem";

interface FileGridProps {
  entries: FileEntry[];
  loading: boolean;
  error: string | null;
  selectedPaths: Set<string>;
  onSelect: (entry: FileEntry, additive: boolean) => void;
  onOpen: (entry: FileEntry) => void;
}

export default function FileGrid({
  entries,
  loading,
  error,
  selectedPaths,
  onSelect,
  onOpen,
}: FileGridProps) {
  if (loading) {
    return (
      <div className="flex flex-1 items-center justify-center text-sm text-nodalix-muted">
        Loading…
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-1 items-center justify-center px-6 text-center text-sm text-nodalix-danger">
        {error}
      </div>
    );
  }

  if (entries.length === 0) {
    return (
      <div className="flex flex-1 items-center justify-center text-sm text-nodalix-muted">
        This folder is empty
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-auto p-4">
      <div className="grid grid-cols-[repeat(auto-fill,minmax(120px,1fr))] gap-3">
        {entries.map((entry) => (
          <FileItem
            key={entry.path}
            entry={entry}
            selected={selectedPaths.has(entry.path)}
            onSelect={onSelect}
            onOpen={onOpen}
          />
        ))}
      </div>
    </div>
  );
}
