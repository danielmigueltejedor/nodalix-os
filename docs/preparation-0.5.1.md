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
  clonado del indicador.
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
cd nxpm && PYTHONPATH=src /usr/bin/python3 -m pytest tests -q
bash tools/test-native-restart.sh
```

No publicar la release ni retocar el tag `0.5.0` desde esta rama.
