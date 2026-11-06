"""Administrative CLI. It calls the same engine as the D-Bus service."""

from __future__ import annotations

import argparse
import json
import logging
import sys

from nxpm.errors import NxpmError


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="nxpm", description="Nodalix Package Manager")
    parser.add_argument("--json", action="store_true", help="Print machine-readable JSON")
    sub = parser.add_subparsers(dest="command", required=True)

    sub.add_parser("sources", help="Show software sources and whether they are implemented")
    sub.add_parser("daemon", help="Run the session service")
    sub.add_parser("check", help="Look for updates without installing them")
    sub.add_parser("updates", help="List available updates")
    sub.add_parser("privileged-upgrade", help=argparse.SUPPRESS)

    installed = sub.add_parser("list", help="List installed packages")
    installed.add_argument("--source", default="", choices=["", "flatpak", "pacman", "appimage"])

    imported = sub.add_parser("import", help="Copy an AppImage or Debian package into ~/Applications")
    imported.add_argument("kind", choices=["appimage", "deb"])
    imported.add_argument("path")

    permissions = sub.add_parser("permissions", help="Show the permissions a Flatpak declares")
    permissions.add_argument("package_id")

    reset = sub.add_parser("permissions-reset", help="Remove local Flatpak overrides for one user application")
    reset.add_argument("package_id")

    search = sub.add_parser("search", help="Search enabled sources")
    search.add_argument("query")

    show = sub.add_parser("show", help="Show one package")
    show.add_argument("package_id")

    install = sub.add_parser("install", help="Install a Flatpak for the current user")
    install.add_argument("package_id")
    install.add_argument("--source", default="flatpak")

    remove = sub.add_parser("remove", help="Remove a user Flatpak")
    remove.add_argument("package_id")
    remove.add_argument("--source", default="flatpak")

    update = sub.add_parser("update", help="Update one Flatpak")
    update.add_argument("package_id")

    sub.add_parser("upgrade-system", help="Apply official repository updates together")

    status = sub.add_parser("status", help="Show a transaction")
    status.add_argument("transaction_id")

    cancel = sub.add_parser("cancel", help="Cancel a transaction that has not started")
    cancel.add_argument("transaction_id")

    history = sub.add_parser("history", help="Show finished operations")
    history.add_argument("--limit", type=int, default=20)

    sub.add_parser("install-session", help="Install the user service and the update check timer")

    args = parser.parse_args(argv)
    logging.basicConfig(level=logging.INFO, format="nxpm: %(message)s")
    try:
        return run(args)
    except NxpmError as exc:
        print(f"{exc.code}: {exc.message}", file=sys.stderr)
        return 2


def run(args) -> int:
    if args.command == "install-session":
        from nxpm.session import install_session

        install_session()
        return 0
    if args.command == "daemon":
        from nxpm.service import serve

        return serve()
    if args.command == "privileged-upgrade":
        from nxpm.backends.pacman import PacmanBackend

        PacmanBackend().upgrade_system(lambda _progress, message: print(message))
        return 0

    from nxpm.engine import build_engine

    engine = build_engine()
    if args.command == "sources":
        return emit(args, engine.sources())
    if args.command == "list":
        return emit(args, [item.to_dict() for item in engine.list_installed(args.source)])
    if args.command == "search":
        return emit(args, [item.to_dict() for item in engine.search(args.query)])
    if args.command == "show":
        return emit(args, engine.details(args.package_id).to_dict())
    if args.command == "updates" or args.command == "check":
        updates = [item.to_dict() for item in engine.list_updates()]
        if args.command == "check":
            from nxpm.check import record_check

            record_check(updates, notify=engine.config.notify and engine.config.check_updates)
        return emit(args, updates)
    if args.command == "import":
        return emit(args, engine.import_file(args.kind, args.path).record().to_dict())
    if args.command == "permissions":
        return emit(args, engine.permissions(args.package_id))
    if args.command == "permissions-reset":
        return emit(args, engine.reset_permissions(args.package_id))
    if args.command == "install":
        return emit(args, engine.install(args.package_id, args.source).record().to_dict())
    if args.command == "remove":
        return emit(args, engine.remove(args.package_id, args.source).record().to_dict())
    if args.command == "update":
        return emit(args, engine.update(args.package_id).record().to_dict())
    if args.command == "upgrade-system":
        return emit(args, engine.upgrade_system().record().to_dict())
    if args.command == "status":
        return emit(args, engine.transactions.get(args.transaction_id).record().to_dict())
    if args.command == "cancel":
        return emit(args, {"cancelled": engine.transactions.cancel(args.transaction_id)})
    if args.command == "history":
        return emit(args, engine.transactions.history(args.limit))
    raise NxpmError("invalid-input", "Unknown command.")


def emit(args, payload) -> int:
    if args.json:
        print(json.dumps(payload, indent=2, ensure_ascii=False))
    elif isinstance(payload, list):
        if not payload:
            print("No results.")
        for item in payload:
            print(format_row(item))
    elif isinstance(payload, dict):
        for key, value in payload.items():
            print(f"{key}: {value}")
    else:
        print(payload)
    return 0


def format_row(item: dict) -> str:
    if "name" in item:
        mark = " preferred" if item.get("preferred") else ""
        update = " update" if item.get("update") else ""
        return f"{item.get('source', ''):8} {item.get('id', '')} {item.get('version', '')}{update}{mark}"
    if "id" in item and "implemented" in item:
        state = "enabled" if item["enabled"] else "disabled"
        return f"{item['id']:10} {state}: {item['reason']}"
    if "state" in item:
        return f"{item['id']} {item['state']} {item.get('message', '')}"
    if "risk" in item and "value" in item:
        return f"{item['risk']:6} {item.get('origin', ''):9} {item.get('kind', '')}={item['value']}"
    return json.dumps(item, ensure_ascii=False)
