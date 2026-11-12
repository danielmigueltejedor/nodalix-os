# Archivador

Nautilus 51.1 (commit `14015deb303ae645e411bd32ba485e71233e1164`, etiqueta `51.1`)
de https://github.com/GNOME/nautilus (GPL-3.0-or-later), con la capa de Nodalix.
Más datos en `docs/archivador.md`.

- `apply.py` convierte un árbol de Nautilus sin tocar en Archivador. Cada edición se ancla
  en texto exacto del original y falla si no lo encuentra.
- `nautilus-nodalix-theme.c/.h` cargan las hojas NDL al arrancar (misma capa que Ajustes).
- `nodalix-overrides.css` es lo propio de Nodalix sobre NDL (fuente del sistema, islas).
- Las hojas NDL no se copian aquí: se leen de `gnome/control-center/ndl/`.

Se construye con `tools/build-archivador-package.py`; libgxdp va fijado al commit que declara
`subprojects/libgxdp.wrap` de Nautilus.
