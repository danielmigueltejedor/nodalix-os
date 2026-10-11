# ISO de Nodalix OS

Nodalix publica una ISO x86_64 para cada release estable.

La ISO se genera con Archiso usando exactamente los mismos paquetes que publica
el actualizador de Nodalix. El tag, `VERSION`, el manifiesto, los paquetes y la
ISO deben pertenecer a la misma versión.

La imagen ofrece arranque BIOS y UEFI, sesión live de GNOME Wayland con GDM y el instalador
basado en Archinstall.

La instalación requiere Internet para descargar las dependencias oficiales de
Arch Linux. Nautilus es el gestor de archivos y Sushi proporciona las previsualizaciones. Los paquetes propios de Nodalix se incluyen dentro de la ISO y se
verifican mediante SHA-256 antes de instalarse.

La versión del instalador no está escrita manualmente en el código. Se obtiene
del archivo `VERSION` incluido durante la construcción de la imagen.

## Descargar la ISO completa

Si la imagen supera el límite por archivo de GitHub, la release la ofrece en
partes, conservando todos los paquetes y el firmware. La herramienta incluida
las descarga, reconstruye la ISO y verifica tanto cada parte como la imagen completa:

```sh
curl -fL https://github.com/danielmigueltejedor/nodalix-os/releases/download/0.3.0/nodalix-download-iso.py -o nodalix-download-iso.py
python3 nodalix-download-iso.py --version 0.3.0
```

Requiere Python 3 y espacio para la ISO. El resultado es
`nodalix-0.3.0-x86_64.iso`, listo para grabar en un USB con tu herramienta habitual.
También puedes descargar las partes desde el navegador y unirlas en su orden
numérico (`part01`, `part02`), comprobando después el archivo `.iso.sha256`.
El manifiesto publica el nombre de la imagen, su tamaño, su SHA-256 y los de cada parte.

## Publicación automática

Al publicar un tag estable, el workflow de release ejecuta:

1. Tests.
2. Construcción de paquetes Nodalix.
3. Generación de `nodalix-manifest.json`.
4. Construcción de la ISO mediante Archiso.
5. Verificación SHA-256 de la ISO.
6. División de las imágenes grandes e incorporación de la ISO y sus partes al manifiesto público.
7. Regeneración de `SHA256SUMS`.
8. Publicación de todos los assets en la misma GitHub Release.

Las prereleases pueden publicar paquetes para el canal beta, pero no generan
una ISO instalable estable.

## Assets de una release estable

Una release estable contiene los paquetes `.pkg.tar.zst`, además de:

```text
nodalix-manifest.json
SHA256SUMS
nodalix-X.Y.Z-x86_64.iso (o sus partes .part01, .part02)
nodalix-download-iso.py
nodalix-X.Y.Z-x86_64.iso.sha256
