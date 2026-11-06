# Repositorio de paquetes `[nodalix]`

Los paquetes de cada release se publican en un repositorio pacman firmado para que
el sistema se actualice con `pacman -Syu` sin descargar archivos a mano.

## Cómo se construye

`tools/build-pacman-repo.sh PAQUETES SALIDA CLAVE` firma cada paquete (`.sig` separada),
crea la base de datos con `repo-add --sign` y deja copias reales de `nodalix.db` y
`nodalix.files` (los alojamientos estáticos no siguen enlaces simbólicos).

El CI lo prueba en cada PR (`signed-repo`): genera una clave desechable, construye el
repositorio con los paquetes del candidato, lo añade a un `pacman.conf` con
`SigLevel = Required DatabaseRequired TrustedOnly`, descarga `nodalix-store` verificando
la firma y comprueba que una base de datos manipulada se rechaza.

## Lo que falta para publicarlo

1. Una clave GnuPG **propia del repositorio** (distinta de la clave minisign del manifiesto)
   guardada como secreto del repositorio de GitHub.
2. Dónde se aloja (GitHub Pages o un servidor propio).
3. Un paquete `nodalix-keyring` con la clave pública y `[nodalix]` en `pacman.conf`.
