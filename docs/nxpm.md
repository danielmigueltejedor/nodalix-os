# NXPM

NXPM (Nodalix Package Manager) vive en este repositorio, en `nxpm/`, porque es una herramienta del propio sistema. Nodalix Store (repositorio `tienda`) es un cliente que depende de su servicio D-Bus `os.nodalix.Nxpm1`.

Estado: el código y las pruebas están aquí; el paquete de Arch y su inclusión en la ISO están pendientes.

## Requisitos

NXPM necesita Python 3.11.4 o superior. El servicio de sesión (`nxpm daemon`) usa los enlaces de Python para D-Bus y GObject (`dbus-python`, `PyGObject`). El backend Flatpak necesita la introspección de libflatpak, y el de pacman necesita `pyalpm` y `pycman`. La lista exacta de dependencias de Arch se declarará en el PKGBUILD.

## Métodos

La interfaz `os.nodalix.Nxpm1` ofrece:

| Método | Entrada | Salida |
| --- | --- | --- |
| `ApiVersion` | | `u` |
| `ListSources` | | `aa{sv}` |
| `ListInstalled` | `s` fuente, vacía para todas | `aa{sv}` |
| `Search` | `s` texto | `aa{sv}` |
| `GetDetails` | `s` id | `a{sv}` |
| `ListUpdates` | | `aa{sv}` |
| `Install` | `s` id, `s` fuente | `s` transacción |
| `Remove` | `s` id, `s` fuente | `s` transacción |
| `Update` | `s` id | `s` transacción |
| `UpgradeSystem` | | `s` transacción |
| `GetTransaction` | `s` id | `a{sv}` |
| `Cancel` | `s` id | `b` |
| `GetHistory` | `u` límite | `aa{sv}` |
| `Import` | `s` tipo (`appimage` o `deb`), `s` ruta | `s` transacción |
| `GetPermissions` | `s` id Flatpak | `aa{sv}` |
| `ResetPermissions` | `s` id Flatpak de usuario | `aa{sv}` |

La señal `TransactionChanged` envía un `a{sv}` con el estado de la operación.

Los identificadores válidos son:

- `flatpak:user:app/org.ejemplo.App/x86_64/stable`
- `flatpak:system:runtime/org.freedesktop.Platform/x86_64/24.08`
- `pacman:nombre`
- `appimage:Nombre`

Cualquier otro texto se rechaza antes de tocar el sistema.

## Estados

Una transacción está en `pending`, `running`, `succeeded`, `failed` o `cancelled`. Solo se cancela con seguridad mientras sigue en `pending`. Una operación Flatpak ya empezada no se aborta: libflatpak, en la versión usada, no ofrece una cancelación que este servicio pueda garantizar.

Las operaciones se ejecutan de una en una.

## Flatpak

El backend usa `Flatpak.Installation` de la introspección GObject. Busca en las instalaciones de usuario y de sistema, instala y retira referencias del usuario, y detecta actualizaciones con `list_installed_refs_for_update`. Los cambios de la instalación de sistema no se aplican desde el servicio de usuario.

## Pacman

Las consultas usan `pyalpm` con la configuración que lee `pycman` desde `/etc/pacman.conf`, incluidos los repositorios y `IgnorePkg`. No se descarga la base de datos dentro de una búsqueda: se usan las bases que ya tiene el equipo.

`list_updates` compara la versión local con la de los repositorios sincronizados mediante `pyalpm.vercmp`.

Quitar un paquete oficial sí está permitido (`nxpm remove pacman:nombre`). Se ejecuta como root con `pkexec` y el comando interno `nxpm privileged-remove`. libalpm comprueba las dependencias en `prepare`: si otro paquete instalado necesita el que se quiere quitar, la operación termina en error y no cambia nada. No se usa `cascade` ni `recurse`, así que no se borran paquetes adicionales. Antes se guarda el inventario `nombre versión`.

Instalar o actualizar un solo paquete oficial desde la tienda está rechazado. La actualización oficial es `UpgradeSystem`, que llama a `transaction.sysupgrade(False)` y por tanto resuelve el conjunto completo. Ese camino solo se ejecuta como root. Desde la sesión, la tienda lo lanza con `pkexec` y el comando `nxpm privileged-upgrade`. Si la autorización no se concede, la operación termina en error y no cambia el sistema.

Antes de esa actualización se guarda en el estado del usuario un inventario `nombre versión`. Es un registro, no una instantánea de disco ni una vuelta atrás automática. No se asume Btrfs, Snapper ni ningún otro mecanismo.

## Aplicaciones

`~/Applications` es la carpeta de programas que el usuario añade, en el mismo sentido en que macOS usa `/Applications`. No sustituye a pacman ni a Flatpak.

```bash
PYTHONPATH=nxpm/src python -m nxpm import appimage ~/Descargas/Programa.AppImage
PYTHONPATH=nxpm/src python -m nxpm import deb ~/Descargas/programa.deb
PYTHONPATH=nxpm/src python -m nxpm permissions flatpak:user:app/org.ejemplo.App/x86_64/stable
PYTHONPATH=nxpm/src python -m nxpm permissions-reset flatpak:user:app/org.ejemplo.App/x86_64/stable
```

Un AppImage se copia y se registra en el menú del usuario. NXPM lee la sección ELF `.upd_info` sin ejecutar el archivo. Si el canal es `gh-releases-zsync`, Actualizar descarga el `.AppImage` publicado en GitHub por HTTPS y lo sustituye solo si el archivo recibido sigue siendo un ELF. Si el canal es `zsync`, NXPM lo muestra y no lo aplica.

Un `.deb` se abre como archivo `ar`, se desempaqueta `data.tar` y se ignoran los scripts de `control.tar`. No se llama a `dpkg`. Los bits setuid se quitan. El programa queda en un AppDir con un `AppRun` propio. `appimagetool`, si está en el `PATH`, genera además el AppImage; esta instalación de desarrollo no lo tiene, así que el resultado comprobado es el AppDir.

No todos los paquetes Debian pueden funcionar: sus bibliotecas no se instalan en el sistema. NXPM no promete compatibilidad con cualquier `.deb` o `.rpm`. El formato RPM no tiene extractor.

## Permisos Flatpak

`GetPermissions` lee el grupo `Context` del metadato que publicó la aplicación y, en la instalación de usuario, el override local. El riesgo `high` marca acceso al sistema, al directorio personal, a todos los dispositivos o a los buses de sesión y de sistema. `ResetPermissions` borra el archivo de override de esa aplicación y deja exactamente los permisos declarados. No crea permisos nuevos.

## CLI

`nxpm` importa el motor. No reimplementa las operaciones ni llama a la tienda.

```bash
PYTHONPATH=nxpm/src python -m nxpm sources
PYTHONPATH=nxpm/src python -m nxpm search firefox
PYTHONPATH=nxpm/src python -m nxpm updates
PYTHONPATH=nxpm/src python -m nxpm list --source pacman
PYTHONPATH=nxpm/src python -m nxpm daemon
```

`nxpm check` escribe `~/.local/state/nxpm/updates.json` y, si hay avisos disponibles, muestra una notificación. No instala nada.

`nxpm install-session` copia el servicio de usuario y un temporizador de systemd que ejecuta `nxpm check` cada seis horas. El temporizador no queda activado hasta que se ejecute `systemctl --user enable --now nxpm-check.timer`.

## Errores

Los fallos usan códigos estables: `invalid-input`, `not-found`, `unsupported`, `authorization-required`, `disabled` y `failed`. En D-Bus se traducen a nombres `os.nodalix.Nxpm1.Error.*`.
