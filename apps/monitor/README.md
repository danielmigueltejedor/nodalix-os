# Nodalix System Monitor

The system monitor of Nodalix OS, written in Rust on GTK 4 with the
[Nodalia Design Language](https://github.com/danielmigueltejedor/nodalia-design) through the
`nodalia-gtk` crate. It does not use libadwaita.

It replaces GNOME System Monitor and does the same three jobs:

- **Processes**: every process with user, CPU, memory, threads and state; search, sort by any
  column, end, force quit, stop and continue the selected one.
- **Resources**: CPU (total and per core), memory, swap and network, as graphs of the last minute.
- **File systems**: mounted disks with used and free space.

## Layout

| Crate | What it is |
| --- | --- |
| `crates/monitor-core` | Reads `/proc` and the mount table. No GTK; every parser takes text and is unit tested. |
| `crates/monitor` | The GTK application. |

## Build and test

See `../README.md` for how to build and test the native apps.

Every colour in the app comes from an NDL token; the graphs read their colour from the widget's
CSS `color`.

`NODALIX_MONITOR_PAGE=resources|disks` opens the app on that page, for screenshots and tests.
