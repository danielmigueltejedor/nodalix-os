#!/usr/bin/env python3
from __future__ import annotations

import glob
import json
import shutil
import subprocess
from pathlib import Path


HARDENING_KEYS = {
    "NoNewPrivileges",
    "PrivateTmp",
    "ProtectSystem",
    "ProtectHome",
    "ProtectKernelTunables",
    "ProtectKernelModules",
    "RestrictSUIDSGID",
    "CapabilityBoundingSet",
}


def run(args: list[str]) -> subprocess.CompletedProcess[str]:
    try:
        return subprocess.run(
            args,
            check=False,
            capture_output=True,
            text=True,
            timeout=8,
        )
    except (FileNotFoundError, subprocess.TimeoutExpired):
        return subprocess.CompletedProcess(args, 127, "", "")


def service_state(unit: str) -> tuple[bool, bool]:
    active = run(["systemctl", "is-active", unit]).returncode == 0
    enabled = run(["systemctl", "is-enabled", unit]).returncode == 0
    return active, enabled


def firewall_status() -> dict[str, object]:
    if shutil.which("ufw"):
        active, enabled = service_state("ufw.service")
        return {
            "provider": "UFW",
            "installed": True,
            "active": active,
            "enabled": enabled,
        }
    if shutil.which("firewall-cmd"):
        active, enabled = service_state("firewalld.service")
        return {
            "provider": "firewalld",
            "installed": True,
            "active": active,
            "enabled": enabled,
        }
    if shutil.which("nft"):
        active, enabled = service_state("nftables.service")
        return {
            "provider": "nftables",
            "installed": True,
            "active": active,
            "enabled": enabled,
        }
    return {
        "provider": "",
        "installed": False,
        "active": False,
        "enabled": False,
    }


def secure_boot_status() -> dict[str, object]:
    efi = Path("/sys/firmware/efi")
    if not efi.is_dir():
        return {"uefi": False, "enabled": False}

    variables = glob.glob("/sys/firmware/efi/efivars/SecureBoot-*")
    if not variables:
        return {"uefi": True, "enabled": False}

    try:
        raw = Path(variables[0]).read_bytes()
        enabled = len(raw) >= 5 and raw[4] == 1
    except OSError:
        enabled = False
    return {"uefi": True, "enabled": enabled}


def pacman_signature_status() -> dict[str, object]:
    path = Path("/etc/pacman.conf")
    siglevels: list[str] = []
    local = ""
    if path.is_file():
        for raw in path.read_text(encoding="utf-8", errors="replace").splitlines():
            line = raw.strip()
            if not line or line.startswith("#") or "=" not in line:
                continue
            key, value = (part.strip() for part in line.split("=", 1))
            if key == "SigLevel":
                siglevels.append(value)
            elif key == "LocalFileSigLevel" and not local:
                local = value

    required = bool(siglevels) and all(
        "Never" not in value and "Required" in value
        for value in siglevels
    )
    return {
        "required": required,
        "siglevels": siglevels,
        "local_file_siglevel": local,
    }


def keyring_status() -> dict[str, object]:
    result = run(["pacman", "-Q", "archlinux-keyring"])
    if result.returncode != 0:
        return {"installed": False, "version": ""}
    parts = result.stdout.strip().split(maxsplit=1)
    return {
        "installed": True,
        "version": parts[1] if len(parts) > 1 else "",
    }


def foreign_packages() -> dict[str, object]:
    result = run(["pacman", "-Qm"])
    packages: list[dict[str, str]] = []
    if result.returncode == 0:
        for line in result.stdout.splitlines():
            parts = line.split(maxsplit=1)
            if not parts:
                continue
            packages.append({
                "name": parts[0],
                "version": parts[1] if len(parts) > 1 else "",
            })
    return {"count": len(packages), "packages": packages}


def flatpak_permissions() -> dict[str, object]:
    if not shutil.which("flatpak"):
        return {"installed": False, "broad_count": 0, "broad_apps": []}

    apps: dict[str, dict[str, object]] = {}
    for scope in ("--user", "--system"):
        listed = run(["flatpak", "list", scope, "--app", "--columns=application,name"])
        if listed.returncode != 0:
            continue
        for line in listed.stdout.splitlines():
            if not line.strip():
                continue
            parts = line.split("\t", 1)
            app_id = parts[0].strip()
            if not app_id:
                continue
            name = parts[1].strip() if len(parts) > 1 else app_id
            perms = run(["flatpak", "info", scope, "--show-permissions", app_id])
            if perms.returncode != 0:
                continue

            reasons: set[str] = set()
            section = ""
            for raw in perms.stdout.splitlines():
                line = raw.strip()
                if line.startswith("[") and line.endswith("]"):
                    section = line[1:-1]
                    continue
                if "=" not in line:
                    continue

                key, value = line.split("=", 1)
                values = [item for item in value.split(";") if item]

                if section == "Context" and key == "filesystems":
                    if any(item == "home" or item.startswith("home:") for item in values):
                        reasons.add("home-filesystem")
                    if any(item == "host" or item.startswith("host-") for item in values):
                        reasons.add("host-filesystem")
                elif section == "Context" and key == "devices" and "all" in values:
                    reasons.add("all-devices")
                elif section == "Session Bus Policy" and key == "*" and value in {"talk", "own"}:
                    reasons.add("session-bus")
                elif section == "System Bus Policy" and key == "*" and value in {"talk", "own"}:
                    reasons.add("system-bus")

            if not reasons:
                continue
            entry = apps.setdefault(
                app_id,
                {"id": app_id, "name": name, "reasons": set()},
            )
            entry["reasons"].update(reasons)

    broad_apps = [
        {
            "id": value["id"],
            "name": value["name"],
            "reasons": sorted(value["reasons"]),
        }
        for value in apps.values()
    ]
    broad_apps.sort(key=lambda item: str(item["name"]).lower())
    return {
        "installed": True,
        "broad_count": len(broad_apps),
        "broad_apps": broad_apps,
    }


def parse_service(path: Path) -> dict[str, object]:
    directives: dict[str, str] = {}
    in_service = False
    try:
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError:
        lines = []

    for raw in lines:
        line = raw.strip()
        if line.startswith("[") and line.endswith("]"):
            in_service = line == "[Service]"
            continue
        if not in_service or not line or line.startswith("#") or "=" not in line:
            continue
        key, value = (part.strip() for part in line.split("=", 1))
        directives[key] = value

    user = directives.get("User", "")
    privileged = user in {"", "root"}
    hardening = sorted(key for key in HARDENING_KEYS if key in directives)
    return {
        "name": path.name,
        "privileged": privileged,
        "user": user or "root",
        "hardening": hardening,
    }


def nodalix_services() -> dict[str, object]:
    selected: dict[str, Path] = {}
    for root in (Path("/usr/lib/systemd/system"), Path("/etc/systemd/system")):
        if not root.is_dir():
            continue
        for path in root.glob("nodalix-*.service"):
            selected[path.name] = path

    services = [parse_service(path) for path in selected.values()]
    services.sort(key=lambda item: str(item["name"]))
    privileged = [item for item in services if item["privileged"]]
    without_hardening = [
        item for item in privileged
        if not item["hardening"]
    ]
    return {
        "total": len(services),
        "privileged_count": len(privileged),
        "without_hardening_count": len(without_hardening),
        "privileged": privileged,
        "without_hardening": without_hardening,
    }


def snapshot() -> dict[str, object]:
    return {
        "firewall": firewall_status(),
        "secure_boot": secure_boot_status(),
        "pacman_signatures": pacman_signature_status(),
        "keyring": keyring_status(),
        "foreign_packages": foreign_packages(),
        "flatpak": flatpak_permissions(),
        "nodalix_services": nodalix_services(),
    }


def main() -> int:
    print(json.dumps(snapshot(), ensure_ascii=False, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
