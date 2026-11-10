# Preparación Nodalix OS 0.5.1

Candidata de mantenimiento sobre la estable 0.5.0. No publica un salto de
GNOME ni sustituye la arquitectura NXPM de `main` por la rama experimental
`feat/nxpm-binary-only-app-migration`.

## Problemas confirmados en instalaciones reales de 0.5.0

1. Una imagen de desarrollo con `VERSION_ID=0.5.0` pero sin el conjunto
   oficial de paquetes se reportaba como `up_to_date`.
2. `nodalix-store` chocaba con archivos de `bazaar` durante `pacman -U`.
3. Una `release.pub` no propiedad de pacman bloqueaba la instalación del
   actualizador.
4. Now Playing Card solo aparecía en el monitor principal; los secundarios
   mostraban los controles multimedia nativos de GNOME.
5. El centro del panel multi-monitor destruía indicadores al aceptar solo
   un hijo.

## Correcciones incluidas en esta candidata

- Estado `repair_needed` cuando faltan paquetes obligatorios del manifiesto
  aunque coincida `VERSION_ID`.
- Migración idempotente `0.5.0-release-key` para claves públicas huérfanas.
- `conflicts`/`replaces` de `bazaar` en `nodalix-store`.
- Parche multi-monitor de Now Playing + `_centerRow` + exclusión de
  clonado del indicador; los indicadores del centro rellenan la altura del
  panel para alinear hover y popups en monitores secundarios.
- Tema híbrido `Nodalix-Icons` (apps Colloid + símbolos Adwaita) como
  predeterminado; cursor Capitaine sigue en `cursor-theme=Nodalix`; ambos
  paquetes fijan el mismo commit de `nodalix-icon-theme`. Migración solo del
  valor legado `Nodalix-Adwaita-Symbols`.
- Endurecimiento NXPM: catálogo atómico, descarga AppImage por streaming,
  comprobación Type-2, transacciones fuera del bucle GLib, rechazo si el
  actualizador o ALPM tienen candado.

## Firma y cadena de confianza (auditoría 0.5.1)

Comprobado en el árbol actual sin cambiar claves privadas:

- Minisign: `updater/release.pub` se empaqueta siempre y el workflow de release
  exige que coincida con la clave de la release estable anterior.
- Manifiesto: `tools/finalize-release.py` firma el manifiesto antes de publicar.
- Pacman/`[nodalix]`: sin cambios; `require_signature` y el keyring existentes
  se mantienen.
- Secure Boot / módulos del kernel: sin cambios de política en esta candidata.
- CI: se rechaza `gh release upload --clobber` sobre una release estable ya
  publicada con artefactos (borradores y prereleases pueden completar subidas).

## Cambios añadidos sobre la candidata (hilo de revisión final)

- `VERSION`, `nodalix-release` y todos los paquetes que se construyen desde el
  árbol (`updater`, `apps`, `nxpm`, `integrations`, `gnome`, `settings`,
  `video-wallpapers`, `gnome-shell`) pasan a 0.5.1. Con `nodalix-gnome` en 0.5.1
  y `VERSION` en 0.5.0 el trabajo `packages` de CI fallaba porque
  `tools/build-packages.sh` nombra los tarballs con `VERSION`.
- Fondos animados: la cuadrícula usa un `GdkPaintable` de 144×108 con recorte
  «cover» (como los fondos fijos). `GtkPicture` pedía el tamaño del JPEG, así que
  cada miniatura de 480 px ocupaba una fila. Las miniaturas empaquetadas e
  importadas comparten ahora la forma 288×216.
- Pantalla de acceso/bloqueo: `nodalix-settings` observa el fondo del escritorio
  y, al salir de un fondo animado, publica la imagen estática una sola vez
  (marcador en `~/.local/state/nodalix/login-background.json`). Las imágenes que
  no son JPEG se convierten antes como usuario, y al detener el vídeo se retira
  su póster del protector de pantalla.
- `sync-login-background` decodifica el JPEG como `nobody` con límites de
  memoria, CPU y archivos; root solo valida y publica el resultado.
- Tienda: nombre visible «Store» traducible (español «Tienda»), icono propio en
  `nodalix-icon-theme`, y los iconos del tema también para entradas leídas de la
  caché y apps no instaladas. Las notificaciones de NXPM usan el mismo nombre.

## Diferido a 0.6.0

- Integración completa de la CLI/UI de la rama `feat/nxpm-binary-only-app-migration`
  (permisos Flatpak gráficos, Applications service) sin sustituir el motor
  D-Bus maduro.
- Mediciones de rendimiento de sesión con perfiles reproducibles en VM
  (inicio, Shell en reposo, indicadores, Steam); no se afirman mejoras sin
  baseline.
- Rotación de claves de release (requiere puente de confianza comprobado).
- Limpieza operativa de candados huérfanos del actualizador en instalaciones
  de desarrollo.

## Validación mínima

```bash
/usr/bin/python3 -m unittest discover -s tests -v
( cd nxpm && PYTHONPATH=src /usr/bin/python3 -m pytest tests -q )
bash tools/test-native-restart.sh
```

No publicar la release ni retocar el tag `0.5.0` desde esta rama.
