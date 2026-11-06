"""Read the AppImage update section without executing the file."""

from __future__ import annotations

import struct
from pathlib import Path

_RUNTIME_LIMIT = 16 * 1024 * 1024
_SECTION_LIMIT = 4096


def is_elf(header: bytes) -> bool:
    return header.startswith(b"\x7fELF")


def update_information(path: Path) -> str:
    """Return the embedded ``.upd_info`` text, or an empty string."""

    try:
        data = _section(path, ".upd_info")
    except (OSError, ValueError):
        return ""
    if not data:
        return ""
    text = data.split(b"\x00", 1)[0].decode("utf-8", "replace").strip()
    if len(text) > 512 or any(ord(char) < 32 for char in text):
        return ""
    return text


def _section(path: Path, wanted: str) -> bytes:
    with path.open("rb") as handle:
        header = handle.read(64)
        if not is_elf(header) or header[5] != 1:
            return b""
        elf_class = header[4]
        if elf_class == 2:
            section_offset = struct.unpack_from("<Q", header, 40)[0]
            section_size, section_count, name_index = struct.unpack_from("<HHH", header, 58)
            entry_size = 64
        elif elf_class == 1:
            section_offset = struct.unpack_from("<I", header, 32)[0]
            section_size, section_count, name_index = struct.unpack_from("<HHH", header, 44)
            entry_size = 40
        else:
            return b""
        if section_count == 0 or section_count > 512 or name_index >= section_count:
            return b""
        if section_size < entry_size or section_offset > _RUNTIME_LIMIT:
            return b""
        table_bytes = section_count * section_size
        if table_bytes > _RUNTIME_LIMIT:
            return b""
        handle.seek(section_offset)
        table = handle.read(table_bytes)
        if len(table) != table_bytes:
            return b""
        names = _section_payload(handle, table, name_index * section_size, elf_class)
        if names is None:
            return b""
        encoded = wanted.encode("ascii")
        for index in range(section_count):
            start = index * section_size
            name_at = struct.unpack_from("<I", table, start)[0]
            if not names.startswith(encoded + b"\x00", name_at) and names[name_at:].split(b"\x00", 1)[0] != encoded:
                continue
            payload = _section_payload(handle, table, start, elf_class)
            return payload or b""
    return b""


def _section_payload(handle, table: bytes, start: int, elf_class: int) -> bytes | None:
    if elf_class == 2:
        offset, size = struct.unpack_from("<QQ", table, start + 24)
    else:
        offset, size = struct.unpack_from("<II", table, start + 16)
    if size == 0 or size > _SECTION_LIMIT or offset > _RUNTIME_LIMIT:
        return None
    handle.seek(offset)
    payload = handle.read(size)
    if len(payload) != size:
        return None
    return payload
