# 0.4.0 preparation and complete open-issue audit

Audit date: 2026-10-31. Base: `b0e557b` on `main` (0.3.1 preparation).
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
| [#26](https://github.com/danielmigueltejedor/nodalix-os/issues/26) Adaptive Hanabi | Existing pause preferences, pinned renderer, VA plugin dependency and cached stills retained. | Unified Gaming/Director integration, decode/power measurements and complete visibility-transition validation remain. |
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
| [#43](https://github.com/danielmigueltejedor/nodalix-os/issues/43) Default shortcuts | Existing user shortcuts are preserved. | Terminal-aware edit actions, Super+Enter defaults, conflict handling, Settings controls and application matrix remain unimplemented. |
| [#44](https://github.com/danielmigueltejedor/nodalix-os/issues/44) Updater | Missing/corrupt identity fails explicitly instead of 0.0.0. Native package-version list, real busy spinner, signal-based UI, bounded private persisted logs, interrupted-state recovery, error hints, critical-package reboot detection, locks/disk/battery preflight and database validation. | Per-package origin/download size, guided keyring repair, complete history/preferences/translations and clean/historical-upgrade VM matrix remain. No signature bypass or partial `pacman -Sy` operation was added. |
| [#45](https://github.com/danielmigueltejedor/nodalix-os/issues/45) Notification height | Existing native scroller is capped to the monitor work area and content height; footer stays reachable and empty list shrinks. Real long-list and keyboard focus/scroll tests pass. | Fractional/HiDPI and more physical resolutions still need validation. |
| [#46](https://github.com/danielmigueltejedor/nodalix-os/issues/46) System resource audit | Opt-in privacy-aware report and sampler; actual active-desktop sample; reduced layout timer schedule, skipped unconfigured iCloud mount and removal of Updates' idle three-second poll. | Controlled 5/15-minute idle, upstream Arch comparison, hardware wakeups/disk/GPU power, battery, sustained-session and no-regression measurements remain. |

Apple's [ANCS specification](https://developer.apple.com/library/archive/documentation/CoreBluetooth/Reference/AppleNotificationCenterServiceSpecification/Specification/Specification.html)
defines notification events/attributes, rather than a public global named-Focus
state feed. Apple's [Focus status integration](https://developer.apple.com/documentation/UserNotifications/handling-communication-notifications-and-focus-status-updates)
requires iOS application integration. A silent-notification flag is not proof of
the current global Focus mode, so this candidate does not infer one.
This also agrees with the existing [technical investigation on #15](https://github.com/danielmigueltejedor/nodalix-os/issues/15#issuecomment-5921527402), which identifies the authorized iOS source/transport as the missing dependency.

## Local validation

- 237 unit/integration tests passed on the actual PC, including real GJS/GIO tests and the upstream ZRAM generator fixtures.
- CI/release native validation is pinned to Ubuntu 26.04 with Nautilus GIR 4.1. The previous `ubuntu-latest` setup requested the removed GIR 4.0 package and failed before executing tests. The existing Nautilus bridge supports both API versions; this PC validates its real 4.1 typelib.
- Five changed packages built locally: `nodalix-updater`, `nodalix-gnome`, `nodalix-integrations`, `nodalix-settings` 0.4.0-1 and `nodalix-control-center` 51.0.0-7. The last package compiles the complete pinned Control Center and its native additions.
- A private GNOME Shell 51.1 Wayland session tests two virtual 1280×720 monitors. It has a separate bus, home, configuration and runtime directory, and does not replace the user's desktop.
- Native notifications: a live call remains first after 18 other sources, including a critical notification; Clear all preserves it; dismissal is hidden; native action signals and backend replacement/closure are exercised.
- The notification viewport is 447 pixels high on the 720-pixel monitor with a 2,276-pixel list. The Clear control ends at y=536; keyboard focus reaches the last item through the native scrolling path.
- Auto Tiling: actual GTK window frames match expected rectangles for 1–4 windows, with minimize/restore, independent monitor groups and disable without logout.
- Global Menu: an actual GTK3 GMenu export produces its native menu, executes an application action, retains its disabled action state and rejects a stale-window response.
- Native Updates: compiled page exercised in the private Wayland session against a fake backend; mandatory restart disables updates and invokes the fake restart exactly once. The test never restarts the PC.

The raw QA directory is `dist/qa-040/` and is not committed because ordinary
desktop logs can contain local paths. Reviewable sanitized evidence is in
`docs/validation/0.4.0/`. The candidate has not been installed as the running OS,
tagged, released or published as an ISO. Privileged install/upgrade/recovery and
the complete clean-install/historical-upgrade/ISO CI jobs remain release gates.

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
