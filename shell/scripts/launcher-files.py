#!/usr/bin/env python3
"""Resident filename index: disk traversal never runs on the shell UI thread."""
import json
import os
from pathlib import Path
import sys
import threading
import time
import unicodedata

SKIP = {"node_modules", "__pycache__", "venv", "build", "target", "vendor"}

def normalize(value):
    return "".join(c for c in unicodedata.normalize("NFKD", value.casefold())
                   if not unicodedata.combining(c))

def scan(home):
    entries = []
    for directory, dirs, files in os.walk(home, followlinks=False):
        dirs[:] = sorted(d for d in dirs if not d.startswith(".") and d not in SKIP
                         and not Path(directory, d).is_symlink())
        for name in files:
            if name.startswith("."):
                continue
            path = Path(directory, name)
            entries.append((normalize(name), str(path)))
    return entries

def search(entries, query):
    words = normalize(query).split()
    if len(normalize(query).strip()) < 2:
        return []
    matches = (e for e in entries if all(word in e[0] for word in words))
    import heapq
    best = heapq.nsmallest(12, matches, key=lambda e: (not e[0].startswith(normalize(query)), len(e[0]), e[1]))
    return [{"_nodalixKind": "file", "_nodalixIconName": "text-x-generic-symbolic",
             "name": Path(path).name, "path": path, "url": Path(path).as_uri()}
            for _, path in best if Path(path).is_file()]

def main():
    home = Path.home()
    entries = []
    latest = {"query": "", "id": 0}
    lock = threading.Lock()
    def respond(request):
        result = search(entries, request["query"])
        with lock:
            if request != latest:
                return
            print(json.dumps({**request, "results": result}), flush=True)
    def refresh():
        nonlocal entries
        while True:
            entries = scan(home)
            with lock:
                request = latest.copy()
            respond(request)
            time.sleep(300)
    threading.Thread(target=refresh, daemon=True).start()
    for line in sys.stdin:
        try:
            request = json.loads(line)
            if not isinstance(request.get("query"), str):
                continue
            with lock:
                latest = request
            respond(request)
        except (ValueError, KeyError, TypeError):
            continue

if __name__ == "__main__":
    main()
