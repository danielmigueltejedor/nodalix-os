# Archivador

Archivador es el gestor de archivos de Nodalix OS: Nautilus 51.1 con el lenguaje de diseño
de Nodalia ([NDL](ndl.md)). Vive en este repositorio (`gnome/archivador/`) porque forma parte
del propio sistema.

## Qué cambia respecto a Nautilus

- Capa NDL sobre libadwaita, igual que Ajustes y la Tienda: las hojas fijadas se compilan en
  el binario y se cargan al arrancar, siguiendo claro, oscuro y alto contraste.
- La barra lateral y el contenido son dos islas redondeadas.
- Se llama Archivador (lanzador, AppStream y cabecera de la barra lateral).
- Sigue siendo `nautilus` por dentro: mismo binario, mismo id de aplicación
  (`org.gnome.Nautilus`), mismos nombres D-Bus y misma API de extensiones, así que los portales,
  las extensiones nautilus-python (incluido el menú LocalSend actual) y todo lo que llama a
  `nautilus` siguen funcionando.
- `NODALIX_NDL=0` desactiva la capa para depurar.

## Paquete

`nodalix-archivador` (`packaging/nodalix-archivador/`): `provides` y `conflicts` de `nautilus` y
`libnautilus-extension` (Arch separa la biblioteca de extensiones; Archivador también la lleva). **Sin `replaces`**: de momento es opcional y una actualización del
sistema no cambia el gestor de archivos por su cuenta. Hacerlo el predeterminado (dependencia
de `nodalix-gnome`, ISO, componente de la release) queda para cuando se haya probado.

`tools/build-packages.sh` lo compila (así el CI verifica que construye) pero **no lo mete en
el conjunto de paquetes de la release**: los trabajos de instalación limpia, actualización y
repositorio firmado instalan todo ese conjunto en un mismo sistema, y Archivador choca con
`nautilus`. Para obtenerlo hoy: `python3 tools/build-archivador-package.py` en Arch.

Se prepara con `tools/build-archivador-package.py`: exporta Nautilus y libgxdp de sus commits
fijados, y `apply.py` aplica la capa en `prepare()`.

## Actualizar Nautilus

Cambia `NAUTILUS_TAG`, `NAUTILUS_REV` y, si el `.wrap` de Nautilus lo pide, `LIBGXDP_REV` en
`tools/build-archivador-package.py`, y la versión en el `PKGBUILD`. Si una edición de
`apply.py` ya no encuentra su ancla, falla con el nombre del fichero: adáptala, no la quites.

## Pendiente

- LocalSend integrado en Archivador (hoy es la extensión nautilus-python de `nodalix-gnome`).
- Revisar a mano en un Nautilus real: diálogos, vista de cuadrícula, selector de archivos,
  hoja `style.css` propia de Nautilus frente a NDL.
- Decidir cuándo pasa a ser el gestor predeterminado.

## Verificación

`tests/test_archivador.py` aplica la capa a un árbol mínimo y comprueba hojas, cargador, islas,
nombre y que un ancla ausente falle. El árbol real de Nautilus 51.1 se probó con `apply.py`
(las seis ediciones encuentran su ancla). **No se ha compilado Archivador**: el entorno de
desarrollo no tiene GTK 4.24 ni libadwaita 1.8; lo compila el CI de paquetes.
