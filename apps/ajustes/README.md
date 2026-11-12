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
| Network (internet status, wired adapters), Wi-Fi, Bluetooth (BlueZ), Sharing, Sound, Power, Keyboard, Mouse and touchpad, Notifications, Multitasking, Privacy and security, Accessibility, Date and time, Apps (default apps, installed apps), Users (name and type), Region and language (AccountsService + `org.gnome.system.locale`) | Native: GSettings keys through one set of row helpers (`pages/gs.rs`, which skips a key the system does not have), plus D-Bus for what is not a setting (Wi-Fi through NetworkManager, power mode and battery, time zone and NTP, device name) and `pactl` for sound. Keyboard layouts and shortcuts still open the previous panel. |
| Everything else (displays, printers, color, online accounts…) | Listed in the sidebar; each opens the previous GNOME panel of the same name in its own window until it is ported. |

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
