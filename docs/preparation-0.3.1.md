# Local desktop integration for 0.3.1

This development branch starts at the current GNOME `main` and imports portable
source from the working desktop. `VERSION` describes the next build; no release,
tag, ISO publication or change to the running desktop is performed by this work.

## Included

| Local improvement | Packaged destination |
| --- | --- |
| Dash to Dock 110 and the installed overview/startup behaviour | `nodalix-gnome`, with refreshed source hashes |
| Startup cursor completion after a normal mapped window is visible for 750 ms | `nodalix-gnome` extension profile |
| Offline Steam launcher/icon sync and native window matching | `nodalix-integrations`, `steam-desktop-sync.timer` |
| GameMode detection for native and Proton games | `nodalix-integrations`, `steam-auto-gamemode.service` |
| Persistent Craft title bars and original launch command handling | `nodalix-integrations`, `nodalix-craft-gnome.service` |

The graphical services stop with the GNOME session. GameMode is optional and its
service skips startup when the library is absent. Steam sync supports native and
Flatpak Steam, uses local manifests/cache and preserves custom launcher settings.
It does not download games, sign into accounts or install Steam.

The 0.3.1 user migration is independent of the completed 0.3.0 migration. It
backs up recognised user service prototypes before allowing packaged services
to take over. Custom units remain untouched. Craft launch records and backups
stay in the user's state directory. Explicit extension disable choices remain
in effect. The existing default profile continues to avoid competing secondary
panels and competing tiling extensions.

## Kept local

OpenLogi already supplies its own agent service; it is documented as an optional
integration rather than copied as a second service. The machine's Spanish-only
Steam rule, personal iCloud backup uploader, VPN, virtualisation and security
enrolment are not default desktop services. iCloud, calendar, Phone Link and
application organisation retain their existing portable packages.

Bluetooth USB power tweaks, interface-specific dual-network settings, controller
pairings, firewall/USB allow lists and Secure Boot/signing keys require the target
machine's own hardware and policy. They are not copied into a generic GNOME OS.
No home directory, account tokens, certificates, backup contents or runtime caches
are part of the imported source.

## Validation before publication

Run the unit/integration suite, package layout checks and private GNOME 51 shell
test. CI additionally builds all packages and boots disposable clean-install,
historical-upgrade and ISO guests. Those full guest checks must pass on this
candidate before a later publication. Updating this branch does not publish it.

Local checks on 2026-10-19: 219 unit/integration tests pass; both changed desktop
packages build as `0.3.1-1` and their payloads pass inspection. The private GNOME
51 shell test enables the extensions and passes three enable/disable cycles,
two-monitor work-area checks, reconfiguration and fullscreen hide/restore. Its
scale-2 path was unavailable on this machine. The standalone native Settings
restart harness could not run locally because `xvfb-run` is absent; CI installs
that dependency. Full candidate package builds, upgrade guests and ISO boot
remain CI validation work, not a locally verified publication claim.

## Audit follow-up, 2026-10-20

The installed desktop exposed a legacy global application-update unit pointing
at a removed QuickShell helper. The system migration now archives that override
even when 0.3.0 migration was already completed. The local user service uses the
packaged updater, stale iCloud unit links were replaced and the competing Tiling
Assistant was disabled while Nodalix Auto Tiling remains active.

Tiling Assistant's saved-session reader took the etag instead of the byte buffer
from promisified GIO, causing an error after session lock. The buffer selection
and corrupt-JSON handling are corrected and tested with real GJS/GIO.

Four historical Phone Link daemon aborts coincided with reconnect workers using
shared D-Bus/GLib state. Reconnect now uses D-Bus asynchronous callbacks on the
main loop, and D-Bus threading is initialised before creating connections, as
required by the [dbus-python documentation](https://dbus.freedesktop.org/doc/dbus-python/dbus.mainloop.html).
This removes the observed cross-thread path; a sustained live check is still
needed before claiming the native heap aborts cannot recur. The installed UI
launcher, plain-text rows and Spanish labels are included in the package source.

CI's first package build failed when GNOME GitLab returned HTTP 503 while cloning
the pinned volume-control subproject. Source fetching now retries three times in
temporary directories and verifies the commit before publishing a checkout.
Integration archives also exclude Python runtime caches.

The local AIDE alert reports 456 added, 41 removed and 1350 changed entries since
its old baseline. It is retained for review rather than automatically accepted
as a new trusted baseline. The automatic system update failed immediately after
resume when DNS was unavailable; DNS currently resolves again. Neither event is
silenced by disabling its service. Package checks found no readable installed
payload missing; protected system paths require administrator validation.
