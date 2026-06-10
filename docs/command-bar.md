# Nodalix Command Bar

Nodalix Command Bar is the main launcher and control surface of Nodalix OS.

It is designed as a Spotlight/Raycast-style interface for Hyprland, focused on fast navigation, system actions and extensibility.

## Goal

Provide one fast interface to access:

- Apps
- Quick actions
- Open windows
- Workspaces
- Wallpapers
- Audio controls
- Network/Bluetooth tools
- Captures and clipboard
- Gaming tools
- Dev tools
- Home Assistant
- Privacy controls
- Optional AI actions

## Default shortcut

```text
Super + Space
```

The shortcut launches:

```text
nodalix-command-bar-toggle
```

The toggle behavior is:

```text
if Command Bar is closed -> open it
if Command Bar is open   -> close it
```

## Web search (Spotlight-style)

When you type at least two characters, Command Bar adds web results via `ranking.py`:

- **Buscar '<query>' en internet** — Google search in the default browser
- **Abrir URL** — if the query looks like a URL (`https://…` or `example.com`)

Web results are **not** forced to the top. Local matches (apps, actions, settings, windows) rank first; web search appears after relevant local results, or first when nothing local matches.

`nodalix-launch-browser` reads `xdg-settings get default-web-browser` and launches that `.desktop` entry (typically Zen on Nodalix). It avoids `xdg-open`, which often opens Chromium when the `x-scheme-handler/https` MIME type points elsewhere.

### Result ranking

Ranking lives in `apps/nodalix-command-bar/src/ranking.py`:

| Match type | Score (approx.) |
|------------|-----------------|
| App exact | 1000 |
| App starts with query | 900 |
| System action exact/starts | 850 |
| Settings exact/starts | 800 |
| App partial | 700 |
| Action/settings partial | 650 |
| Fuzzy local | 400–500 |
| Web when locals exist | 100 |
| Web when no locals | 1000 |

Examples:

- `curs` → Cursor (app), then other locals, then **Buscar 'curs' en internet**
- `wifi` → Wi-Fi action/settings, then web search
- `asdkjhaskjdh` → web search first (no local matches)

Tests: `python3 apps/nodalix-command-bar/tests/test_ranking.py`

## Files

```text
local/bin/nodalix-command-bar
local/bin/nodalix-command-bar-toggle
config/nodalix/actions.json
```

## Runtime config path

At runtime, actions are read from:

```text
~/.config/nodalix/actions.json
```

The repository provides the default version here:

```text
config/nodalix/actions.json
```

`install.sh` links the Nodalix config directory into:

```text
~/.config/nodalix
```

## Action schema

Each action in `actions.json` can define:

```json
{
  "icon": "󰒡",
  "title": "Nodalix Doctor",
  "subtitle": "Diagnóstico del sistema",
  "category": "Sistema",
  "search": "doctor diagnostico sistema logs errores salud",
  "aliases": ["doctor", "health"],
  "command": "alacritty -e nodalix-doctor",
  "requires": ["alacritty"],
  "hide_if_missing": true,
  "danger": false,
  "confirm_message": ""
}
```

## Fields

### `icon`

Nerd Font symbol used for actions, windows and workspaces.

Apps use their real icon from `.desktop` files instead.

### `title`

Visible action name.

### `subtitle`

Secondary description.

### `category`

Used to generate dynamic filter chips.

Examples:

```text
Nodalix
Sistema
Energía
Audio
Red
Capturas
Wallpapers
Gaming
Dev
IA
Home Assistant
Privacidad
```

### `search`

Main searchable text.

### `aliases`

Extra terms for search.

Example:

```json
"aliases": ["ha", "hass", "casa", "domotica"]
```

### `command`

Shell command executed when the action is selected.

### `requires`

List of required binaries.

Example:

```json
"requires": ["pavucontrol"]
```

### `hide_if_missing`

If `true`, the action is hidden when a required binary is missing.

### `danger`

If `true`, Command Bar asks for confirmation before running the action.

Recommended for:

```text
poweroff
reboot
suspend
closing active windows
destructive commands
```

### `confirm_message`

Text shown in the confirmation dialog.

## Current providers

### Apps

Apps are loaded from `.desktop` files in:

```text
~/.local/share/applications
/usr/local/share/applications
/usr/share/applications
```

The Command Bar reads:

```text
Name
Exec
Icon
StartupWMClass
NoDisplay
Hidden
Terminal
```

Apps use their real system icon.

If an app is already open, Command Bar tries to focus the existing Hyprland window instead of launching another instance.

### Actions

Actions are loaded from:

```text
~/.config/nodalix/actions.json
```

Fallback:

```text
config/nodalix/actions.json
```

### Windows

Open windows are read from:

```bash
hyprctl clients -j
```

Long titles are truncated so the UI stays compact.

### Workspaces

Workspaces are read from:

```bash
hyprctl workspaces -j
```

## UI behavior

The Command Bar includes:

- Main search input
- Dynamic category chips
- Horizontal chip scroller
- App/action/window/workspace result list
- Category badges
- Danger badges
- Disabled action badges
- Keyboard navigation
- Single-instance lock

## Keyboard controls

```text
Enter     Run selected item
Escape    Close Command Bar
Up/Down   Move selection
```

## Dynamic chips

Base chips:

```text
Todo
Apps
Acciones
Ventanas
Workspaces
```

Additional chips are generated from action categories in `actions.json`.

## Disabled actions

If an action has missing requirements and `hide_if_missing` is enabled, it is hidden by default.

To show disabled actions:

```bash
touch ~/.config/nodalix/show-disabled-actions
```

To hide them again:

```bash
rm ~/.config/nodalix/show-disabled-actions
```

There is also a Command Bar action:

```text
Mostrar/Ocultar acciones no disponibles
```

## Safety model

Dangerous actions should use:

```json
"danger": true,
"confirm_message": "Vas a apagar el equipo ahora."
```

This prevents accidental execution from keyboard search.

## Current limitations

- Actions are still shell-command based.
- Confirmation dialog uses GTK default style.
- Disabled actions depend only on binary presence, not service state.
- App focusing relies on `.desktop` metadata and Hyprland window class matching.
- Categories are plain strings, not yet strongly typed.
- No plugin/provider API yet.

## Roadmap

### v4 ideas

- Custom Nodalix confirmation popup
- Action editor UI
- Per-action permissions
- Provider modules
- Recent files provider
- Audio devices provider
- Network/Bluetooth provider
- Home Assistant entity provider
- AI command provider
- Fuzzy scoring improvements
- Per-category icons
- Result preview panel
- Command history
- User-defined aliases from UI

## Design principle

The Command Bar should feel like the central nervous system of Nodalix OS:

```text
fast
predictable
keyboard-first
beautiful
safe by default
deeply integrated with Hyprland
```
