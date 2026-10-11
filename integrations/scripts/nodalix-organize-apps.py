#!/usr/bin/env python3
from pathlib import Path
import ast
import configparser
import subprocess

HOME = Path.home()

APP_DIRS = [
    Path("/usr/share/applications"),
    Path("/usr/local/share/applications"),
    Path("/var/lib/flatpak/exports/share/applications"),
    HOME / ".local/share/flatpak/exports/share/applications",
    HOME / ".local/share/applications",
]

FOLDERS = {
    "nodalix-craft": ["ArtCraft", []],
    "engineering": ["Ingeniería", []],
    "internet": ["Internet", []],
    "office": ["Oficina", []],
    "multimedia": ["Multimedia", []],
    "development": ["Desarrollo", []],
    "games": ["Juegos", []],
    "settings": ["Ajustes", []],
    "hardware": ["Hardware", []],
    "administration": ["Administración", []],
    "utilities": ["Utilidades", []],
    "other": ["Otros", []],
}

CRAFT_NAMES = (
    "photocraft", "vectorcraft", "filmcraft", "lightcraft", "pdfcraft",
    "effectcraft", "designcraft", "wordcraft", "deckcraft", "gridcraft",
    "cadcraft", "soundcraft",
)
CRAFT = {
    f"{publisher}.{name}.desktop"
    for publisher in ("ai.storyteller", "com.storytold")
    for name in CRAFT_NAMES
} | {"ai.storyteller.artcraft-launcher.desktop"}


ENGINEERING_PATTERNS = {
    "matlab",
    "simulink",
    "octave",
    "paraview",
    "opencad",
    "freecad",
    "openscad",
    "gmsh",
    "salome",
    "ansys",
    "comsol",
    "openfoam",
}

entries = {}

# /usr -> /usr/local -> ~/.local, de forma que el usuario tenga prioridad
for directory in APP_DIRS:
    if not directory.exists():
        continue
    for path in directory.glob("*.desktop"):
        entries[path.name] = path


def read_entry(path):
    cp = configparser.ConfigParser(
        interpolation=None,
        strict=False,
    )
    cp.optionxform = str

    try:
        cp.read(path, encoding="utf-8")
        if "Desktop Entry" not in cp:
            return None
        return cp["Desktop Entry"]
    except Exception:
        return None


for desktop_id, path in sorted(entries.items()):
    e = read_entry(path)

    if e is None:
        continue

    if e.get("Type", "Application") != "Application":
        continue

    if e.get("Hidden", "false").lower() == "true":
        continue

    if e.get("NoDisplay", "false").lower() == "true":
        continue

    name = e.get("Name", "")
    exec_cmd = e.get("Exec", "")

    searchable = " ".join([
        desktop_id,
        name,
        exec_cmd,
    ]).lower()

    categories = {
        value
        for value in e.get("Categories", "").split(";")
        if value
    }

    # Craft siempre tiene prioridad.
    if desktop_id in CRAFT:
        FOLDERS["nodalix-craft"][1].append(desktop_id)
        continue

    # Software científico/técnico.
    if any(word in searchable for word in ENGINEERING_PATTERNS):
        FOLDERS["engineering"][1].append(desktop_id)
        continue

    # Enlace móvil es una utilidad, no una carpeta Nodalix.
    if (
        desktop_id == "com.nodalix.PhoneLink.desktop"
        or name.lower() in {"enlace móvil", "phone link"}
    ):
        FOLDERS["utilities"][1].append(desktop_id)
        continue

    # Gestores de sesiones.
    if desktop_id == "com.pojtinger.felicitas.Sessions.desktop" or name.lower() in {"sesiones", "sessions"}:
        FOLDERS["utilities"][1].append(desktop_id)
        continue

    if "Game" in categories:
        FOLDERS["games"][1].append(desktop_id)
        continue

    if categories & {"Development", "IDE", "TextEditor"}:
        FOLDERS["development"][1].append(desktop_id)
        continue

    if categories & {
        "Office",
        "Spreadsheet",
        "WordProcessor",
        "Presentation",
        "Calendar",
    }:
        FOLDERS["office"][1].append(desktop_id)
        continue

    if categories & {
        "Network",
        "WebBrowser",
        "Email",
        "InstantMessaging",
        "Chat",
        "FileTransfer",
    }:
        FOLDERS["internet"][1].append(desktop_id)
        continue

    if categories & {
        "HardwareSettings",
        "X-GNOME-HardwareSettings",
    }:
        FOLDERS["hardware"][1].append(desktop_id)
        continue

    if "Settings" in categories:
        FOLDERS["settings"][1].append(desktop_id)
        continue

    if categories & {
        "System",
        "PackageManager",
        "Monitor",
    }:
        FOLDERS["administration"][1].append(desktop_id)
        continue

    if categories & {
        "AudioVideo",
        "Audio",
        "Video",
        "Graphics",
        "Photography",
        "Player",
        "Recorder",
    }:
        FOLDERS["multimedia"][1].append(desktop_id)
        continue

    if "Utility" in categories:
        FOLDERS["utilities"][1].append(desktop_id)
        continue

    FOLDERS["other"][1].append(desktop_id)


# No mostrar carpetas vacías.
FOLDERS = {
    key: value
    for key, value in FOLDERS.items()
    if value[1]
}

# El organizador es dueño de estas carpetas.
old_managed = [
    "nodalix-craft",
    "nodalix",
    "engineering",
    "internet",
    "office",
    "multimedia",
    "development",
    "games",
    "settings",
    "hardware",
    "administration",
    "utilities",
    "other",
]

try:
    current = ast.literal_eval(
        subprocess.check_output(
            [
                "gsettings",
                "get",
                "org.gnome.desktop.app-folders",
                "folder-children",
            ],
            text=True,
        ).strip()
    )
except Exception:
    current = []

# Conservar carpetas creadas manualmente que no gestiona Nodalix.
custom = [
    folder
    for folder in current
    if folder not in old_managed
]

children = list(FOLDERS.keys()) + custom

subprocess.run(
    [
        "gsettings",
        "set",
        "org.gnome.desktop.app-folders",
        "folder-children",
        repr(children),
    ],
    check=True,
)

for folder_id, (name, apps) in FOLDERS.items():
    apps = sorted(set(apps))

    schema = (
        "org.gnome.desktop.app-folders.folder:"
        f"/org/gnome/desktop/app-folders/folders/{folder_id}/"
    )

    subprocess.run(
        ["gsettings", "set", schema, "name", repr(name)],
        check=True,
    )

    subprocess.run(
        ["gsettings", "set", schema, "translate", "false"],
        check=True,
    )

    subprocess.run(
        ["gsettings", "set", schema, "categories", "[]"],
        check=True,
    )

    subprocess.run(
        ["gsettings", "set", schema, "apps", repr(apps)],
        check=True,
    )

    print(f"{name:16} {len(apps):3} apps")

# Limpiar la carpeta Nodalix antigua.
subprocess.run(
    [
        "dconf",
        "reset",
        "-f",
        "/org/gnome/desktop/app-folders/folders/nodalix/",
    ],
    check=False,
)

