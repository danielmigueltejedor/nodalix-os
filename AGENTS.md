# Agent instructions — nodalix-os

## Repository-specific deterministic quality contract (2026-10-26)

**Project:** Arch Linux GNOME-based OS distribution with updater, ISO images and package migration.

**Engineering requirements:** Never weaken kernel/boot/updater integrity, upgrade path from supported releases, pacman atomicity or recovery. Test migrations on clean system images.

**Verification commands:** /usr/bin/python3 -m unittest discover -s tests -v; bash tools/test-native-restart.sh; existing package migration, clean install and ISO verification jobs.

Run relevant tests and give actual outputs, including failures. Add regression coverage for behavior, edges and error paths. Never weaken/delete/skip tests, suppress errors or reduce thresholds solely to get CI green. Changes to this file, `QUALITY_GATES.md`, workflows and test policy require human review. Make the `quality` check required on protected branches. Disclose missing system dependencies. Preserve existing instructions.
