# Local baseline: 2026-09-13

## Provenance

The user's installed system, rather than the historical remote repository, is
the authoritative starting point. Imported shell sources come from
`/etc/xdg/quickshell/nodalix/`. The installed packages report:

| Component | Version |
| --- | --- |
| Nodalix global version | 0.1.1 |
| nodalix-shell | 0.1.1-4 |
| nodalix-updater | 0.1.0-1 |
| quickshell | 0.3.1-1 |
| qt6-base | 6.11.2-3 |
| qt6-declarative / qt6-shadertools | 6.11.2-1 |
| ttf-fluent-emoji | 0.8.5-2 |

`installed-shell.json` records SHA-256 and size of every installed shell file.
Compiled Caelestia.Blobs output, its build-path resource map, package archives,
user settings, credentials, caches and user overrides are not source inputs.
The plugin's C++ and shader sources are included instead.

## Architecture and discrepancies

- `shell.qml` creates one MainWindow and desktop-widget tree per unique screen,
  a single Settings window, singleton services and named IPC handlers.
- `MainWindow.qml` combines the bar and several panels in a layer surface.
  `PopoutService` tracks one bar popup; launcher, notifications and Settings
  have independent lifecycle controllers. There is no common overlay stack.
- Settings has a Loader, but its activation depends on screen availability,
  not whether Settings is open. Its large content remains instantiated.
- ShellId and QuickShell's state/data/cache pragmas use Nodalix, but
  `Paths.qml` still points application state at `quickshell`. Hyprland reads
  generated bindings from `nodalix`. This inconsistency needs migration.
- Generated BindingService commands still use ambiguous `qs ipc` calls.
- The global service executes a user checkout launcher, with KillMode=process.
  A user override changes ExecStart to `/usr/bin/nodalix-shell`.
- The shell bootstrap recipe copies a local archive with SKIP checksums and
  embeds a locally compiled plugin. It does not package the systemd unit.
- The updater queries GitHub Releases and stages/checks SHA-256 before one
  pacman invocation, but manifest validation, signatures, transaction state,
  locking, error JSON, semantic prerelease ordering and privilege entry need
  production work. It writes global version metadata outside a package.
- Settings' existing UpdateService uses sudo password piping and AUR updates;
  it is not integrated with the imported Nodalix backend.
- Fluent Emoji is installed locally; redistribution and reproducible packaging
  have not yet been validated.

## Validation status

The baseline import was verified: 132 shell source/resource files matched the
installed SHA-256 hashes before modifications. The updater passed Python syntax
validation and `info --json` reported 0.1.1. Caelestia.Blobs built successfully.
Its staged installation exposed an incorrect qmltypes filename; the source
CMake file is corrected and its default install path is now package-relative,
rather than derived from the build user's HOME. This is the first source fix
relative to the recorded installed hashes.

Inspection is read-only against the running system. No shell restart, system
upgrade, release publication or ISO build has occurred. Access to the desktop
user bus is denied in the current sandbox, so no startup/RAM/CPU results are
claimed. This audit is not evidence of runtime acceptance.

Before v0.2.0: build real packages; validate signed manifests; test upgrade from
0.1.1 including self-update and failures; validate overlay focus/input/lifecycle
on two monitors; validate Qt/GTK/Electron/browser emoji; measure startup and
idle/peak memory and CPU; test two independent users and offline boot. Only
then publish stable packages and build/test the final ISO. The local Archiso
profile and independent app sources still need to be located/imported.
