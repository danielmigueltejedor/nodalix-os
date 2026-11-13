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

## Cómo se publica

El repositorio vive en GitHub Pages: `https://getnodalia.github.io/nodalix-os/x86_64`.
Lo publica el workflow **Publish pacman repository**, que solo se ejecuta a mano
(`workflow_dispatch`) indicando el id de una ejecución de CI cuyos paquetes
(`nodalix-0.4.0-candidate`) hayan pasado todas las pruebas. `tools/ci-publish-repo.sh`:

1. importa la clave privada del secreto `NODALIX_REPO_GPG_KEY` (y, si existe, su
   contraseña en `NODALIX_REPO_GPG_PASSPHRASE`) y comprueba que su huella es la del
   keyring (`3DE029728CDE1828E18B437930D356AC0F6AB1CF`);
2. se niega a publicar si los paquetes superan 900 MB (el límite de Pages es 1 GB);
3. firma los paquetes y la base de datos con `tools/build-pacman-repo.sh`.

Los secretos de firma deben estar en el *environment* `nodalix-repo-signing`
(Settings → Environments), restringido a la rama `main`, y no como secretos del
repositorio: así ninguna rama modificada puede ejecutar el workflow con la clave. El
workflow además comprueba que el id indicado es una ejecución correcta del CI de este
repositorio en `main` antes de firmar nada.

Para activarlo hay que habilitar Pages en el repositorio (Settings → Pages → Source:
GitHub Actions). Si algún día los paquetes no caben, el plan B es alojarlos en un release
fijo de GitHub y mantener solo la base de datos en Pages.

## Lo que falta

- Añadir `[nodalix]` (con `SigLevel = Required DatabaseRequired TrustedOnly`) al
  `pacman.conf` de los sistemas; `nodalix-keyring` ya instala la clave que lo verifica.
