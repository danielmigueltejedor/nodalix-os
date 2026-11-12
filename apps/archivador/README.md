# Archivador

The file manager of Nodalix OS, written in Rust on GTK 4 with the
[Nodalia Design Language](https://github.com/danielmigueltejedor/nodalia-design) through the
`nodalia-gtk` crate. It does not use libadwaita and it is not a theme over Nautilus: it is its own
app that follows Nautilus where Nautilus is right (trash layout, behaviour of cut and paste,
sorting) and goes its own way elsewhere.

## What it does today

| Area | State |
| --- | --- |
| Browsing | Folders as a list (with a column header that sorts) or a grid, image thumbnails, back, forward, parent, a path bar you can click or type in (Ctrl+L), places and drives in the sidebar with eject, live updates when the folder changes, a status line with the count or the selection. Sort by name, date, size or type, reverse order, hidden files, search in the folder (just start typing). |
| Editing | New folder, rename, copy, cut, paste (names never overwrite: `name (2)`), move to the trash, delete for good, properties (folder sizes are added up in the background), Open With…. Copy, move and delete run off the main thread, with a spinner in the header. |
| Clipboard and drag and drop | Copy and cut use the system clipboard (GNOME's `x-special/gnome-copied-files`, URI lists and file lists), so they paste to and from other file managers. Files drag out to other apps and in from them; onto a folder, a place in the sidebar, the trash, or the empty part of the folder. A drag inside Archivador moves, with Ctrl it copies; one from another app copies. |
| Trash | The freedesktop.org home trash, so what Archivador trashes shows in every other file manager. Restore, delete, empty. A file that cannot be trashed (another drive) offers to be deleted instead. |
| LocalSend | Right click, *Send with LocalSend…*: lists the nearby devices from the session service (`com.nodalix.LocalSend`) and sends through `/usr/lib/nodalix/localsend-send`, as the system menu always did. |
| Remembers | View, sort order, reverse and hidden files, in `~/.config/nodalix/archivador.json`. |

Not done yet: tabs, connecting to servers, mounting drives that are not mounted, progress with a
cancel button for long copies, bookmarks, the Nautilus extension API.

## Layout

| Crate | What it is |
| --- | --- |
| `crates/archivador-core` | Everything that is not drawing: reading and sorting folders, natural name order, unique names, copy / move / delete, the trash, the clipboard, the places, history, LocalSend devices and the send command, settings. No GTK; unit tested. |
| `crates/archivador` | The GTK application: window (`browser.rs`), the rows and tiles (`items.rs`), thumbnails, clipboard and drag and drop formats (`transfer.rs`), properties, dialogs, the LocalSend dialog. |
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
