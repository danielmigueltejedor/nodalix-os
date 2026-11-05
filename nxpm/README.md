# NXPM: binary-only Nodalix applications

This is the initial migration of **Nodalix Apps** into NXPM. It is deliberately
**fail closed**: a published, trusted package must be available before the
client installs or updates anything. **The PC never clones, patches or compiles
an application.** Existing locally built apps are preserved; no automatic
deletion, overwrite or false "migration completed" message.

## Application catalogue

| NXPM ID | Binary package required | Previous local source updater |
| --- | --- | --- |
| graphite | nodalix-graphite | graphite-update |
| hylki | nodalix-hylki | hylki-update |
| opencad | nodalix-opencad | opencad-update |
| patchy | nodalix-patchy | patchy-update |
| vinilo | nodalix-vinilo | vinilo-update |

These package names are **reserved proposals, not a claim that binaries
already exist**. Until they are built, smoke-tested, signed and published in
the official repository, installation and migration must fail without side effects.

## Client contract

```sh
nxpm list
nxpm status graphite
nxpm install graphite
nxpm update graphite
nxpm update --all
nxpm remove graphite
```

The initial CLI uses **pacman's ALPM transaction backend** with a restricted
curated app catalogue. It never executes arbitrary updater scripts. A future
native libalpm binding can replace the transport without changing the CLI
or Nodalix Store API. The repository must be managed by the OS, with
`SigLevel = Required` for both package and database signatures and an
appropriately trusted signing key. No `SigLevel = Never`, `--assume-installed`,
unsigned payloads, source builds, or AUR build hooks on user machines.
Use full ALPM system transactions, not partial `pacman -Sy` upgrades.

NOTE: a repository **not yet configured** is intentionally an error. Do not
point the client at a fabricated repository URL or an unsigned cache.

## Required server-side CI before shipping apps

1. For each app, pin a **source commit and overlay commit**, record license,
   upstream provenance, patch IDs and expected output executables.
2. Build each target architecture in an isolated CI runner, **never on clients**.
   Package all needed runtime assets (Graphite/CEF, OpenCAD dependencies,
   Vinilo/Aguja sidecar and services) without editing user home directories.
3. Run cold-start smoke tests in a clean Nodalix image, checking graphical
   launchers, MIME handlers, desktop integration, translations and dependencies.
4. Produce versioned `.pkg.tar.zst` packages, SBOM/provenance, SHA-256,
   signed package artifacts and a signed ALPM repository database.
5. Promote atomically to the Nodalix repository only after tests pass.
6. Verify release policy compatibility (kernel, GNOME, glibc and drivers).
   Retain old packages and a rollback path; never serve partial core upgrades.

## Legacy installation transition

Do **not** remove `~/.local/opt/nodalix/apps`, `~/.local/bin`, user desktop
entries or application data during the initial NXPM release. Existing launchers
may shadow new system packages. Implement an **explicit per-app migration** once
the binary package exists: install/validate the package, atomically move aside
only positively identified old managed launchers and symlinks, preserve all user
data, verify the new application launches, and offer rollback/undo. Unknown or
modified files are never deleted. Only after migration is verified should the
old source-updater scripts cease being shipped.

The Store/Applications GUI should retain installed-app discovery, local
AppImage import, Flatpak support and trash integration during this transition.
Moving GUI functionality requires preserving its current behavior and tests.

## Remaining work before 'complete'

This PR establishes NXPM and blocks client-side source builds from official
Nodalix Apps update commands; **it does not yet publish or migrate the five
applications**. The official repository, CI recipes, package signing, migration
and visual Store integration must land and pass system tests before claiming
the task complete.
