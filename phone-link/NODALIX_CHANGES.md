# Enlace móvil de Nodalix

This component is derived from `gabrielmeir53/iphonebridge` v0.4.2 and keeps
its GPL-2.0-or-later license. Nodalix packages it as **Enlace móvil** and adds
desktop integration, Spanish-facing UI strings, reconnection handling,
notification actions, call-only Bluetooth audio routing, and systemd units.

## Nodalix 0.2.5 integration

- ANCS notifications use direct BlueZ `AcquireNotify` transports when
  available, reassemble fragmented Data Source packets and recover after LE
  reconnects without restarting the daemon.
- Automatic reconnect keeps BR/EDR and LE recovery independent and bounds the
  absent-phone retry backoff.
- HFP calls use oFono as the single Hands-Free owner; the experimental BlueZ
  HFP client is disabled so oFono can own call control and PipeWire SCO audio.
- HFP recovery is kicked immediately when the iPhone becomes reachable again
  instead of waiting for a stale long retry timer.
- Live calls are resident, critical notifications pinned above all other
  notifications with Answer/Decline/Hang up actions always reachable.
- PBAP contact refreshes are mirrored into a dedicated
  **iPhone (Nodalix)** Evolution Data Server address book so GNOME Contacts
  sees the iPhone phonebook without touching the user's other address books.

No paired-device address, account, contact, notification, or other personal
configuration is included in this repository or in the release package.
