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

```sh
cd apps/monitor
cargo test --workspace          # needs GTK 4.20 or newer for the app crate
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p nodalix-monitor
```

`nodalia-gtk` is a git dependency pinned to an exact commit of nodalia-design.
When NDL changes, bump the `rev` in `crates/monitor/Cargo.toml`.

## Fonts and colours

Nodalix uses the system font (Adwaita Sans) and ships no other, so `style.css` points NDL's font
variables at it and converts NDL's px sizes to pt, which GTK scales with the system text size.
Every colour in the app comes from an NDL token; the graphs read their colour from the widget's
CSS `color`.

`NODALIX_MONITOR_PAGE=resources|disks` opens the app on that page, for screenshots and tests.
