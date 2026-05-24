#!/usr/bin/env fontforge
import fontforge
import psMat
import sys
import os

if len(sys.argv) < 4:
    print("Uso: fontforge -script add-nodalix-glyph.py FUENTE.ttf ICONO.svg SALIDA.ttf")
    sys.exit(1)

font_path = sys.argv[1]
svg_path = sys.argv[2]
out_path = sys.argv[3]

codepoint = int(os.environ.get("NODALIX_CODEPOINT", "0xE00B"), 16)
glyph_name = "nodalix-logo"

family = "JetBrainsMono Nerd Font Nodalix"
fullname = "JetBrainsMono Nerd Font Nodalix Regular"
fontname = "JetBrainsMonoNerdFontNodalix-Regular"

font = fontforge.open(font_path)

# Renombrar fuente internamente
font.familyname = family
font.fullname = fullname
font.fontname = fontname

# Limpiar nombres SFNT viejos todo lo posible
font.sfnt_names = (
    ("English (US)", "Family", family),
    ("English (US)", "SubFamily", "Regular"),
    ("English (US)", "Fullname", fullname),
    ("English (US)", "PostScriptName", fontname),
    ("English (US)", "Preferred Family", family),
    ("English (US)", "Preferred Styles", "Regular"),
    ("English (US)", "Compatible Full", fullname),
    ("English (US)", "UniqueID", fontname),
)

glyph = font.createChar(codepoint, glyph_name)
glyph.clear()

glyph.importOutlines(svg_path)
glyph.removeOverlap()
glyph.correctDirection()

em = font.em
target_width = int(em * 0.86)
target_height = int(em * 0.86)

x_min, y_min, x_max, y_max = glyph.boundingBox()
w = x_max - x_min
h = y_max - y_min

if w <= 0 or h <= 0:
    print("ERROR: el SVG no generó contornos válidos.")
    sys.exit(1)

scale = min(target_width / w, target_height / h)
glyph.transform(psMat.scale(scale))

x_min, y_min, x_max, y_max = glyph.boundingBox()
w = x_max - x_min
h = y_max - y_min

advance = em
x_offset = (advance - w) / 2 - x_min
y_offset = (em - h) / 2 - y_min - int(em * 0.14)

glyph.transform(psMat.translate(x_offset, y_offset))

glyph.width = advance
glyph.vwidth = em

glyph.removeOverlap()
glyph.correctDirection()
glyph.round()

font.generate(out_path)
font.close()

print(f"Fuente generada: {out_path}")
print(f"Glifo Nodalix asignado a U+{codepoint:04X}")
