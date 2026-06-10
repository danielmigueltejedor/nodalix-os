# Bloqueo de sesión (Hyprlock)

## Wrapper único

Todo bloqueo debe pasar por:

```bash
nodalix-lock
```

Ubicación en el repo: `local/bin/nodalix-lock` → `~/.local/bin/nodalix-lock` (symlink).

Instalación rápida (sin sudo):

```bash
./scripts/install-nodalix-lock.sh
```

Instalación completa: `./install.sh` (incluye el script anterior).

`nodalix-system-lock` es un alias que delega en `nodalix-lock`.

## Cómo evita locks duplicados

1. **Flock** en `$XDG_RUNTIME_DIR/nodalix-lock.flock` — serializa intentos simultáneos.
2. **`pgrep -x hyprlock`** — si ya hay hyprlock, sale sin hacer nada.
3. **Un solo daemon idle** — `hypridle` arranca una vez en `default/hypr/autostart.lua`.
4. **`nodalix-idle.service` (swayidle) deshabilitado** — evitaba duplicar timers con hypridle.

Logs: `~/.local/state/nodalix/lock.log`

## hypridle (`config/hypr/hypridle.conf`)

- `lock_cmd` → `nodalix-lock` (único `exec hyprlock`)
- `before_sleep_cmd` → `loginctl lock-session` (no hyprlock directo)
- Listener 300 s → `loginctl lock-session`
- Listener 360 s → solo `dpms off/on` (no bloqueo)

## Atajo manual

`Super + Ctrl + L` → `nodalix-lock`

## Aplicar cambios

```bash
cd ~/Proyectos/nodalix-os
./install.sh   # o enlaza scripts y deshabilita nodalix-idle

systemctl --user disable --now nodalix-idle.service 2>/dev/null || true
pkill -x hypridle; hyprctl dispatch exec -- hypridle   # o reinicia sesión
hyprctl reload
```

## Probar

```bash
nodalix-lock          # debe abrir hyprlock una vez
nodalix-lock            # segunda llamada: no-op
tail -f ~/.local/state/nodalix/lock.log
```

Tras desbloquear, no debería reaparecer hyprlock varias veces.
