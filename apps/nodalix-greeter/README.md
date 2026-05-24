# nodalix-greeter

Custom GTK4/libadwaita greetd greeter for Nodalix OS.

It is designed to replace ReGreet visually while keeping ReGreet available as a manual fallback. The UI follows the Nodalix Hyprlock look: wallpaper background, dark overlay, large centered clock, Spanish date, Nodalix brand, glass password pill, bottom-left user selector, and a loading overlay while greetd starts the session.

## Demo mode

```sh
cargo run -- --demo
```

Demo mode does not connect to greetd. It uses fake users and accepts `nodalix` or `password` as a fake password so the loading state can be previewed.

## greetd mode

When launched by greetd, the app reads `GREETD_SOCK`, authenticates the selected user through greetd IPC, and starts the configured session command after successful authentication.

Default session command:

```sh
uwsm start hyprland-uwsm.desktop
```

If `GREETD_SOCK` is missing and `--demo` was not passed, the app shows a friendly error screen instead of crashing.

## Config

Runtime config path:

```sh
/etc/nodalix/greeter/config.toml
```

Defaults:

```toml
background = "/etc/nodalix/wallpapers/current/greeter-wallpaper.png"
session_command = "uwsm start hyprland-uwsm.desktop"
accent_color = "#cba6f7"
font_family = "JetBrainsMono Nerd Font"
default_user = ""
```

If the config file is missing, these defaults are used.

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

## Build

```sh
cargo build --release
```

## Install

```sh
sudo scripts/install-nodalix-greeter.sh
```

The installer builds the release binary, installs CSS/assets, creates `/etc/nodalix/greeter/config.toml` only if missing, and prints manual greetd switching steps. It does not overwrite active greetd config without making a timestamped backup.

## Roll back to ReGreet

```sh
sudo scripts/rollback-regreet.sh
```

