#!/usr/bin/env python3
import configparser
import os
import re
import subprocess
from pathlib import Path

HOME = Path.home()
CACHE = HOME / ".cache/nodalix/launcher-icons"
CACHE.mkdir(parents=True, exist_ok=True)

SYMBOLIC_DARK = CACHE / "symbolic-dark"
SYMBOLIC_LIGHT = CACHE / "symbolic-light"
SYMBOLIC_DARK.mkdir(parents=True, exist_ok=True)
SYMBOLIC_LIGHT.mkdir(parents=True, exist_ok=True)

SIZE = 64

# Pre-tinted neutral shell colors.
# No runtime tinting is performed by the launcher.
SYMBOLIC_ON_DARK = "#F0F0F0"
SYMBOLIC_ON_LIGHT = "#303030"

INTERNAL_ICONS = {
    "application-x-executable",
    "system-users-symbolic",
    "system-lock-screen-symbolic",
    "software-update-available-symbolic",
    "applications-utilities-symbolic",
    "drive-harddisk-symbolic",
    "preferences-desktop-appearance-symbolic",
    "view-grid-symbolic",
    "audio-speakers-symbolic",
    "preferences-desktop-wallpaper-symbolic",
    "send-to-symbolic",
    "preferences-system-notifications-symbolic",
    "preferences-desktop-keyboard-shortcuts-symbolic",
    "weather-clear-symbolic",
    "open-menu-symbolic",
    "preferences-desktop-display-symbolic",
    "applications-system-symbolic",
    "preferences-system-symbolic",
    "edit-find-symbolic",
    "text-x-generic-symbolic",
}

def cache_key(name):
    return re.sub(r"[^A-Za-z0-9._-]", "_", name or "application-x-executable")

roots = [
    HOME / ".local/share/icons/Colloid-Teal",
    HOME / ".icons/Colloid-Teal",
    Path("/usr/share/icons/Colloid-Teal"),
    HOME / ".local/share/icons/hicolor",
    Path("/usr/share/icons/hicolor"),
    Path("/usr/share/icons/Adwaita"),
    Path("/usr/share/pixmaps"),
]

index = {}

for root in roots:
    if not root.exists():
        continue
    for f in root.rglob("*"):
        if f.is_file() and f.suffix.lower() in {".svg", ".png", ".xpm"}:
            index.setdefault(f.stem, f)

def resolve(name):
    p = Path(name).expanduser()
    if p.is_absolute() and p.exists():
        return p
    return index.get(name) or index.get("application-x-executable")

def raster_symbolic(src, dst, color):
    base = CACHE / (".symbolic-base-" + dst.name)
    mask = CACHE / (".symbolic-mask-" + dst.name)

    try:
        if src.suffix.lower() == ".svg":
            subprocess.run([
                "rsvg-convert",
                "-w", str(SIZE),
                "-h", str(SIZE),
                "-o", str(base),
                str(src)
            ], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        else:
            subprocess.run([
                "magick",
                str(src),
                "-background", "none",
                "-resize", f"{SIZE}x{SIZE}",
                str(base)
            ], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

        # The cached raster becomes an alpha mask. The final RGB colour
        # is baked in here, never calculated by QML during a search.
        subprocess.run([
            "magick",
            str(base),
            "-alpha", "extract",
            str(mask)
        ], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

        subprocess.run([
            "magick",
            "-size", f"{SIZE}x{SIZE}",
            f"xc:{color}",
            str(mask),
            "-alpha", "off",
            "-compose", "CopyOpacity",
            "-composite",
            str(dst)
        ], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

        return True
    except Exception:
        return False
    finally:
        base.unlink(missing_ok=True)
        mask.unlink(missing_ok=True)


def raster(src, dst):
    tmp = dst.with_suffix(".tmp.png")

    try:
        if src.suffix.lower() == ".svg":
            subprocess.run([
                "rsvg-convert",
                "-w", str(SIZE),
                "-h", str(SIZE),
                "-o", str(tmp),
                str(src)
            ], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        else:
            subprocess.run([
                "magick",
                str(src),
                "-background", "none",
                "-resize", f"{SIZE}x{SIZE}",
                str(tmp)
            ], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

        tmp.replace(dst)
        return True
    except Exception:
        tmp.unlink(missing_ok=True)
        return False

icons = set(INTERNAL_ICONS)

application_dirs = [
    HOME / ".local/share/applications",
    HOME / ".local/share/flatpak/exports/share/applications",
    Path("/usr/local/share/applications"),
    Path("/usr/share/applications"),
    Path("/var/lib/flatpak/exports/share/applications"),
]

seen_desktops = set()

for directory in application_dirs:
    if not directory.exists():
        continue

    for desktop in directory.glob("*.desktop"):
        if desktop.name in seen_desktops:
            continue
        seen_desktops.add(desktop.name)

        parser = configparser.ConfigParser(interpolation=None, strict=False)
        parser.optionxform = str

        try:
            parser.read(desktop, encoding="utf-8")
            entry = parser["Desktop Entry"]

            if entry.get("NoDisplay", "false").lower() == "true":
                continue

            icon = entry.get("Icon", "").strip()
            if icon:
                icons.add(icon)
        except Exception:
            pass

made = 0
skipped = 0
failed = 0

for icon in sorted(icons):
    src = resolve(icon)
    if not src:
        failed += 1
        continue

    dst = CACHE / (cache_key(icon) + ".png")

    if dst.exists() and dst.stat().st_mtime >= src.stat().st_mtime:
        skipped += 1
        continue

    if raster(src, dst):
        made += 1
    else:
        failed += 1

# Generate pre-tinted variants for Nodalix/Adwaita symbolic icons.
symbolic_generated = 0

for icon in sorted(INTERNAL_ICONS):
    src = resolve(icon)
    if not src:
        continue

    key = cache_key(icon)

    targets = [
        (SYMBOLIC_DARK / (key + ".png"), SYMBOLIC_ON_DARK),
        (SYMBOLIC_LIGHT / (key + ".png"), SYMBOLIC_ON_LIGHT),
    ]

    for dst, color in targets:
        if dst.exists() and dst.stat().st_mtime >= src.stat().st_mtime:
            continue

        if raster_symbolic(src, dst, color):
            symbolic_generated += 1

print(f"Nodalix launcher icon cache: {made} generated, {skipped} current, {failed} unresolved; {symbolic_generated} symbolic variants generated")
