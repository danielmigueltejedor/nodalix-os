# Preparación de la 0.6.0

La 0.6.0 sustituye cuatro apps de GNOME por apps propias en NDL. Este documento dice qué hay hecho, qué falta para
cambiar de una a otra y qué se comprueba antes de publicar.

| App nueva | Paquete | Sustituye a | Estado |
| --- | --- | --- | --- |
| Ajustes | `nodalix-ajustes` | `nodalix-control-center` (GNOME Settings) | En el conjunto de la release |
| Monitor del sistema | `nodalix-monitor` | `gnome-system-monitor` | En el conjunto de la release |
| Archivador | `nodalix-archivador` (Rust) | `nautilus` y el Archivador de 0.5 | En el conjunto de la release |
| Tienda | `nodalix-store` 0.6.0 | `bazaar` | En el conjunto de la release |
| Brújula | `nodalix-brujula` | `brujula` sin fijar (si estuviera) | 0.2.0, en el conjunto de la release |

Los tres primeros salen de un solo `PKGBUILD` dividido (`packaging/nodalix-native-apps/`), que compila el espacio de
trabajo de `apps/` una vez desde `Cargo.lock`. Cada paquete declara `provides`/`conflicts`/`replaces` del paquete de
GNOME al que sustituye, así que nada que dependa de ese nombre se quita, y lleva los archivos que otros programas siguen
usando para llamarlo: `/usr/bin/gnome-control-center`, `org.gnome.Settings.desktop`, `/usr/bin/gnome-system-monitor`,
`org.gnome.SystemMonitor.desktop`, `/usr/bin/nautilus` y el servicio D-Bus `org.freedesktop.FileManager1`.

## Qué falta en las apps antes del cambio

- **Ajustes** debe abrir una página por línea de comandos y por activación (`nodalix-settings wifi`), porque el Shell, las
  notificaciones y los indicadores de red y Bluetooth lanzan `gnome-control-center <panel>`.
- **Archivador** debe servir `org.freedesktop.FileManager1` (`ShowFolders`, `ShowItems`, `ShowItemProperties`) y aceptar
  `--select`, `--new-window` y `-q`. Sin eso, «Mostrar en la carpeta» de los navegadores, Steam, Discord y Telegram deja
  de funcionar al quitar Nautilus.

## El cambio (una sola PR, porque las instalaciones limpias instalan todo el conjunto a la vez)

1. `VERSION`, componentes y notas de versión pasan a 0.6.0 (`release/components.json`, `docs/releases/v0.6.0.md`).
2. Los tres paquetes entran en el conjunto de la release; salen `nodalix-control-center` y el `nodalix-archivador` de 0.5.
3. `nodalix-gnome` deja de depender de `nautilus`, `nautilus-python` y `nodalix-control-center`; depende de
   `nodalix-ajustes` y `nodalix-archivador`. Se retira la extensión de LocalSend para Nautilus (la trae Archivador).
4. `iso/installer/packages.txt` y `packaging/nodalix-release` cambian igual; también las listas de `tools/build-iso.py`.
5. Carpetas → Archivador ya va en `gnome-mimeapps.list` del paquete `nodalix-archivador` (Daniel: cualquier app que pida el gestor de archivos abre Archivador; `org.freedesktop.FileManager1` cubre «Mostrar en carpeta»). Web → Brújula va en `/etc/xdg/gnome-mimeapps.list` del paquete `nodalix-brujula` (otro directorio, sin choque de ficheros). Los tipos MIME predeterminados van en `gnome-mimeapps.list`, porque la sesión
   es `XDG_CURRENT_DESKTOP=GNOME` y `mimeapps.list` ya lo usa la Tienda.
6. Pruebas: se retiran solo las que verifican el código de GNOME Settings y Nautilus que deja de existir (páginas
   parcheadas de Actualizaciones, galería animada, almacenamiento); la cobertura equivalente ya está en las pruebas de
   las apps de `apps/`. Cambia la política de pruebas, así que lo revisa una persona.

## Antes de publicar

- Instalación limpia, migración desde 0.2.4, historial de actualizaciones y arranque de la ISO, en CI.
- **Actualización 0.5.2 → 0.6.0 en una instalación real**: debe quitar `nodalix-control-center` y `nautilus` en la misma
  transacción, sin dejar el sistema sin gestor de archivos ni Ajustes si se corta a medias.
- Que `gnome-control-center wifi`, `nautilus --select <ruta>` y `gdbus call … FileManager1.ShowItems` funcionen.
- Auditoría de seguridad y rendimiento (primera pasada: `/mnt/project-files/audit/auditoria-0.6.0-apps.md`).
- Firma del manifiesto con minisign (obligatoria desde la 0.5.2).

## Riesgos conocidos

- `nodalia-gtk` está fijado a un commit de la PR nodalia-design#26 (sin fusionar). Si se fusiona con *squash*, hay que
  volver a fijar el commit de `main`; mientras tanto, el commit solo es alcanzable mientras exista la rama de la PR.
- Los trabajos de VM de CI tardan unos 30 minutos y cada empuje a la PR cancela el anterior: agrupar cambios.

## Brújula 0.2.0

`packaging/nodalix-brujula/PKGBUILD` empaqueta la etiqueta v0.2.0 (commit 4f42d39) con checksum real, como la Tienda. Se construye aparte (`dist/build-aside`, `dist/native-apps`) hasta el PR de cambio, donde entra en el conjunto de la release y en `components.json`. Reemplaza a `brujula` y deja `text/html` y `http(s)` apuntando a Brújula mediante `/etc/xdg/gnome-mimeapps.list`, para no chocar con el `gnome-mimeapps.list` de Archivador.
