# NXPM — Nodalix Package Manager

NXPM replaces **Nodalix Apps**, including the old hard-coded catalogue of
Graphite, Hylki, OpenCAD, Patchy and Vinilo. No applications are automatically
installed by NXPM. There is **no application-specific Git checkout, overlay,
local compilation script or always-moving upstream build** on user machines.

## Interfaces

- Native binary repositories: ALPM/pacman (a Nodalix-owned libalpm binding
  may replace this adapter later).
- Flatpak: install, uninstall, update and explicit user permission overrides.
- AppImage: import portable binaries through the Applications installer
  without invoking their binaries to inspect metadata.
- AUR/source-only packages: **not compiled on user machines**. To distribute
  an AUR-only application, a Nodalix build service must build and validate a
  redistributable, signed native package first; until then it is unavailable.
- Third-party .deb/.rpm: NOT directly executable/installable on an Arch system.
  Use Flatpak/AppImage or a suitable binary repackage; do not claim universal
  .deb/.rpm compatibility.

Examples:

    nxpm search blender
    nxpm search blender --source flatpak
    nxpm install blender
    nxpm install flatpak:org.blender.Blender
    nxpm install ~/Descargas/example.AppImage
    nxpm list
    nxpm update --all
    nxpm remove blender
    nxpm permissions show org.blender.Blender
    nxpm permissions grant org.blender.Blender xdg-download
    nxpm permissions revoke org.blender.Blender home
    nxpm permissions reset org.blender.Blender

A native installation synchronises and upgrades the **whole compatible ALPM
transaction**, never a partial pacman -Sy. Kernel updates are controlled by
Nodalix releases. **Before shipping production**, provide a signed, internally
consistent Nodalix repository/snapshot and enforce core component pinning;
the current adapter alone cannot guarantee the stability of frozen GNOME
components on a rolling Arch base.

## Aplicaciones (.app)

The existing Nodalix Applications mechanism moves under NXPM. It creates
small, token-registered **.app launch descriptors**, not macOS executable
bundles. Linux packages retain their real filesystem layout and package
ownership. Double-click activates the registered desktop entry.

On a user's desktop, the folder is ~/Aplicaciones (or ~/Applications under an
English locale). A future shared /Aplicaciones view must be a privileged or
virtual mount; it must **not** become world-writable. Uninstall by moving
a registered .app into Trash; a validated Trash intent triggers the appropriate
ALPM/Flatpak/AppImage uninstall, preserves user data, and restores the .app if
the uninstall fails or requires authentication. Renaming, copying, replacing,
or deleting unrelated files cannot initiate privileged package removal.

File deletion outside Trash does **not** uninstall software. That is a deliberate
fail-safe until an authenticated delete-intent portal supports it. Protected
system packages cannot be removed by deleting a launcher.

## Migration safety

The nxpm package **replaces** nodalix-apps in the package database. The legacy
app-specific manifests and build scripts are not shipped. Existing local
installations under ~/.local/opt/nodalix/apps and their launchers/data are
**not automatically deleted**; no migration may remove a user's data or an
unrecognized file. Nodalix's graphical library preserves its previous
registry and desktop IDs, so old .app descriptors can be recognized. Legacy
manual builds require an explicit guided replacement with trusted binaries
before removal; the user may continue using them meanwhile.

## Work not yet complete

This branch establishes the shared CLI, adapters, packaged Applications
integration, and regression foundation. Still needed for a polished 1.0:
graphical cross-source search and installation, repository provenance and
transaction-policy UI, AppImage update feeds and signature policies,
a remote AUR build farm, Flatpak permission management inside Settings,
transaction history, recovery, and full VM/real-hardware validation.
No claims of universal compatibility or finished release without these.
