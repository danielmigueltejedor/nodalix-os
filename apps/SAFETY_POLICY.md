# Nodalix App Safety Policy

## Rules

- No destructive actions without confirmation.
- No reboot, shutdown, logout, or suspend without clear intent; reboot and shutdown require confirmation.
- No system modifications in prototypes.
- No root requirement unless absolutely necessary and documented.
- Missing dependencies must fail gracefully.
- Logs must not contain secrets.
- Passwords must never be logged.
- System configuration changes must create backups.
- Every app should expose a safe development mode when relevant.

## Scope

These rules apply to apps in `apps/`, wrappers in `local/bin`, and future integration scripts.

