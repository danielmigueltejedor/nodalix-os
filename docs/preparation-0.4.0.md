# 0.4.0 preparation and complete open-issue audit

Audit date: 2026-10-31; desktop revision validated 2026-11-02. Base: `b0e557b` on `main` (0.3.1 preparation).
GitHub had **26 open issues** at the start of this work. They include defects,
feature requests and the architecture epic for a later major release. This
candidate does **not** complete the entire backlog or publish a stable release.
Issues remain open until their acceptance criteria and release validation pass.

## Issue-by-issue result

| Issue | Changes or existing coverage | Remaining acceptance work |
| --- | --- | --- |
| [#15](https://github.com/danielmigueltejedor/nodalix-os/issues/15) iPhone Focus | Protocol feasibility reviewed. No fabricated Focus state is exposed. | An authorized iPhone-side source is needed for a reliable global Focus state and named modes; companion/protocol design and real-device validation remain. |
| [#19](https://github.com/danielmigueltejedor/nodalix-os/issues/19) Live-call priority | Native notification objects are marked from the existing Phone Link hint. Calls stay first, hide dismissal, survive Clear all and keep their real actions. | Final real-phone regression and merge/release validation. Private GNOME tests exercise the complete native notification path. |
| [#20](https://github.com/danielmigueltejedor/nodalix-os/issues/20) Architecture epic | Notification compatibility and update-state logic have separate modules. Existing backend/packaging separation is retained. | The full architecture inventory, versioned service contracts and subsystem migration are a separate major body of work; this epic is not complete. |
| [#22](https://github.com/danielmigueltejedor/nodalix-os/issues/22) Auto Tiling | Four-window default; largest-region insertion produces halves/quarters. Real 1/2/3/4-window, minimize/restore, two-monitor and disable tests added. | Physical/fractional-scale and additional floating-dialog/size-constraint acceptance coverage. |
| [#23](https://github.com/danielmigueltejedor/nodalix-os/issues/23) Performance Director | Diagnostics and reproducible sampling provide evidence for its design. | The shared event-driven controller, reversible state transactions and runtime integrations remain unimplemented. |
| [#24](https://github.com/danielmigueltejedor/nodalix-os/issues/24) Gaming Mode | Existing Steam GameMode requests are retained. | A unified Quick Settings switch and reversible coordination through the Director remain unimplemented. |
| [#25](https://github.com/danielmigueltejedor/nodalix-os/issues/25) Secondary bars | Existing native bars pass repeated lifecycle, two-monitor work-area, hotplug configuration and fullscreen hide/restore tests. Primary Quick Settings count remains stable. | Physical monitor/fractional-scale validation. The headless monitor-removal path still triggers an upstream workspace clipping warning, also present in the unmodified baseline. |
| [#26](https://github.com/danielmigueltejedor/nodalix-os/issues/26) Adaptive Hanabi | Native DMA-BUF negotiation, unused-filter removal, rectangular shader fast path, graphics offload when supported, cached blur, and event-driven coverage pause/reveal; original files unchanged. Hardware 4K30 and software Theora animate on both monitors, retain Overview rounding and pause behind tiled windows. | Unified Gaming/Director integration, physical laptop watts, Intel/NVIDIA and older-GPU validation remain. |
| [#27](https://github.com/danielmigueltejedor/nodalix-os/issues/27) Overview actions | Existing native GNOME application search remains. | A localized Nodalix SearchProvider2 and its action/permission tests remain unimplemented. |
| [#28](https://github.com/danielmigueltejedor/nodalix-os/issues/28) Diagnostics | New native System diagnostics page, explicit copy action and local JSON collector. Versions, memory, CPU/GPU driver, displays, refresh modes, optional VRR properties, power profile, swap, failed-unit counts, portal and update/recovery state. | VA-API/quantum/xrun details, Phone Link/GLocalSend runtime status, concise summary and targeted health actions are not yet complete. |
| [#29](https://github.com/danielmigueltejedor/nodalix-os/issues/29) Power profiles | Native GNOME/PPD integration is present. This desktop reports balanced, power-saver and performance through its AMD driver. Diagnostics use that real backend. | Temporary Director/Gaming requests and laptop behavior remain to be implemented/validated. |
| [#30](https://github.com/danielmigueltejedor/nodalix-os/issues/30) ZRAM | Vendor default: half RAM capped at 8 GiB, LZ4, swap priority 100. Actual upstream generator tested with 2/16/32 GiB fixtures and an administrator override. Existing disk swap/resume configuration is retained. | Low-RAM VM pressure/OOM, sustained load and reboot tests; LZ4 availability on every supported kernel. No universal performance improvement is claimed. |
| [#31](https://github.com/danielmigueltejedor/nodalix-os/issues/31) Safe Updates | Opt-in `pre-update` snapshot through existing recovery engine, before dependency installation/migrations or native system updates; creation verification and retention of three owned pre-update snapshots. Unsupported filesystems report a skip. Snapshot failures stop a protected update. | Real supported-Btrfs update/rollback boot test and a full recovery UI remain. Protection must first be enabled with the existing recovery schedule setting. |
| [#32](https://github.com/danielmigueltejedor/nodalix-os/issues/32) Startup/lazy activation | Layout maintenance now watches relevant directories/configuration; reconciliation fallback changes from every minute to every 30 minutes. iCloud mount uses an ExecCondition and never prompts for an unconfigured/encrypted remote. | Controlled boot/login budget and critical-chain comparisons, and further optional-service activation work remain. |
| [#33](https://github.com/danielmigueltejedor/nodalix-os/issues/33) Low-latency audio | Stable PipeWire defaults retained. | Reversible profile, per-device fallback/xrun validation and Director integration remain unimplemented. |
| [#34](https://github.com/danielmigueltejedor/nodalix-os/issues/34) High refresh / VRR | Diagnostics read Mutter's current/available modes and scales without changing them. Absent VRR data stays unknown. | Physical high-refresh/VRR, fullscreen, mixed-refresh and hotplug matrix remains. |
| [#35](https://github.com/danielmigueltejedor/nodalix-os/issues/35) Welcome | Existing one-time migrations preserve upgraded-user choices. | The skippable first-run application, translations and shortcut introduction remain unimplemented. |
| [#36](https://github.com/danielmigueltejedor/nodalix-os/issues/36) Wallpaper accent | Native appearance and wallpaper integration retained. | Palette selection, explicit opt-in, caching and native accent update feature remain unimplemented. |
| [#37](https://github.com/danielmigueltejedor/nodalix-os/issues/37) Shell integration layer | Versioned GNOME 51 notification compatibility module with reversible injections and tested lifecycle. | Remaining Nodalix Shell integrations and versioned backend contracts are not consolidated yet. |
| [#38](https://github.com/danielmigueltejedor/nodalix-os/issues/38) Clipboard history | No clipboard content is read or persisted by this candidate. | History, privacy policy/enforcement, retention, UI and paste workflow remain unimplemented. |
| [#39](https://github.com/danielmigueltejedor/nodalix-os/issues/39) Benchmarks | Private real-GNOME behavioral suite expanded; opt-in resource sampler and an actual desktop sample added. | Controlled idle/upstream comparison, frame timing, load/pressure workloads and per-hardware regression thresholds remain. |
| [#41](https://github.com/danielmigueltejedor/nodalix-os/issues/41) Global Menu | Existing native menu integration retained. Responses now identify their window; stale-window replies are rejected. Guessed shortcut menus and generic fallback are off by default; explicit choices remain configurable. A real GTK3 fixture exports a menu, invokes its actual action and preserves disabled state. | Representative GTK4, Qt and Electron action/disabled-state/keyboard matrix is still required. |
| [#43](https://github.com/danielmigueltejedor/nodalix-os/issues/43) Default shortcuts | GNOME native custom shortcuts for Super+C/V with a fixed-action, active-unlocked-seat clipboard broker; no keyboard Shell extension; default terminal/browser/mail launchers; native full/area capture; Super+Q closes the focused window; hover focus without auto-raise. Exact legacy keyd and dock close/screenshot conflicts are migrated while preserving custom values. Real GTK, Qt and Ghostty tests pass. | Unified Settings shortcut controls and a wider application/accessibility matrix remain. See [shortcuts](shortcuts-0.4.0.md). |
| [#44](https://github.com/danielmigueltejedor/nodalix-os/issues/44) Updater | Missing/corrupt identity fails explicitly instead of 0.0.0. Native package-version list, real busy spinner, signal-based UI, bounded private persisted logs, interrupted-state recovery, error hints, critical-package reboot detection, locks/disk/battery preflight and database validation. Four persistent automatic-update switches request authorization on enable only, with owner-controlled password-free disable, actual timer state, cancellation/partial-failure recovery and fixed scheduled jobs. | Per-package origin/download size, guided keyring repair, complete history/preferences/translations and clean/historical-upgrade VM matrix remain. No signature bypass or partial `pacman -Sy` operation was added. |
| [#45](https://github.com/danielmigueltejedor/nodalix-os/issues/45) Notification height | Calendar and notification area share a work-area height cap independent of optional Phone Link; native scrollers retain reachable controls and shrink with content. Real long-list and keyboard focus/scroll tests pass. | Fractional/HiDPI and more physical resolutions still need validation. |
| [#46](https://github.com/danielmigueltejedor/nodalix-os/issues/46) System resource audit | Opt-in privacy-aware report and sampler; actual active-desktop sample; reduced layout timer schedule, skipped unconfigured iCloud mount and removal of Updates' idle three-second poll. | Controlled 5/15-minute idle, upstream Arch comparison, hardware wakeups/disk/GPU power, battery, sustained-session and no-regression measurements remain. |

Apple's [ANCS specification](https://developer.apple.com/library/archive/documentation/CoreBluetooth/Reference/AppleNotificationCenterServiceSpecification/Specification/Specification.html)
defines notification events/attributes, rather than a public global named-Focus
state feed. Apple's [Focus status integration](https://developer.apple.com/documentation/UserNotifications/handling-communication-notifications-and-focus-status-updates)
requires iOS application integration. A silent-notification flag is not proof of
the current global Focus mode, so this candidate does not infer one.
This also agrees with the existing [technical investigation on #15](https://github.com/danielmigueltejedor/nodalix-os/issues/15#issuecomment-5921527402), which identifies the authorized iOS source/transport as the missing dependency.

## Local validation

- 293 unit/integration tests passed on the actual PC, including real GJS/GIO tests and the upstream ZRAM generator fixtures.
- CI/release native validation is pinned to Ubuntu 26.04 with Nautilus GIR 4.1. The previous `ubuntu-latest` setup requested the removed GIR 4.0 package and failed before executing tests. The existing Nautilus bridge supports both API versions; this PC validates its real 4.1 typelib.
- Previous revision: five changed packages built locally: `nodalix-updater`, `nodalix-gnome`, `nodalix-integrations`, `nodalix-settings` 0.4.0-2 and `nodalix-control-center` 51.0.0-8; updater and GNOME are also 0.4.0-2, integrations remains 0.4.0-1. The last package compiles the complete pinned Control Center and its native additions.
- A private GNOME Shell 51.1 Wayland session tests two virtual 1280×720 monitors. It has a separate bus, home, configuration and runtime directory, and does not replace the user's desktop.
- Native notifications: a live call remains first after 18 other sources, including a critical notification; Clear all preserves it; dismissal is hidden; native action signals and backend replacement/closure are exercised.
- The notification viewport is 447 pixels high on the 720-pixel monitor with a 2,276-pixel list. The Clear control ends at y=536; keyboard focus reaches the last item through the native scrolling path.
- Auto Tiling: actual GTK window frames match expected rectangles for 1–4 windows, with minimize/restore, independent monitor groups and disable without logout.
- Global Menu: an actual GTK3 GMenu export produces its native menu, executes an application action, retains its disabled action state and rejects a stale-window response.
- Native Updates: compiled page exercised in the private Wayland session against a fake backend; mandatory restart disables updates and invokes the fake restart exactly once. The test never restarts the PC. Automatic switches restore after cancelled authorization, persist across page recreation and disable correctly.
- Actual key injection verifies GTK and Qt copy/paste, Ghostty paste and no-selection copy without interrupting the terminal process, unchanged Ctrl+C interruption, native area selection and a saved full-screen capture. Pointer movement changes focus without clicking; Super+Q closes exactly one focused window. Browser/mail launch helpers honor configured defaults through isolated application fixtures.

The raw QA directory is `dist/qa-040/` and is not committed because ordinary
desktop logs can contain local paths. Reviewable sanitized evidence is in
`docs/validation/0.4.0/`. The preceding candidate was installed locally as 0.4.0-dev after a recovery snapshot. It has not been tagged or published as a stable release. All six CI jobs (validation, packages, clean installation, historical upgrades, 0.2.4 migration and ISO build) passed on `e805966`; this native-shortcut/GPU revision requires fresh CI results. Its four new packages compile locally: GNOME and Settings 0.4.0-3, Control Center 51.0.0-9, video wallpapers 0.4.0-2.

## Reproducing resource measurements

Run `nodalix-diagnostics --json` for a local support report. Add `--sample 10` for
an explicit ten-second sample; allowed sampling durations are 1–60 seconds.
Nothing is sent over the network or scheduled automatically. Reports deliberately
exclude process command lines/environments, host/user names, network addresses,
device identifiers, notification/clipboard contents and personal files.

For an idle baseline, use the same kernel, driver, display refresh/scale, power
profile, extensions and workload. Wait 5 and 15 minutes after login, avoid builds
and UI interaction, record both cases, then repeat under an upstream Arch+GNOME
installation with equivalent settings. Record locked, fullscreen, normal use
and return-to-desktop separately. Do not compare this working session against
an idle reference or infer battery watts from context switches.

The checked-in sample was measured during **active development**, with the
installed identity still at 0.3.0 and GNOME Shell 51.1. Shell used approximately
34.83% of one CPU core and 300.8 MiB PSS over ten seconds. This identifies Shell
as a follow-up profiling target for that workload; it is **not** an idle finding,
a before/after improvement, or an upstream comparison. No battery was available
for a laptop discharge measurement. Context-switch rate is not hardware wakeup
rate; RSS shares pages between processes, so PSS is preferable when accessible.

The layout timer's regular reconciliation schedule changes from at most roughly
1,440 daily minute intervals to 48 half-hour intervals, plus real filesystem
events. This is a scheduling change, not a measured whole-system CPU/power saving.
The packaged ZRAM default uses `/usr/lib/systemd/zram-generator.conf`, so the
administrator's `/etc/systemd/zram-generator.conf` takes precedence. Disk swap
and hibernation policy are not removed or rewritten.

## Graphics revision

See [the measured GPU evidence](validation/0.4.0/wallpaper-gpu.json). With the same original 3840×2160 H264 30 fps video and two 1920×1080 private monitors, this trial measured compositor graphics-engine occupancy from 0.506% to 0.395%; combined Shell/renderer CPU from 4.17% to 3.58% of one core. These are short samples on one AMD GPU, not universal thresholds or battery measurements. Decode activity remains necessary while the video is visible. With eight ordinary windows covering the monitors, renderer CPU and reported GPU engines were 0% over the three-second sample, and playback resumed after reveal. Per-client VRAM varies and can include shared allocations; no memory-saving claim is inferred from those counters. The Theora test verifies the software compatibility path without changing the original collection.

A fast cursor move to the bottom edge now reveals both docks: the throttled dwell callback reads the latest pointer position. Cleanup archives replaced local extensions and preferences; it preserves unrelated extensions and user customizations. Shell code replacement needs a new login. The update icon resolves through the active theme to the Adwaita symbolic refresh asset.

## Native panels, popup height and application folders

The development-only replacement bar has been removed. Multi Monitor Bar mirrors
GNOME on the secondary displays. It leaves the primary panel allocation, clipping
and clock placement to GNOME. All panel, popup and dock blur comes from Blur My
Shell. Its hidden popup surfaces are removed even while a descendant animation
continues, preventing a blur outline from outliving its menu. User blur pipelines
and preferences remain intact.

Tiling Assistant is retained, including newer user-local copies. The calendar
agenda restores its previous 12em height limit; notifications scroll within the
monitor work area independently of Phone Link. The same cap applies on secondary
monitors. Shell module replacement requires a new login.

The dock blur follows the actual background allocation and startup translations.
Private GNOME 51 tests exercise the first appearance before hide/show, both native
menus, preserved primary Quick Settings, hidden blur surfaces, monitor changes,
manual tiling and native shortcuts. Packages: nodalix-gnome 0.4.0-7 and
nodalix-integrations 0.4.0-2.

Application folders discover publisher namespaces and ArtCraft metadata rather
than a closed product list. System and Flatpak applications follow desktop-file
precedence, hidden overrides and categories. Custom user folders are retained;
ArtCraft suite membership is additional to the app's purpose folder, so CADCraft
and SolidCraft also appear in Engineering and the document tools also appear in
Office. Personal folder contents and exclusions are not rewritten. Folder changes have a private backup
and directory events coalesce without polling or repeated gsettings subprocesses.

The default appearance uses GNOME's native panel plus Blur My Shell. Global Menu
and Rounded Window Corners are enabled in the 0.4.0 profile. A one-time upgrade
repair restores both after an earlier development candidate disabled them.
Nodalix no longer seeds custom blur pipelines or brightness;
only its previously imposed brightness preset and popup cache are reset, while
custom pipeline choices remain. Two- and three-monitor native menu tests pass.

## Calendar agenda and official Blur My Shell

The earlier CSS-only agenda limit did not survive popup restyling. The native
scroll viewport now requests at most 12em (bounded by 30% of the monitor work
area), recalculates when events arrive and retains scrolling. Private GNOME tests
exercise 36 events followed by 72 on every monitor: the agenda stays 175 px and
the complete popup 600 px at the tested default font and 720 px monitor height.

Blur My Shell 77 is copied byte-for-byte from the published GNOME Extensions
archive (version tag 77413, SHA-256 recorded with source file hashes). The old
modified copy, shader compatibility code and subpixel geometry hacks are removed.
Its official preference migration removes manual corner effects from legacy
pipelines, so those added panel effects do not return at login. The native panel,
menu controllers and Tiling Assistant remain. Secondary panel teardown drops
notification-source handlers and ignores deferred blur registrations for panels
that no longer exist.

The complete private GNOME tests also load this PC's actual blur preferences in
an isolated session; no real calendar events, windows or personal screenshots are
committed. Replacing already loaded Shell modules requires a new login.

## Live wallpaper below the native panel

The physical session exposed a missing integration check: Hanabi cropped every
background to the work area, including Blur My Shell's private input backgrounds.
The first 32 pixels were therefore missing on each monitor, producing coloured
strips in static blur and white echoes of native panel text in dynamic blur.
Desktop and blur input backgrounds now cover the entire monitor. Only rounded
Overview workspace previews retain the work-area crop. Blur My Shell remains the
unmodified official version 77; the fix changes wallpaper input geometry only.

The real-GBM two-monitor regression first fails on the old clipped input, then
passes against the compiled wallpaper package: both static and dynamic panel
samples show the fixture background, video keeps moving, Overview rounding is
restored, and coverage pause/reveal still works. After loading the fix in a fresh
physical session, the user confirmed the panel is correct. GNOME Shell's 389
packaged files remain unmodified. The follow-up package also restores Global
Menu and Rounded Window Corners as requested system defaults.

## ArtCraft folders and supplied application icons

The old per-user organizer service shadowed the packaged helper and still used a
closed list of tools. A one-time migration archives only the exact known service,
watcher and helper prototypes, switches to the packaged event-driven organizer
and preserves customized units. Current and future suite apps use publisher
namespaces or explicit suite metadata, independently of their standard category.
The same desktop ID is present in both native folders; no extra launcher or Shell
extension is created. A real GNOME app-grid check verifies the shared membership.

The icon resolver also uses the suite namespaces rather than a closed name list.
It resolves TextureCraft, Craft Anything, FigCraft, SolidCraft and future apps to
their supplied hicolor assets, retaining launch commands, actions and personal
icon choices. The local SolidCraft launcher was also corrected to its installed
AppImage version; this local launcher repair does not claim to implement a
general AppImage update manager.

## Updates, release identity and GNOME contacts

Settings translates updater status codes into readable Spanish text. The About
page displays the Nodalix release version separately from the Arch base and
watches the release file for atomic replacement without polling.

Automatic system transactions defer with status 75 when pacman is busy; systemd
retries these transient conditions after five minutes without a daily retry gap.
Settings reports the last failed or deferred job even when its timer is enabled.
The authorized local system job completed six package upgrades without further
authentication; the subsequent repository check found no pending system updates.
No automatic reboot was performed.

Phone Link offers a searchable native GNOME Contacts picker with each saved
phone number. Reading Evolution Data Server is asynchronous and cancellable;
selecting a number preserves its stored country prefix. Names are read from
vCard attributes because the untyped contact getter exposes pointers in current
GI. Native widget tests use synthetic contacts and a fake dial backend; no real
call was placed. The desktop integration reads existing address books only.

## Applications, storage and Flatpak

Files presents installed desktop applications in the localized Applications
folder with a themed folder icon and the supplied app icons. Existing AppImages
are adopted with compatibility paths and a private backup. New supported files
can open the installer or be dropped into Applications. Registered descriptors
launch their actual desktop app. A verified Trash move from Applications asks
the appropriate provider to uninstall it; renames and moves elsewhere are not
uninstall requests, and personal data is preserved. The native test installs,
opens and uninstalls a disposable AppImage using a real GIO Trash move. The user
also confirmed FilmCraft opens from its Applications entry after the Flatpak
document portal recovered. The launcher checks that mount before opening Flatpak.

Storage is a native subpage of Applications. It scans on demand in a separate
process and shows volume usage, apps ordered by installed size, large documents,
music, photos/video, downloads and common development outputs and caches. The
file inventory stays within selected personal folders, avoids following links,
and labels partial results. Native widget and filesystem tests exercise sorting
and both Trash routes with disposable data. No personal files were removed.

Flatpak checks separate installed origins and scopes. An obsolete local remote
no longer blocks Flathub, and bundles without an update repository are disclosed.
The existing Flatpak preference on the test PC was repaired; both scheduled
services and the Settings check completed successfully. The prior clean-boot VM
checks aborted before reaching the screenshot test on an obsolete Blur My Shell
key. The new checks verify the official schema and active panel blur, and retain
real portal readiness and PNG capture checks.

## Phone notification connection

BlueZ can request advertisement properties during registration, so synchronous
registration could leave the advertisement inactive. Asynchronous registration
keeps the callback available and survives owner changes. A live ANCS subscription
is authoritative even when the experimental LE bearer property is false. Address
resolution is applied to paired iPhones before ANCS discovery, preserving other
controller flags. The adapter now sets A/V Hands-Free using the correct fields.
The active advertisement, address resolution and controller class were observed
on the test PC; phone-side notification delivery remains a separate live check.
