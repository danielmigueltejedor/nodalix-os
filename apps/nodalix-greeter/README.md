# nodalix-greeter

Custom GTK4/libadwaita greetd greeter for Nodalix OS.

It is designed to replace ReGreet visually while keeping ReGreet available as a manual fallback. The UI follows the Nodalix Hyprlock look: wallpaper background, dark overlay, large centered clock, Spanish date, Nodalix brand, glass password pill, bottom-left user selector, and a loading overlay while greetd starts the session.

The Nodalix logo is loaded from the shared SVG brand asset. It does not use a private Nerd Font glyph, because greetd can run with a restricted font environment and render private codepoints incorrectly.

## Demo mode

```sh
cargo run -- --demo
```

Demo mode does not connect to greetd. It uses fake users and accepts `nodalix` or `password` as a fake password so the loading state can be previewed.

## greetd mode

When launched by greetd, the app reads `GREETD_SOCK`, authenticates the selected user through greetd IPC, and starts the configured session command after successful authentication.

Default session command:

```sh
uwsm start nodalix.desktop
```

If `GREETD_SOCK` is missing and `--demo` was not passed, the app shows a friendly error screen instead of crashing.

## Debugging real login

Safe greeter logs are written to:

```sh
cat /tmp/nodalix-greeter.log
```

The log records app mode, selected user, session command, non-secret greetd message types, auth success/failure, session start request/acceptance/failure, socket close after success, and UI state transitions. It never logs passwords or PAM response contents.

Useful greetd journal command after a failed or suspicious boot:

```sh
journalctl -b -u greetd --no-pager | grep -Ei "nodalix|greeter|greetd|auth|pam|error|warn|hypr|uwsm" | tail -200
```

Common config locations:

- `/etc/greetd/config.toml`
- `/etc/greetd/hyprland-nodalix-greeter.conf`
- `/etc/nodalix/greeter/config.toml`

If login succeeds but a Hyprland command-line/loading screen is visible briefly, the likely cause is output from the real session compositor or launcher being shown on the VT after the greeter compositor exits. The greeter now keeps its loading overlay in the success transition and treats greetd socket close after `StartSession` as success, but polishing the VT handoff may also require adjusting the system greetd/session command. This repository only provides examples; it does not edit `/etc` automatically.

## Config

Runtime config path:

```sh
/etc/nodalix/greeter/config.toml
```

Defaults:

```toml
background = "/etc/nodalix/wallpapers/current/greeter-wallpaper.png"
session_command = "uwsm start nodalix.desktop"
accent_color = "#cba6f7"
font_family = "Inter"
default_user = ""
logo_path = "/usr/share/nodalix/brand/nodalix-logo-symbol.svg"
```

If the config file is missing, these defaults are used.

Logo lookup order:

1. `logo_path` from `/etc/nodalix/greeter/config.toml`
2. `/etc/nodalix/brand/nodalix-logo-symbol.svg`
3. `/usr/share/nodalix/brand/nodalix-logo-symbol.svg`
4. `/home/dani/Projects/nodalix-os/assets/brand/nodalix-logo-symbol.svg`
5. plain text fallback: `Nodalix OS`

## Users and avatars

Human users are read from `/etc/passwd`.

Included users:

- UID >= 1000
- shell does not contain `nologin` or `false`

Excluded users:

- `nobody`
- `greeter`
- system users

Avatar lookup order:

1. `/var/lib/AccountsService/icons/$USER`
2. `/home/$USER/.face`
3. `/home/$USER/.face.icon`
4. generated initials fallback

Unreadable avatar files are ignored gracefully.

## Sample greetd files

Sample files are provided only. They are not installed over active greetd configuration automatically:

- `config/greetd/hyprland-nodalix-greeter.conf`
- `config/greetd/config.nodalix-greeter.toml`
- `config/greetd/config.regreet-fallback.toml`
- `data/greetd/hyprland-nodalix-greeter.conf.example`
- `data/greetd/config.toml.example`

## Build

```sh
cargo build --release
```

## Install

```sh
sudo scripts/install-nodalix-greeter.sh
```

The installer builds the release binary, installs CSS/assets, creates `/etc/nodalix/greeter/config.toml` only if missing, and prints manual greetd switching steps. It does not overwrite active greetd config without making a timestamped backup.

It also installs the shared logo assets to:

```sh
/usr/share/nodalix/brand/nodalix-logo-symbol.svg
/usr/share/nodalix/brand/nodalix-logo.svg
```

Manual asset install, if needed:

```sh
sudo mkdir -p /usr/share/nodalix/brand
sudo cp /home/dani/Projects/nodalix-os/assets/brand/nodalix-logo-symbol.svg /usr/share/nodalix/brand/
sudo chmod 755 /usr/share/nodalix /usr/share/nodalix/brand
sudo chmod 644 /usr/share/nodalix/brand/nodalix-logo-symbol.svg
```

## Roll back to ReGreet

```sh
sudo scripts/rollback-regreet.sh
```
