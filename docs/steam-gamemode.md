# GameMode automático y perfiles de energía para Steam

Nodalix activa GameMode automáticamente para procesos de juegos de Steam nativos y de
Proton que reconoce mediante `SteamAppId` y su directorio de instalación. No
hay que cambiar las opciones de lanzamiento de Steam.

Mientras quede al menos un juego detectado, el servicio de usuario
`steam-auto-gamemode.service` solicita a `power-profiles-daemon` una reserva
temporal (`HoldProfile`) del perfil `performance`, si está disponible. Esta
reserva no sustituye las preferencias del usuario: al terminar el último
juego, cerrar el servicio o finalizar el proceso, se libera automáticamente.
Una selección manual de otro perfil durante la partida cancela la reserva y
Nodalix **no vuelve a forzar Rendimiento hasta la próxima sesión de juego**.

El script continúa funcionando con GameMode incluso si no está instalado
`power-profiles-daemon` o el perfil rendimiento no está disponible. Una caída
del daemon hace que Nodalix vuelva a solicitarlo cuando se recupere, salvo si
el usuario ya canceló la reserva manualmente.

## Comprobación sin modificar la configuración del equipo

Antes de abrir un juego:

```sh
systemctl --user is-active steam-auto-gamemode.service
powerprofilesctl list
powerprofilesctl get
gamemoded -s
```

Abre un juego y, tras unos cinco segundos:

```sh
gamemoded -s
powerprofilesctl get
powerprofilesctl list-holds
cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor
journalctl --user -u steam-auto-gamemode.service -b --no-pager -n 30
```

En máquinas con el perfil `performance`, deberías ver `gamemode is
active`, `powerprofilesctl get` con `performance` y una reserva de
`com.nodalix.SteamAutoGameMode`. Después de cerrar **todos** los juegos,
comprueba que desaparece la reserva y vuelve el perfil que habías elegido
antes de abrirlos.

La política exacta del governor depende del driver de CPU y de GameMode; no
se presupone `performance` para todos los dispositivos.

## Diagnóstico

- El servicio debe estar activo, aunque `systemctl --user is-enabled` pueda
  mostrar `disabled` cuando la activación procede del enlace vendor de
  `graphical-session.target.wants`.
- La identificación automática revisa los procesos cada cinco segundos.
  No todos los lanzadores especiales se detectan necesariamente.
- Un juego que ya utiliza `gamemoderun` se contabiliza para la reserva
  energética, pero no se modifica su registro GameMode externo.
- Si aparece `GNOME performance hold unavailable`, consulta los permisos,
  perfiles disponibles y estado de `power-profiles-daemon`.
- Si GNOME se queda en `balanced` a pesar del governor `performance`,
  revisa `powerprofilesctl list-holds` y si el usuario canceló manualmente
  la reserva.

El cambio no fuerza `powerprofilesctl set performance` permanentemente ni
necesita privilegios de administrador por juego. Se emplea la API D-Bus
`org.freedesktop.UPower.PowerProfiles.HoldProfile` y `ReleaseProfile`.

Referencia: https://upower.pages.freedesktop.org/power-profiles-daemon/gdbus-org.freedesktop.UPower.PowerProfiles.html
