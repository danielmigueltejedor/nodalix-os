# Diseño NDL en las apps de Nodalix OS

Nodalix 0.6.0 lleva el lenguaje de diseño de Nodalia
([NDL](https://github.com/danielmigueltejedor/nodalia-design)) a las apps
nativas del sistema. Las que ya son GTK/libadwaita lo adoptan por capas, como
documenta NDL: se cargan sus hojas sobre libadwaita y no se reescribe la
interfaz.

| App | Estado | Dónde |
|---|---|---|
| Ajustes | Capa NDL + islas (barra lateral y contenido) | `gnome/control-center/` (`ndl/`, `cc-nodalix-theme.c`, `apply.py`) |
| Tienda | Capa NDL | repo `tienda`, `src/ndl/` y `src/bz-ndl-theme.c` |
| Monitor del sistema | Pendiente de definir qué app es | — |

## Cómo funciona

- Las hojas generadas por `@nodalia/gtk` se copian **fijadas a un commit**
  (`ndl/PROVENANCE.md`, `ndl/SHA256SUMS`) y se compilan en el binario. Los tests
  comprueban que no se editan a mano.
- Se instalan al arrancar con prioridad `APPLICATION - 50`: por encima del tema
  de libadwaita y por debajo del `style.css` de cada app.
- La hoja de color cambia sola entre claro, oscuro y alto contraste según
  `AdwStyleManager`. Densidad regular.
- Tipografía: Adwaita Sans. `nodalix-overrides.css` apunta las variables de
  fuente de NDL (Inter y Sora) a la del sistema; Nodalix no empaqueta fuentes.
- `NODALIX_NDL=0` desactiva la capa para depurar.

## Actualizar NDL

Copia las ocho hojas de `packages/gtk/crate/src/css/` del commit nuevo en
`gnome/control-center/ndl/` y en `tienda/src/ndl/`, actualiza `PROVENANCE.md` y
`SHA256SUMS` en ambos y mira el resultado en Ajustes y Tienda antes de
fusionar. Las dos copias deben ser idénticas.

## Verificación

Comprobado en GTK 4.22 y libadwaita 1.9 con un arnés propio: las hojas se
analizan sin errores del parser y el cambio claro/oscuro recarga la hoja de
color. El aspecto se revisó con una ventana de prueba con la misma estructura
que Ajustes (barra lateral, filas, interruptores, botones). **Falta revisar a
mano Ajustes y Tienda reales** (paneles con widgets propios, diálogos, el
`style.css` de la Tienda).
