# Archivador

The file manager of Nodalix OS, written in Rust on GTK 4 with the
[Nodalia Design Language](https://github.com/danielmigueltejedor/nodalia-design) through the
`nodalia-gtk` crate. It does not use libadwaita and it is not a theme over Nautilus: it is its own
app that follows Nautilus where Nautilus is right (trash layout, behaviour of cut and paste,
sorting) and goes its own way elsewhere.

## What it does today

| Area | State |
| --- | --- |
| Browsing | Folders as a list (with a column header that sorts), a grid, or columns (Ctrl+1, 2, 3; one column per folder on the way, and a preview of what is selected: a folder's content, or a card with the file's details), full-colour file and folder icons from the system's icon theme (the user's own folders show their picture), image thumbnails, back, forward, parent, a path bar you can click or type in (Ctrl+L), places and drives in the sidebar with eject, live updates when the folder changes, a status line with the count or the selection. Sort by name, date, size or type, reverse order, hidden files, search in the folder (just start typing). |
| Editing | New folder, rename, copy, cut, paste (names never overwrite: `name (2)`), move to the trash, delete for good, properties (folder sizes are added up in the background), Open With…. Copy, move and delete run off the main thread, with a spinner in the header. |
| Clipboard and drag and drop | Copy and cut use the system clipboard (GNOME's `x-special/gnome-copied-files`, URI lists and file lists), so they paste to and from other file managers. Files drag out to other apps and in from them; onto a folder, a place in the sidebar, the trash, or the empty part of the folder. A drag inside Archivador moves, with Ctrl it copies; one from another app copies. |
| Trash | The freedesktop.org home trash, so what Archivador trashes shows in every other file manager. Restore, delete, empty. A file that cannot be trashed (another drive) offers to be deleted instead. |
| Applications | *Applications* is a place in the sidebar (the real one of `Aplicaciones`/`Applications`, as Nodalix's layout service records it). A folder's `.hidden` file is honoured, so the alias Nodalix keeps for the other name does not show up twice, and the icon a file or folder carries in `metadata::custom-icon` is drawn, which is how each app and the folder itself get theirs. |
| Cloud | The accounts Settings has set up (Google Drive, OneDrive, Dropbox, iCloud Drive, in `~/.config/nodalix/cloud.json`) are listed under *Cloud* in the sidebar. They are rclone remotes (`nodalix-<id>`) mounted by the user service `nodalix-cloud@<id>.service` at `~/Nube/<name>`; clicking one that is not mounted starts the service, and its eject button stops it. Archivador never writes the configuration. |
| Recent, Favorites | *Recent* lists the files the desktop remembers opening (`recently-used.xbel`), newest visit first. *Favorites* are the folders pinned with "Add to Favorites" (right click on a folder) or in the file GTK has always used, `~/.config/gtk-3.0/bookmarks`, so they are the same in every file manager. |
| Network | *Connect to a server…* mounts `smb://`, `sftp://` (`ssh://`), `ftp://`, `dav://`, `nfs://` or `afp://` addresses through GVfs, which asks for a password if it needs one; what was used is remembered in `~/.config/nodalix/archivador-servers` and listed under *Network*, along with the network places that are mounted. A server is shown only when GVfs's FUSE mount gives it a path. |
| Icons | Archivador draws its own icons, so they look the same on any icon theme: flat two-tone folders in Nodalia's teal with a glyph on the front for the user's own (Documents, Downloads, Music…), and a plain white page with a folded corner, a glyph and the extension under it for files, in one colour per kind (PDF, text, documents, sheets, slides, images, audio, video, archives, code, disk images, packages). Right click a folder, *Customize folder…*: one of nine colours, or *Choose image…*; *Reset* undoes both. The colour is kept in the folder's `metadata::nodalix-folder-color` and the picture in `metadata::custom-icon`, which other GNOME file managers read too. |
| Dragging | A drag shows the file's icon (with a count for several). Resting it on a folder, in the list or in the sidebar, opens that folder after 0.7 s; the one it is over is outlined. Pressing on a file draws no selection box. |
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
