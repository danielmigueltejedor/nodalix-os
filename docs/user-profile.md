# Perfil de usuario Nodalix OS

Configuración compartida entre Ajustes, greeter, barra y Control Center.

## Ubicación

| Recurso | Ruta |
|---------|------|
| Perfil JSON | `~/.config/nodalix/user/profile.json` |
| Avatar Nodalix | `~/.config/nodalix/user/avatar.png` |
| Compatibilidad freedesktop | `~/.face` (sincronizado al cambiar avatar) |
| Plantilla en repo | `config/nodalix/user/profile.json.example` |

## Esquema (`profile.json`)

```json
{
  "username": "dani",
  "display_name": null,
  "avatar_path": "/home/dani/.config/nodalix/user/avatar.png",
  "visual": {
    "color_scheme": "dark",
    "accent_preset": "purple",
    "density": "comfortable",
    "corner_radius": "standard",
    "transparency": "glass"
  },
  "shell": {
    "active_bar": "waybar",
    "control_center_compact": false
  }
}
```

Campos `visual.*` se guardan desde Ajustes → Apariencia. Solo el acento está conectado al sistema (`nodalix-accent-color`); el resto queda preparado para fases futuras.

Campos `shell.*` coordinan la migración gradual de la shell:

- `active_bar`: `waybar` o `nodalix-bar`
- `control_center_compact`: preferencia futura del panel

## Módulo compartido

Lógica centralizada en el crate Rust:

`crates/nodalix-profile/`

Consumido por:

- `apps/nodalix-settings`
- `apps/nodalix-greeter`
- `apps/nodalix-bar`
- `apps/nodalix-control-center`

## Cambiar avatar desde Ajustes

1. Abrir **Ajustes de Nodalix** (`nodalix-settings`)
2. Ir a **Perfil y usuarios** (o pulsar la tarjeta de perfil en la barra lateral)
3. Pulsar **Cambiar imagen…**
4. Elegir PNG/JPEG/WebP

El avatar se copia a `~/.config/nodalix/user/avatar.png`, se actualiza `profile.json` y se sincroniza `~/.face`.

## Greeter

Al listar usuarios desde `/etc/passwd`, el greeter resuelve el avatar con:

1. Ruta en `profile.json` / `avatar.png` del home del usuario
2. `/var/lib/AccountsService/icons/<usuario>`
3. `~/.face` y `~/.face.icon`

Si no hay imagen, muestra iniciales con el estilo actual del greeter.

## Barra y Control Center

- **Nodalix Bar** (`nodalix-bar`): avatar circular a la derecha; abre Ajustes. Botón 󰕮 abre Control Center.
- **Control Center** (`nodalix-control-center`): cabecera con avatar, nombre y preferencias visuales del perfil.
- **Launcher de barra** (`nodalix-session-bar`): lee `shell.active_bar`, lanza Nodalix Bar si está activa y vuelve a Waybar si falla.

## Lanzar componentes

```bash
nodalix-settings          # Ajustes del sistema
nodalix-bar                 # Barra propia (prototipo)
nodalix-control-center      # Panel estilo macOS
nodalix-session-bar         # Inicia la barra activa con fallback
nodalix-bar-toggle          # Alterna Nodalix Bar / Waybar
```

## Pendiente (siguiente fase)

- Wayland layer-shell para anclar la barra
- Actualización en vivo de red/audio/Bluetooth en la barra
- Aplicar `visual.color_scheme`, `density`, `transparency` al escritorio
- Cuenta multi-usuario en greeter con avatar por home
- Integración AccountsService vía D-Bus
- Sustitución progresiva de módulos Waybar restantes
