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
| Network (internet status, wired adapters, VPN, proxy, and the IPv4 address and name servers of a saved connection through `nmcli connection modify`), Wi-Fi, Bluetooth (BlueZ), Sharing, Sound, Power, Keyboard, Mouse and touchpad, Notifications, Multitasking, Privacy and security, Accessibility, Date and time, Backgrounds (still pictures and the animated gallery through the session service), Displays (screens read from mutter, Night Light), Printers (CUPS, `lpstat`), Online accounts (GNOME Online Accounts), Color (colord), Cloud storage (rclone, see below), Apps (default apps, installed apps), Users (your name, picture and password, and other accounts: add, change type, remove; through AccountsService, with polkit asking for an administrator's password; the password is hashed with `openssl passwd -6`, read from standard input, and only the hash goes to the service), Region and language (AccountsService + `org.gnome.system.locale`) | Native: GSettings keys through one set of row helpers (`pages/gs.rs`, which skips a key the system does not have), plus D-Bus for what is not a setting (Wi-Fi through NetworkManager, power mode and battery, time zone and NTP, device name) and `pactl` for sound. Keyboard (layouts, compose key and shortcuts), Printers (find on the network, add by address, remove) and Color (assign and import profiles) are native too. |
| Adding an online account | Opens `gnome-online-accounts-gtk`, the sign-in window of GNOME's online accounts library (the sign-in for Google or Microsoft is a browser flow that only that library can run). The row is shown only when the program is installed. No page calls `gnome-control-center`: it is not installed with Nodalix OS. |

Every page is native. Enterprise Wi-Fi (a login, not a password) is not supported yet.

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

## Opening a page

`nodalix-settings` takes the command line of `gnome-control-center`, so what launches that (the
Shell, notifications, the network indicator, desktop files) keeps working when this app takes
its place:

- `nodalix-settings wifi`, `nodalix-settings --page wifi`, `nodalix-settings --page=wifi` and
  `nodalix-settings -p wifi` open the Wi-Fi page. The names are the page ids and the names of
  the GNOME panels (`pages::resolve`: `background`, `universal-access`, `info-overview`,
  `default-apps`, `wwan`, `usb`, ...). A name nobody knows opens the first page, with no error;
  options and the extra arguments of a panel (`wifi connect-hidden-network`) are ignored.
- There is one window per session (application id `os.nodalix.Settings`). A second launch takes
  the open window to that page and brings it forward.
- The `open-page` action of the application, with the page name as a string, does the same from
  D-Bus: `gapplication action os.nodalix.Settings open-page "'wifi'"`. To start the app by
  D-Bus activation (`gio launch` with `DBusActivatable=true`, `gapplication` on a closed app)
  install `data/os.nodalix.Settings.service` in `/usr/share/dbus-1/services/`.

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

## Search in the Shell's overview

`nodalix-settings --search-provider` is the search provider the Shell asks (`org.gnome.Shell.SearchProvider2`)
on the name and path the GNOME Settings one had: `org.gnome.Settings.SearchProvider`,
`/org/gnome/Settings/SearchProvider`. It finds pages by name and by what is in them, in English
and Spanish (`ajustes-core/src/search.rs`; a term matches the start of a word, every term has to
match). It has no window and does not start GTK, so it needs no display; D-Bus starts it when the
Shell first asks, and it quits after 45 seconds without a call. Opening a result runs
`nodalix-settings --page <page>`.

- Result ids are `nodalix-<page id>`: `nodalix-storage` for "documentos grandes",
  `nodalix-updates`, `nodalix-appearance` (backgrounds), `nodalix-about` (version),
  `nodalix-localsend`, ...
- Files to install: `data/org.gnome.Settings.search-provider.ini` in
  `/usr/share/gnome-shell/search-providers/` and `data/org.gnome.Settings.SearchProvider.service`
  in `/usr/share/dbus-1/services/` (its `Exec` is `/usr/bin/nodalix-settings --search-provider`).
