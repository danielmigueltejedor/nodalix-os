# Native Nodalix apps

Apps written for Nodalix OS in Rust on GTK 4 with the
[Nodalia Design Language](https://github.com/danielmigueltejedor/nodalia-design) (`nodalia-gtk`,
no libadwaita), so that every core app looks and behaves the same.

| App | Directory | Replaces |
| --- | --- | --- |
| System Monitor | `monitor/` | GNOME System Monitor |
| Settings | `ajustes/` | GNOME Control Center |
| Files (Archivador) | `archivador/` | Nautilus |

`crates/nodalix-ui` is what they share. One Cargo workspace (this directory) holds them all, so
there is one lock file and one NDL revision: `nodalia-gtk` is pinned to an exact commit in
`Cargo.toml`, and bumping it moves every app together.

```sh
cd apps
cargo test --workspace            # needs GTK 4.20 or newer
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p nodalix-monitor
cargo run -p nodalix-settings
cargo run -p nodalix-archivador
```

Nodalix uses the system font (Adwaita Sans) and ships no other. `crates/nodalix-ui/src/base.css`
points NDL's font variables at it and converts NDL's px sizes to pt, which GTK scales with the
system's text size. Every colour comes from an NDL token; apps never name one.

Strings are English in the source with a Spanish table in each app's `i18n.rs`; a test fails for a
string without a translation.

Not packaged yet: the apps are built and tested by `.github/workflows/apps.yml`, but do not yet
ship in the OS or replace the GNOME ones.
