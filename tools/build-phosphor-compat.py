#!/usr/bin/env python3
"""Build a Phosphor icon font compatible with the shell's saved Nerd glyphs.

Inputs are the official @phosphor-icons/web font/catalog and Nerd Fonts'
glyphnames.json. The output is a renamed MIT-licensed Phosphor derivative; no
JetBrains Mono or Material Design outlines are copied into it.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from fontTools.ttLib import TTFont
from fontTools.ttLib.tables._c_m_a_p import CmapSubtable


ROOT = Path(__file__).resolve().parents[1]
SHELL = ROOT / "shell"
ALIASES = SHELL / "assets/icons/phosphor-aliases.json"
OUTPUT = SHELL / "assets/fonts/NodalixPhosphorCompat.ttf"
MANIFEST = SHELL / "assets/icons/phosphor-compat-manifest.json"


def source_glyphs() -> set[str]:
    return {
        char
        for path in SHELL.rglob("*.qml")
        for char in path.read_text(encoding="utf-8")
        if 0xF0000 <= ord(char) <= 0xFFFFD
    }


def build(font_path: Path, catalog_path: Path, nerd_path: Path) -> dict[str, dict[str, object]]:
    aliases = json.loads(ALIASES.read_text(encoding="utf-8"))
    catalog = json.loads(catalog_path.read_text(encoding="utf-8"))
    phosphor = {
        name.strip(): icon["properties"]["code"]
        for icon in catalog["icons"]
        for name in icon["properties"]["name"].split(",")
    }
    nerd = json.loads(nerd_path.read_text(encoding="utf-8"))
    mdi = {entry["char"]: key.removeprefix("md-").replace("_", "-")
           for key, entry in nerd.items()
           if key.startswith("md-")}

    font = TTFont(font_path)
    unicode_cmap = next(table.cmap for table in font["cmap"].tables if table.isUnicode())
    compat_cmap = dict(unicode_cmap)
    manifest: dict[str, dict[str, object]] = {}
    for char in sorted(source_glyphs(), key=ord):
        mdi_name = mdi.get(char)
        if mdi_name is None:
            raise ValueError(f"Unknown shell glyph U+{ord(char):X}")
        icon_name = aliases.get(mdi_name, mdi_name)
        phosphor_code = phosphor.get(icon_name)
        if phosphor_code is None:
            raise ValueError(f"No Phosphor icon for {mdi_name}: {icon_name}")
        glyph_name = unicode_cmap.get(phosphor_code)
        if glyph_name is None:
            raise ValueError(f"Phosphor font lacks U+{phosphor_code:X}: {icon_name}")
        compat_cmap[ord(char)] = glyph_name
        manifest[f"U+{ord(char):X}"] = {
            "material_name": mdi_name,
            "phosphor_name": icon_name,
            "phosphor_codepoint": f"U+{phosphor_code:X}",
        }

    # Phosphor ships only BMP format-4 subtables; Nerd Font MDI glyphs are in
    # the supplementary PUA and need format 12 for Qt/fontconfig to see them.
    for platform_id, encoding_id in ((0, 4), (3, 10)):
        table = CmapSubtable.newSubtable(12)
        table.platformID = platform_id
        table.platEncID = encoding_id
        table.language = 0
        table.cmap = compat_cmap
        font["cmap"].tables.append(table)

    for record in font["name"].names:
        if record.nameID in (1, 4, 16):
            record.string = "Nodalix Phosphor Compat".encode(record.getEncoding())
        elif record.nameID == 6:
            record.string = "NodalixPhosphorCompat".encode(record.getEncoding())

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    font.save(OUTPUT)
    MANIFEST.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return manifest


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("font", type=Path)
    parser.add_argument("catalog", type=Path)
    parser.add_argument("nerd_glyphnames", type=Path)
    args = parser.parse_args()
    manifest = build(args.font, args.catalog, args.nerd_glyphnames)
    print(f"Mapped {len(manifest)} shell icons to {OUTPUT}")


if __name__ == "__main__":
    main()
