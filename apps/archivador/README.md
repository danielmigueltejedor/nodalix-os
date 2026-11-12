# Archivador

The file manager of Nodalix OS, written in Rust on GTK 4 with the
[Nodalia Design Language](https://github.com/danielmigueltejedor/nodalia-design) through the
`nodalia-gtk` crate. It does not use libadwaita and it is not a theme over Nautilus: it is its own
app that follows Nautilus where Nautilus is right (trash layout, behaviour of cut and paste,
sorting) and goes its own way elsewhere.

## What it does today

| Area | State |
| --- | --- |
| Browsing | Folders as a list or a grid, back, forward, parent, path bar, places and drives in the sidebar, live updates when the folder changes. Sort by name, date, size or type, reverse order, hidden files, search in the folder (just start typing). |
| Editing | New folder, rename, copy, cut, paste (names never overwrite: `name (2)`), move to the trash, delete for good. Copy and delete run off the main thread. |
| Trash | The freedesktop.org home trash, so what Archivador trashes shows in every other file manager. Restore, delete, empty. A file that cannot be trashed (another drive) offers to be deleted instead. |
| LocalSend | Right click, *Send with LocalSend…*: lists the nearby devices from the session service (`com.nodalix.LocalSend`) and sends through `/usr/lib/nodalix/localsend-send`, as the system menu always did. |
| Remembers | View, sort order, reverse and hidden files, in `~/.config/nodalix/archivador.json`. |

Not done yet: drag and drop, thumbnails, properties, open with…, tabs, connecting to servers,
mounting and ejecting drives, a system clipboard that other apps can paste from (copy and cut work
between Archivador windows only), progress for long copies, the Nautilus extension API.

## Layout

| Crate | What it is |
| --- | --- |
| `crates/archivador-core` | Everything that is not drawing: reading and sorting folders, natural name order, unique names, copy / move / delete, the trash, the clipboard, the places, history, LocalSend devices and the send command, settings. No GTK; unit tested. |
| `crates/archivador` | The GTK application: window (`browser.rs`), the rows and tiles (`items.rs`), dialogs, the LocalSend dialog. |
| `../crates/nodalix-ui` | NDL set up for Nodalix OS (system font, text scaling), shared with the other native apps. |

```sh
cd apps
cargo test -p archivador-core          # no GTK needed
cargo run -p nodalix-archivador        # opens the home folder
cargo run -p nodalix-archivador -- ~/Documents
```

Strings are English in the source with a Spanish table in `i18n.rs`.

## Not packaged yet

Like the other native apps, Archivador is built and tested by CI but does not ship in the OS, and it
is not the default file manager: `MimeType=inode/directory` in the desktop file only lets it be
chosen. The earlier approach, a patched Nautilus (`gnome/archivador/`, package `nodalix-archivador`),
is still in the repository and is what the OS would install today; see `docs/archivador.md`.
