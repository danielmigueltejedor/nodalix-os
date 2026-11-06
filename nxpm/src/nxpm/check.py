"""Record update checks. This never installs or removes software."""

from __future__ import annotations

import json
from datetime import datetime, timezone

from nxpm.paths import state_dir


def record_check(updates: list[dict], notify: bool) -> None:
    payload = {
        "checked_at": datetime.now(timezone.utc).isoformat(),
        "count": len(updates),
        "updates": updates,
    }
    path = state_dir() / "updates.json"
    path.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    if notify and updates:
        _notify(len(updates))


def _notify(count: int) -> None:
    try:
        import gi

        gi.require_version("Notify", "0.7")
        from gi.repository import Notify
    except Exception:
        return
    Notify.init("Nodalix Store")
    note = Notify.Notification.new(
        "Software updates",
        f"{count} update(s) are available. Open Nodalix Store to review them.",
        "software-update-available",
    )
    try:
        note.show()
    except Exception:
        return
