# Nodalix Apps Local Overlay System

Nodalix Apps follows upstream projects while allowing Nodalix-specific
modifications to be maintained without carrying separate patch files.

## Source of truth

Local modifications are normal Git commits in a fork.

The preferred branch name is:

    nodalix

Exported `.patch` files are not the canonical source of Nodalix changes.

## Layout

Source/build cache:

    ~/.cache/nodalix-apps/src/<app>

Installed builds:

    ~/.local/opt/nodalix/apps/<app>/versions

Active build:

    ~/.local/opt/nodalix/apps/<app>/current

Launchers:

    ~/.local/bin

The Git checkout is a source cache, not a second application installation.

## Updating

`nodalix-apps update <app>`:

1. Fetches upstream.
2. Fetches the configured Nodalix fork.
3. Resolves the latest allowed upstream version.
4. Detects overlay commits whose effective changes are not upstream.
5. Checks out a clean upstream tree.
6. Replays those overlay commits.
7. Regenerates application-specific generated resources.
8. Builds with native CPU optimisation where appropriate.
9. Validates the resulting build.
10. Stores it as a separate version.
11. Atomically switches `current`.
12. Updates the launcher.
13. Reapplies Nodalix icons and integration.

## Editing an application

Make changes in its fork:

    git switch nodalix

Edit the source, then:

    git add ...
    git commit -m "area: description"
    git push origin nodalix

Nothing needs to be exported manually.

The next:

    nodalix-apps update <app>

will detect the changed overlay automatically.

## Detecting merged upstream changes

Overlay comparison uses Git patch equivalence.

A commit whose effective change is already present upstream is excluded
automatically even if its SHA differs.

Therefore a Nodalix change accepted upstream stops being replayed without
requiring an immediate rewrite of the fork history.

## Conflicts

If an overlay commit no longer applies cleanly:

- abort the update;
- abort the cherry-pick;
- delete temporary build output;
- leave the active installation untouched;
- report the conflicting commit.

Never silently discard a Nodalix modification that has not been incorporated
upstream.

## Build identity

A managed build is identified by:

    upstream version/commit
    + CPU target
    + effective overlay hash

The overlay hash is based on stable Git patch IDs rather than commit SHAs.

Rebasing identical changes therefore does not force a rebuild, while changing
the actual local code does.

## Generated files

Generated files should normally be regenerated during the build instead of
being maintained as overlay commits when possible.

Examples:

- gettext POT files
- synchronized PO catalogues
- generated caches
- generated manifests

Patch the source of the generated information and regenerate the derived data
after applying the overlay.

## Icons

Nodalix application icons do not belong in application forks.

Canonical custom icons are stored in:

    /usr/share/nodalix-apps/assets/icons/colloid

Every successful update runs the Nodalix icon integration automatically.

Updating an application must never require manually restoring its icon.

## Installation policy

Once a Nodalix-managed native build has been verified, it should be the active
installation of that application.

A second Flatpak, AUR package, AppImage, or native installation should only be
kept intentionally for testing.

Source repositories and build caches do not count as additional installations.

## Preferred fork model

For every application that needs local modifications:

    upstream repository
        +
    user fork
        branch: nodalix

Apps without local modifications need no fork. If local modifications are
introduced later, add a fork and `nodalix` overlay branch to the manifest.

## Rollback

Builds are immutable once installed.

A failed update never overwrites the active build.

`current` changes only after a complete successful build and validation.

## In-app update integration

Applications managed by Nodalix Apps must never replace themselves using their
upstream installer, Flatpak, AppImage, package manager, or download flow.

An application may keep its native upstream update detection and version
notification.

When the user chooses to install an available update, the application must call:

    nodalix-app-update <app-id>

Examples:

    nodalix-app-update opencad
    nodalix-app-update patchy

`nodalix-app-update` is the stable GUI-facing update bridge. It delegates to
`nodalix-apps update`, so the normal upstream, Git overlay, CPU-native build,
validation, versioned installation, atomic activation and Nodalix integration
workflow is always preserved.

The running application is not replaced in place. The newly activated build is
used on the next launch.

Upstream self-installers must never be invoked from a Nodalix-managed build.

