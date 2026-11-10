"""Record update checks. This never installs or removes software."""

from __future__ import annotations

import json
import os
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


# The store is shown by a translated name, like the stock apps of other systems
# (English "Store", Spanish "Tienda"); the notification follows the same rule.
_STORE_TEXT = {
    "es": ("Tienda", "Actualizaciones de software",
           "Hay {count} actualización(es) disponible(s). Abre Tienda para revisarlas."),
    "ca": ("Botiga", "Actualitzacions de programari",
           "Hi ha {count} actualització(ns) disponible(s). Obre Botiga per revisar-les."),
    "fr": ("Boutique", "Mises à jour logicielles",
           "{count} mise(s) à jour disponible(s). Ouvrez Boutique pour les consulter."),
    "pt": ("Loja", "Atualizações de software",
           "Há {count} atualização(ões) disponível(is). Abra a Loja para revê-las."),
    "it": ("Negozio", "Aggiornamenti software",
           "Sono disponibili {count} aggiornamento/i. Apri Negozio per esaminarli."),
}
_STORE_DEFAULT = ("Store", "Software updates",
                  "{count} update(s) are available. Open Store to review them.")


def store_text(environ: dict[str, str] | None = None) -> tuple[str, str, str]:
    """Return (application name, title, body template) for the user's language."""
    environ = os.environ if environ is None else environ
    locale = next((environ[v] for v in ("LC_ALL", "LC_MESSAGES", "LANG") if environ.get(v)), "")
    if locale.split(".")[0] in ("", "C", "POSIX"):
        return _STORE_DEFAULT  # gettext ignores LANGUAGE in the C locale
    # LANGUAGE is an ordered fallback list and wins over the locale name itself.
    for entry in [*environ.get("LANGUAGE", "").split(":"), locale]:
        language = entry.split(".")[0].split("@")[0].split("_")[0].lower()
        if language in _STORE_TEXT:
            return _STORE_TEXT[language]
        if language == "en":
            break
    return _STORE_DEFAULT


def _notify(count: int) -> None:
    try:
        import gi

        gi.require_version("Notify", "0.7")
        from gi.repository import GLib, Notify
    except Exception:
        return
    name, title, body = store_text()
    Notify.init(name)
    note = Notify.Notification.new(title, body.format(count=count), "os.nodalix.Store")
    note.set_hint("desktop-entry", GLib.Variant.new_string("os.nodalix.Store"))
    try:
        note.show()
    except Exception:
        return
