# Nodalix Settings

The settings app of Nodalix OS, written in Rust on GTK 4 with the
[Nodalia Design Language](https://github.com/danielmigueltejedor/nodalia-design) through the
`nodalia-gtk` crate. It does not use libadwaita. It replaces the GNOME Control Center that
Nodalix has been shipping with its own pages.

## Pages

| Page | State |
| --- | --- |
| Appearance | Native: colour scheme, accent colour, text size, animations. |
| Updates | Native: a client of the session service `com.nodalix.Settings` (the same service the previous Settings used; it keeps running when the window closes and does the privileged work through its own helper). |
| LocalSend | Native: a client of the same session service (`GetLocalSend`, `SetLocalSend*`); name, receive folder, auto-accept and auto-disable options, nearby devices with favourites. |
| About | Native. |
| Network (internet status, wired adapters), Wi-Fi, Bluetooth (BlueZ), Sharing, Sound, Power, Keyboard, Mouse and touchpad, Notifications, Multitasking, Privacy and security, Accessibility, Date and time, Backgrounds (still pictures and the animated gallery through the session service), Displays (screens read from mutter, Night Light), Printers (CUPS, `lpstat`), Online accounts (GNOME Online Accounts), Color (colord), Cloud storage (rclone, see below), Apps (default apps, installed apps), Users (your name, picture and password, and other accounts: add, change type, remove; through AccountsService, with polkit asking for an administrator's password; the password is hashed with `openssl passwd -6`, read from standard input, and only the hash goes to the service), Region and language (AccountsService + `org.gnome.system.locale`) | Native: GSettings keys through one set of row helpers (`pages/gs.rs`, which skips a key the system does not have), plus D-Bus for what is not a setting (Wi-Fi through NetworkManager, power mode and battery, time zone and NTP, device name) and `pactl` for sound. Keyboard (layouts, compose key and shortcuts), Printers (find on the network, add by address, remove) and Color (assign and import profiles) are native too. |
| What still opens the previous GNOME panel (from inside a native page) | Adding an online account (the sign-in for Google or Microsoft is a browser flow that only GNOME's own library can run). |

Still to port from the previous Settings, in the order they are used most: animated backgrounds
(inside Appearance), LocalSend, storage, then the GNOME panels.

## Layout

| Crate | What it is |
| --- | --- |
| `crates/ajustes-core` | Pages, system information and the update service's state. No GTK; unit tested. |
| `crates/ajustes` | The GTK application. |
| `../crates/nodalix-ui` | NDL set up for Nodalix OS (system font, text scaling), shared with the other native apps. |

Adding a page: add it to `pages::SECTIONS` (the test fails until its title has a Spanish
translation), build its widget in `crates/ajustes/src/pages/`, add it to the stack in `window.rs`.

`NODALIX_SETTINGS_PAGE=updates` (any page id) opens the app on that page, for screenshots and
tests.

See `../README.md` for how to build and test the native apps.

## Cloud storage

The Cloud storage page sets up Google Drive, OneDrive, Dropbox and iCloud Drive with `rclone` and
shares them with the file manager through one file and one service per account:

- `~/.config/nodalix/cloud.json` lists the accounts (`id`, `provider` = `drive` | `onedrive` |
  `dropbox` | `iclouddrive`, `name`). Ids are letters, digits, `_` and `-`.
- The `rclone` remote of an account is `nodalix-<id>`.
- `~/.config/systemd/user/nodalix-cloud@.service` (written by the page) mounts an account with
  `rclone mount` on `~/Nube/<name>` (the name with `/` replaced by `-`; the provider's title when
  the name is empty). The folder comes from `~/.config/nodalix/cloud/<id>.env`. The unit is
  `Type=notify`, so starting it returns once the mount is up and fails if it is not.
- Settings adds (browser sign-in through `rclone authorize`; iCloud with the Apple ID, password and
  the six-digit code), removes, connects and disconnects. The file manager only reads the list
  and starts or stops the unit.
