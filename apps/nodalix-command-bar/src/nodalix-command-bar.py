#!/usr/bin/env python3

import os
if "*" in os.environ.get("GDK_BACKEND", ""):
    os.environ.pop("GDK_BACKEND", None)
    os.unsetenv("GDK_BACKEND")

os.environ.setdefault("GDK_BACKEND", "wayland")

import gi
import json
import subprocess
import shlex
import shutil
import fcntl
import sys
from pathlib import Path

_SCRIPT_DIR = Path(__file__).resolve().parent
if str(_SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPT_DIR))

from ranking import rank_search_results

gi.require_version("Gtk", "3.0")
from gi.repository import Gtk, Gdk, GLib, GdkPixbuf


APP_TITLE = "Nodalix Command Bar"
NODALIX_LOGO_CANDIDATES = [
    Path("/etc/nodalix/brand/nodalix-logo-symbol.svg"),
    Path("/usr/share/nodalix/brand/nodalix-logo-symbol.svg"),
    Path("/home/dani/Projects/nodalix-os/assets/brand/nodalix-logo-symbol.svg"),
]

LOCK_FILE_HANDLE = None


def acquire_single_instance_lock():
    global LOCK_FILE_HANDLE

    lock_path = Path("/tmp/nodalix-command-bar.lock")
    LOCK_FILE_HANDLE = lock_path.open("w")

    try:
        fcntl.flock(LOCK_FILE_HANDLE, fcntl.LOCK_EX | fcntl.LOCK_NB)
        LOCK_FILE_HANDLE.write(str(os.getpid()))
        LOCK_FILE_HANDLE.flush()
        return True
    except BlockingIOError:
        return False


def nodalix_logo_path():
    return next((path for path in NODALIX_LOGO_CANDIDATES if path.is_file()), None)


def make_brand_widget(text="Nodalix", size=28):
    box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
    box.set_valign(Gtk.Align.CENTER)
    logo_path = nodalix_logo_path()
    if logo_path:
        try:
            pix = GdkPixbuf.Pixbuf.new_from_file_at_scale(str(logo_path), size, size, True)
            image = Gtk.Image.new_from_pixbuf(pix)
            image.set_name("brand_logo")
            box.pack_start(image, False, False, 0)
        except Exception:
            pass
    label = Gtk.Label(label=text, xalign=0)
    label.set_name("brand")
    box.pack_start(label, True, True, 0)
    return box


def find_repo_root():
    for parent in Path(__file__).resolve().parents:
        if (parent / "config/nodalix/actions.json").exists():
            return parent
    return Path.home() / "Projects/nodalix-os"


REPO_ROOT = find_repo_root()
APP_DIR = Path(__file__).resolve().parents[1]
USER_ACTIONS_FILE = Path.home() / ".config/nodalix/actions.json"
REPO_ACTIONS_FILE = REPO_ROOT / "config/nodalix/actions.json"
SHOW_DISABLED_ACTIONS_FILE = Path.home() / ".config/nodalix/show-disabled-actions"
DEBUG = os.environ.get("NODALIX_COMMAND_BAR_DEBUG", "").lower() in {"1", "true", "yes", "on"}


def debug(message):
    if DEBUG:
        print(f"[nodalix-command-bar] {message}", file=sys.stderr)



def load_actions_from_json():
    candidates = [
        USER_ACTIONS_FILE,
        REPO_ACTIONS_FILE,
    ]

    for file in candidates:
        if not file.exists():
            continue

        try:
            data = json.loads(file.read_text())
        except Exception:
            continue

        if not isinstance(data, list):
            continue

        actions = []
        show_disabled = show_disabled_actions()

        for raw in data:
            if not isinstance(raw, dict):
                continue

            title = str(raw.get("title") or "").strip()
            command = str(raw.get("command") or "").strip()

            if not title or not command:
                continue

            missing = missing_requirements(raw)
            is_available = len(missing) == 0
            hide_if_missing = bool(raw.get("hide_if_missing", False))

            if hide_if_missing and missing and not show_disabled:
                continue

            category = str(raw.get("category") or "General").strip()
            subtitle = str(raw.get("subtitle") or "Acción").strip()

            if missing:
                subtitle = "No disponible · falta: " + ", ".join(missing)
            elif category and category.lower() not in subtitle.lower():
                subtitle = category + " · " + subtitle

            aliases = raw.get("aliases") or []
            if not isinstance(aliases, list):
                aliases = []

            search_text = " ".join([
                str(raw.get("search") or title),
                category,
                " ".join(str(a) for a in aliases),
                "disabled missing no disponible" if missing else "",
            ])

            actions.append({
                "kind": "action",
                "icon": str(raw.get("icon") or "󰒓"),
                "title": title,
                "subtitle": subtitle,
                "search": search_text.lower(),
                "command": command,
                "category": category,
                "requires": raw.get("requires") or [],
                "missing_requires": missing,
                "available": is_available,
                "disabled": not is_available,
                "danger": bool(raw.get("danger", False)),
                "confirm_message": str(raw.get("confirm_message") or ""),
            })

        if actions:
            return actions

    return None


def sh(args):
    try:
        return subprocess.check_output(args, text=True, stderr=subprocess.DEVNULL).strip()
    except Exception:
        return ""


def run_cmd(cmd):
    subprocess.Popen(cmd, shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def open_in_default_browser(url):
    if command_exists("nodalix-launch-browser"):
        run_cmd(f"nodalix-launch-browser {shlex.quote(url)}")
        return
    if command_exists("zen-browser"):
        run_cmd(f"zen-browser --new-window {shlex.quote(url)}")
        return
    nodalix = shutil.which("nodalix")
    if nodalix:
        run_cmd(f"{shlex.quote(nodalix)} browser {shlex.quote(url)}")
        return
    if command_exists("xdg-open"):
        run_cmd(f"xdg-open {shlex.quote(url)}")


def command_exists(binary):
    return shutil.which(str(binary)) is not None


def show_disabled_actions():
    return SHOW_DISABLED_ACTIONS_FILE.exists()


def missing_requirements(raw):
    requires = raw.get("requires") or []

    if not isinstance(requires, list):
        return []

    return [str(binary) for binary in requires if not command_exists(binary)]


def action_requirements_available(raw):
    return len(missing_requirements(raw)) == 0


def confirm_dangerous_action(item):
    if not item.get("danger"):
        return True

    dialog = Gtk.MessageDialog(
        transient_for=None,
        flags=0,
        message_type=Gtk.MessageType.WARNING,
        buttons=Gtk.ButtonsType.OK_CANCEL,
        text=item.get("title", "Confirmar acción"),
    )

    dialog.format_secondary_text(
        item.get("confirm_message") or "Esta acción puede afectar al sistema."
    )

    response = dialog.run()
    dialog.destroy()

    return response == Gtk.ResponseType.OK


def setup_css():
    repo_root = APP_DIR.parent.parent
    fonts_css = (repo_root / "assets/styles/nodalix-fonts.css").read_bytes()
    app_css = (APP_DIR / "data/nodalix-command-bar.css").read_bytes()
    css = fonts_css + app_css

    provider = Gtk.CssProvider()
    provider.load_from_data(css)
    screen = Gdk.Screen.get_default()
    if screen is not None:
        Gtk.StyleContext.add_provider_for_screen(
            screen,
            provider,
            Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION,
        )



def clean_exec_command(exec_cmd):
    for token in ["%f", "%F", "%u", "%U", "%i", "%c", "%k"]:
        exec_cmd = exec_cmd.replace(token, "")

    return exec_cmd.strip()


def exec_basename(exec_cmd):
    try:
        parts = shlex.split(exec_cmd)
    except Exception:
        parts = exec_cmd.split()

    if not parts:
        return ""

    return Path(parts[0]).name.lower()




APP_BLOCKLIST_BASES = {
    "assistant6",
    "avahi-discover",
    "bssh",
    "bvnc",
    "cliamp",
    "cmake-gui",
    "designer6",
    "electron37",
    "imv",
    "imv-dir",
    "java",
    "jconsole",
    "jshell",
    "linguist6",
    "lstopo",
    "pinentry-qt",
    "pinentry-qt5",
    "qdbusviewer6",
    "qv4l2",
    "qvidcap",
    "rofi-theme-selector",
    "uuctl",
    "xgps",
    "xgpsspeed",
    "yad-icon-browser",
    "yad-settings",
}

APP_EQUIV_BASES = {
    "baobab": "disk-usage",
    "bitwarden-desktop": "passwords",
    "ghostty": "terminal",
    "gnome-calendar": "calendar",
    "gnome-clocks": "clock",
    "gnome-connections": "connections",
    "gnome-disks": "disks",
    "gnome-logs": "logs",
    "loupe": "photos",
    "meld": "diff",
    "mpv": "media-player",
    "openrgb": "rgb",
    "papers": "documents",
    "pavucontrol": "sound",
    "qalculate-gtk": "calculator",
    "simple-scan": "scanner",
    "steam": "steam",
    "steam-rx9070xt": "steam",
    "system-config-printer": "printers",
    "thunderbird": "mail",
}

BROWSER_COMMON_ALIASES = ("browser", "navegador", "web", "internet")
BROWSER_ALIASES = {
    "zen": ("zen", "zen-browser", "zen browser", *BROWSER_COMMON_ALIASES),
    "zen-browser": ("zen", "zen-browser", "zen browser", *BROWSER_COMMON_ALIASES),
    "zen-bin": ("zen", "zen-browser", "zen browser", *BROWSER_COMMON_ALIASES),
    "firefox": ("firefox", *BROWSER_COMMON_ALIASES),
    "chromium": ("chromium", "chrome", *BROWSER_COMMON_ALIASES),
    "chrome": ("chrome", "chromium", *BROWSER_COMMON_ALIASES),
    "google-chrome": ("chrome", "google chrome", "chromium", *BROWSER_COMMON_ALIASES),
    "google-chrome-stable": ("chrome", "google chrome", "chromium", *BROWSER_COMMON_ALIASES),
}
ZEN_FALLBACK_BINARIES = ("zen-browser", "zen", "zen-bin")

PREFERRED_NODALIX_PREFIXES = (
    "nodalix-",
    "com.nodalia.",
)

def desktop_file_id(path):
    try:
        return Path(path).name.lower()
    except Exception:
        return ""

def app_exec_signature(exec_cmd):
    try:
        parts = shlex.split(str(exec_cmd))
    except Exception:
        parts = str(exec_cmd).split()

    if not parts:
        return ""

    # Soporta wrappers tipo: env GSK_RENDERER=gl app
    if parts[0] == "env":
        parts = [p for p in parts[1:] if "=" not in p]
        if not parts:
            return ""

    base = Path(parts[0]).name.lower()
    group = APP_EQUIV_BASES.get(base)

    if group:
        return "app-group:" + group

    # Para webapps Chromium, conservar argumentos: cada webapp es una app distinta.
    if base in {"chromium", "chrome", "google-chrome", "zen", "zen-browser", "zen-bin"}:
        return "exec:" + " ".join(parts).lower()

    # Para apps normales, deduplicar por ejecutable + argumentos ya limpios.
    return "exec:" + " ".join(parts).lower()

def app_priority(item):
    path = str(item.get("desktop_file", ""))
    file_id = desktop_file_id(path)
    title = str(item.get("title", "")).lower()

    score = 0

    if "/.local/share/applications/" in path:
        score += 30

    if file_id.startswith(PREFERRED_NODALIX_PREFIXES):
        score += 100

    if "nodalix" in file_id:
        score += 80

    if "/usr/share/applications/" in path:
        score += 10

    if app_command_resolvable(item):
        score += 50

    # NoDisplay/Hidden siguen indexados para Command Bar, pero no deben ganar
    # automáticamente a un lanzador visible equivalente si ambos existen.
    if item.get("nodisplay"):
        score -= 5
    if item.get("hidden"):
        score -= 10

    # Preferir el lanzador Steam optimizado.
    if "steam-rx9070xt" in str(item.get("command", "")):
        score += 120

    # Evitar nombres genéricos cuando hay alias Nodalix más claros.
    if title in {
        "document viewer",
        "image viewer",
        "disk usage analyzer",
        "volume control",
        "print settings",
        "text editor",
        "logs",
        "disks",
        "calendar",
        "clocks",
        "connections",
    }:
        score -= 20

    return score


def app_command_resolvable(item):
    command = str(item.get("command", "")).strip()

    if not command:
        return False

    try:
        parts = shlex.split(command)
    except Exception:
        parts = command.split()

    if not parts:
        return False

    if parts[0] == "env":
        parts = [part for part in parts[1:] if "=" not in part]
        if not parts:
            return False

    if parts[0] in {"sh", "bash", "zsh", "flatpak", "gtk-launch"}:
        return command_exists(parts[0])

    executable = Path(parts[0])

    if executable.is_absolute() or "/" in parts[0]:
        return executable.exists()

    return command_exists(parts[0])


def desktop_bool(value):
    return str(value or "").strip().lower() == "true"


def browser_aliases(name, exec_cmd, categories, keywords, comment, generic_name, wmclass):
    hay = " ".join(
        [
            str(name or ""),
            str(exec_cmd or ""),
            str(categories or ""),
            str(keywords or ""),
            str(comment or ""),
            str(generic_name or ""),
            str(wmclass or ""),
        ]
    ).lower()

    exec_base = exec_basename(clean_exec_command(str(exec_cmd or "")))
    aliases = set()
    is_webapp = "--app=" in hay or " --app " in hay

    if not is_webapp and (
        "webbrowser" in hay
        or "web browser" in hay
        or "navegador" in hay
        or exec_base in BROWSER_ALIASES
    ):
        aliases.update(BROWSER_COMMON_ALIASES)

    if not is_webapp and "firefox" in hay:
        aliases.update(BROWSER_ALIASES["firefox"])

    if not is_webapp and "chromium" in hay:
        aliases.update(BROWSER_ALIASES["chromium"])

    if not is_webapp and ("google chrome" in hay or "google-chrome" in hay or "chrome" in hay):
        aliases.update(BROWSER_ALIASES["google-chrome"])

    if not is_webapp and ("zen browser" in hay or "zen-browser" in hay):
        aliases.update(BROWSER_ALIASES["zen"])

    return sorted(aliases)

def should_hide_app(item):
    base = str(item.get("exec_base", "")).lower()
    title = str(item.get("title", "")).lower()
    path = str(item.get("desktop_file", "")).lower()

    if base in APP_BLOCKLIST_BASES:
        return True

    if "vnc server browser" in title:
        return True

    if "openjdk java" in title:
        return True

    if "avahi" in title:
        return True

    if "qt v4l2" in title:
        return True

    if "pinentry" in title:
        return True

    if path.endswith("mimeinfo.cache"):
        return True

    return False


def focus_existing_app(item):
    base = item.get("exec_base", "").lower()

    # Nunca hacer focus automático en Zen ni webapps.
    # Queremos permitir múltiples ventanas/instancias.
    if any(x in base for x in ("zen", "zen-browser", "zen-bin")):
        return False

    raw = sh(["hyprctl", "clients", "-j"])

    if not raw:
        return False

    try:
        clients = json.loads(raw)
    except Exception:
        return False

    title = item.get("title", "").lower()
    wmclass = item.get("wmclass", "").lower()

    strong_tokens = [x for x in [wmclass, base] if x]
    soft_tokens = [title] if title else []

    for c in clients:
        address = c.get("address")

        if not address:
            continue

        values = [
            str(c.get("class") or "").lower(),
            str(c.get("initialClass") or "").lower(),
            str(c.get("title") or "").lower(),
            str(c.get("initialTitle") or "").lower(),
        ]

        for token in strong_tokens:
            for value in values[:2]:
                if token == value or token in value or value in token:
                    run_cmd(f"hyprctl dispatch focuswindow address:{address}")
                    return True

        for token in soft_tokens:
            if len(token) >= 4:
                for value in values[2:]:
                    if token in value:
                        run_cmd(f"hyprctl dispatch focuswindow address:{address}")
                        return True

    return False

    raw = sh(["hyprctl", "clients", "-j"])

    if not raw:
        return False

    try:
        clients = json.loads(raw)
    except Exception:
        return False

    title = item.get("title", "").lower()
    wmclass = item.get("wmclass", "").lower()
    base = item.get("exec_base", "").lower()

    strong_tokens = [x for x in [wmclass, base] if x]
    soft_tokens = [title] if title else []

    for c in clients:
        address = c.get("address")
        if not address:
            continue

        values = [
            str(c.get("class") or "").lower(),
            str(c.get("initialClass") or "").lower(),
            str(c.get("title") or "").lower(),
            str(c.get("initialTitle") or "").lower(),
        ]

        # Match fuerte: clase de ventana / binario.
        for token in strong_tokens:
            for value in values[:2]:
                if token == value or token in value or value in token:
                    run_cmd(f"hyprctl dispatch focuswindow address:{address}")
                    return True

        # Match suave: nombre de app dentro del título.
        # Útil para apps sin StartupWMClass correcto.
        for token in soft_tokens:
            if len(token) >= 4:
                for value in values[2:]:
                    if token in value:
                        run_cmd(f"hyprctl dispatch focuswindow address:{address}")
                        return True

    return False


def make_icon_widget(item):
    kind = item.get("kind", "")
    icon_text = item.get("icon") or "󰀻"
    icon_name = item.get("icon_name") or ""

    # Las apps usan iconos reales del sistema.
    if kind == "app":
        image = Gtk.Image()
        image.set_name("item_icon")

        try:
            if icon_name and Path(icon_name).exists():
                pix = GdkPixbuf.Pixbuf.new_from_file_at_scale(icon_name, 24, 24, True)
                image.set_from_pixbuf(pix)
            elif icon_name:
                image.set_from_icon_name(icon_name, Gtk.IconSize.DIALOG)
                image.set_pixel_size(24)
            else:
                image.set_from_icon_name("application-x-executable", Gtk.IconSize.DIALOG)
                image.set_pixel_size(24)
        except Exception:
            image.set_from_icon_name("application-x-executable", Gtk.IconSize.DIALOG)
            image.set_pixel_size(24)

        return image

    # Acciones, ventanas y workspaces usan Nerd Font como texto.
    label = Gtk.Label(label=icon_text)
    label.set_name("item_icon")
    label.set_width_chars(2)
    label.set_xalign(0.5)
    return label



def nodalix_app_category(name, exec_cmd, categories, keywords, comment):
    hay = " ".join([
        str(name or ""),
        str(exec_cmd or ""),
        str(categories or ""),
        str(keywords or ""),
        str(comment or ""),
    ]).lower()

    cats = {c.strip().lower() for c in str(categories or "").split(";") if c.strip()}

    # Apps base Nodalix con filtros propios.
    if "nodalix files" in hay or "filemanager" in cats or "inode/directory" in hay:
        return "Archivos"

    if "mousam" in hay or "weather" in hay or "forecast" in hay or "tiempo" in hay or "clima" in hay:
        return "Tiempo"

    if "missioncenter" in hay or "mission center" in hay or "system monitor" in hay or "ksystemlog" in hay or "gnome logs" in hay or "logs" in hay or "journal" in hay:
        return "Sistema"

    if "calendar" in hay or "morgen" in hay or "calendario" in hay or "agenda" in hay:
        return "Productividad"

    if "papers" in hay or "evince" in hay or "document viewer" in hay or "pdf" in hay or "documents" in hay or "documentos" in hay:
        return "Documentos"

    if "loupe" in hay or "drawing" in hay or "image" in hay or "photo" in hay or "graphics" in cats or "2dgraphics" in cats:
        return "Imágenes"

    if "development" in cats:
        return "Dev"

    if "game" in cats:
        return "Gaming"

    if "network" in cats:
        return "Red"

    if "audiovideo" in cats or "audio" in cats or "video" in cats:
        return "Audio"

    if "settings" in cats or "system" in cats or "monitor" in cats:
        return "Sistema"

    if "office" in cats:
        return "Productividad"

    if "utility" in cats:
        return "Utilidades"

    return "Apps"


def parse_desktop_file(path):
    name = None
    exec_cmd = None
    try_exec = None
    icon_name = None
    wmclass = None
    comment = None
    generic_name = None
    categories = ""
    keywords = ""
    hidden = False
    nodisplay = False
    terminal = False
    app_type = "Application"
    in_desktop_entry = False

    try:
        for raw in Path(path).read_text(errors="ignore").splitlines():
            line = raw.strip()

            if line == "[Desktop Entry]":
                in_desktop_entry = True
                continue

            if line.startswith("[") and line != "[Desktop Entry]":
                in_desktop_entry = False

            if not in_desktop_entry or not line or line.startswith("#"):
                continue

            if line.startswith("Name=") and name is None:
                name = line.split("=", 1)[1].strip()
            elif line.startswith("Type="):
                app_type = line.split("=", 1)[1].strip()
            elif line.startswith("GenericName=") and generic_name is None:
                generic_name = line.split("=", 1)[1].strip()
            elif line.startswith("Comment=") and comment is None:
                comment = line.split("=", 1)[1].strip()
            elif line.startswith("Exec=") and exec_cmd is None:
                exec_cmd = line.split("=", 1)[1].strip()
            elif line.startswith("TryExec=") and try_exec is None:
                try_exec = line.split("=", 1)[1].strip()
            elif line.startswith("Icon=") and icon_name is None:
                icon_name = line.split("=", 1)[1].strip()
            elif line.startswith("StartupWMClass=") and wmclass is None:
                wmclass = line.split("=", 1)[1].strip()
            elif line.startswith("Categories="):
                categories = line.split("=", 1)[1].strip()
            elif line.startswith("Keywords="):
                keywords = line.split("=", 1)[1].strip()
            elif line.startswith("NoDisplay="):
                nodisplay = desktop_bool(line.split("=", 1)[1])
            elif line.startswith("Hidden="):
                hidden = desktop_bool(line.split("=", 1)[1])
            elif line.startswith("Terminal="):
                terminal = desktop_bool(line.split("=", 1)[1])
    except Exception:
        return None

    if app_type and app_type != "Application":
        debug(f"skip non-application desktop file: {path}")
        return None

    if not name or not exec_cmd:
        debug(f"skip incomplete desktop file: {path}")
        return None

    exec_cmd = clean_exec_command(exec_cmd)
    base = exec_basename(exec_cmd)

    if terminal and exec_cmd:
        exec_cmd = f"alacritty -e {exec_cmd}"

    category = nodalix_app_category(name, exec_cmd, categories, keywords, comment)
    aliases = browser_aliases(name, exec_cmd, categories, keywords, comment, generic_name, wmclass)

    subtitle_parts = [category]
    if generic_name:
        subtitle_parts.append(generic_name)
    elif comment:
        subtitle_parts.append(comment)
    else:
        subtitle_parts.append(exec_cmd)

    return {
        "kind": "app",
        "icon": "󰣆",
        "icon_name": icon_name or "application-x-executable",
        "title": name,
        "subtitle": " · ".join(subtitle_parts),
        "search": f"{name} {generic_name or ''} {comment or ''} {exec_cmd} {try_exec or ''} {wmclass or ''} {base} {categories} {keywords} {' '.join(aliases)}".lower(),
        "search_aliases": aliases,
        "command": exec_cmd,
        "category": category,
        "wmclass": wmclass or "",
        "exec_base": base,
        "desktop_file": str(path),
        "exec_signature": app_exec_signature(exec_cmd),
        "nodisplay": nodisplay,
        "hidden": hidden,
        "try_exec": try_exec or "",
    }


def make_binary_app(binary, *, title=None, aliases=()):
    resolved = shutil.which(binary)

    if not resolved:
        return None

    app_title = title or binary
    search_aliases = sorted({str(a).lower() for a in aliases if str(a).strip()})

    return {
        "kind": "app",
        "icon": "󰖟",
        "icon_name": "zen-browser" if "zen" in binary else "application-x-executable",
        "title": app_title,
        "subtitle": f"Apps · {resolved}",
        "search": f"{app_title} {binary} {resolved} {' '.join(search_aliases)}".lower(),
        "search_aliases": search_aliases,
        "command": binary,
        "category": "Apps",
        "wmclass": "zen" if "zen" in binary else "",
        "exec_base": Path(binary).name.lower(),
        "desktop_file": "",
        "exec_signature": f"binary:{Path(binary).name.lower()}",
        "search_priority": 0,
    }


def add_zen_binary_fallback(apps):
    has_zen_desktop = any(
        item.get("kind") == "app"
        and (
            "zen" in str(item.get("title", "")).lower()
            or str(item.get("exec_base", "")).lower() in {"zen", "zen-browser", "zen-bin"}
            or "zen-browser" in str(item.get("search", "")).lower()
        )
        for item in apps
    )

    if has_zen_desktop:
        return

    for binary in ZEN_FALLBACK_BINARIES:
        fallback = make_binary_app(
            binary,
            title="Zen Browser",
            aliases=BROWSER_ALIASES.get(binary, BROWSER_ALIASES["zen"]),
        )
        if fallback:
            debug(f"added Zen Browser binary fallback from PATH: {binary}")
            apps.append(fallback)
            return



def load_apps():
    dirs = [
        Path.home() / ".local/share/applications",
        Path.home() / ".local/share/flatpak/exports/share/applications",
        Path("/var/lib/flatpak/exports/share/applications"),
        Path("/usr/local/share/applications"),
        Path("/usr/share/applications"),
    ]

    by_signature = {}
    by_title = {}

    for d in dirs:
        if not d.exists():
            continue

        for file in d.glob("*.desktop"):
            item = parse_desktop_file(file)

            if not item:
                continue

            if should_hide_app(item):
                debug(f"skip blocklisted desktop app: {item.get('title')} ({file})")
                continue

            item["search_priority"] = app_priority(item)
            signature = item.get("exec_signature") or item["title"].lower()
            title_key = item["title"].strip().lower()

            current = by_signature.get(signature)

            if current is None or app_priority(item) > app_priority(current):
                by_signature[signature] = item

            current_title = by_title.get(title_key)
            chosen = by_signature.get(signature, item)

            if current_title is None or app_priority(chosen) > app_priority(current_title):
                by_title[title_key] = chosen

    # Segunda pasada: dedupe final por título, manteniendo el mejor candidato.
    final = {}

    for item in by_signature.values():
        title_key = item["title"].strip().lower()
        current = final.get(title_key)

        if current is None or app_priority(item) > app_priority(current):
            final[title_key] = item

    apps = list(final.values())
    add_zen_binary_fallback(apps)
    apps.sort(key=lambda x: x["title"].lower())
    return apps



def ellipsize(text, max_len=72):
    text = str(text or "").strip()

    if len(text) <= max_len:
        return text

    return text[: max_len - 1].rstrip() + "…"


def compact_window_title(title):
    return ellipsize(title, 58)


def compact_subtitle(text):
    return ellipsize(text, 86)


def load_windows():
    raw = sh(["hyprctl", "clients", "-j"])

    if not raw:
        return []

    try:
        clients = json.loads(raw)
    except Exception:
        return []

    items = []

    for c in clients:
        raw_title = c.get("title") or c.get("class") or "Window"
        klass = c.get("class") or "unknown"
        address = c.get("address")
        workspace = c.get("workspace", {}).get("name", "?")

        if not address:
            continue

        title = compact_window_title(raw_title)
        subtitle = compact_subtitle(f"Ventana · {klass} · workspace {workspace}")

        items.append({
            "kind": "window",
            "icon": "󰖲",
            "title": title,
            "subtitle": subtitle,
            "search": f"{raw_title} {klass} {workspace}".lower(),
            "command": f"hyprctl dispatch focuswindow address:{address}",
        })

    return items



def load_workspaces():
    raw = sh(["hyprctl", "workspaces", "-j"])

    if not raw:
        return []

    try:
        workspaces = json.loads(raw)
    except Exception:
        return []

    items = []

    for w in sorted(workspaces, key=lambda x: x.get("id", 0)):
        wid = w.get("id")
        name = w.get("name", str(wid))
        windows = w.get("windows", 0)

        items.append({
            "kind": "workspace",
            "icon": "󰈹",
            "title": f"Workspace {name}",
            "subtitle": f"{windows} ventanas",
            "search": f"workspace {name} {wid}".lower(),
            "command": f"hyprctl dispatch workspace {wid}",
        })

    return items


def load_actions():
    external_actions = load_actions_from_json()

    if external_actions:
        return external_actions

        return [
            {
                "kind": "action",
                "icon": "󰒓",
                "title": "Abrir ajustes de Nodalix",
                "subtitle": "Sistema · Nodalix Settings",
                "search": "settings ajustes nodalix sistema configuracion",
                "command": "nodalix-settings",
            },
            {
                "kind": "action",
                "icon": "󰒡",
                "title": "Nodalix Doctor",
                "subtitle": "Diagnóstico del sistema",
                "search": "doctor diagnostico sistema logs errores salud",
                "command": "alacritty -e nodalix-doctor",
            },
            {
                "kind": "action",
                "icon": "󰜉",
                "title": "Recargar Hyprland",
                "subtitle": "Sistema · hyprctl reload",
                "search": "reload recargar hyprland compositor config",
                "command": "hyprctl reload && notify-send \"Nodalix\" \"Hyprland recargado\"",
            },
            {
                "kind": "action",
                "icon": "󰜉",
                "title": "Recargar Waybar",
                "subtitle": "Sistema · reiniciar barra superior",
                "search": "reload recargar waybar barra topbar panel",
                "command": "pkill waybar; waybar >/tmp/waybar.log 2>&1 &",
            },
            {
                "kind": "action",
                "icon": "󰌾",
                "title": "Bloquear sesión",
                "subtitle": "Seguridad · Bloqueo Nodalix",
                "search": "bloquear lock pantalla seguridad hyprlock",
                "command": "nodalix-lock",
            },
            {
                "kind": "action",
                "icon": "󰤄",
                "title": "Suspender equipo",
                "subtitle": "Energía · systemctl suspend",
                "search": "suspender sleep dormir energia suspension",
                "command": "systemctl suspend",
            },
            {
                "kind": "action",
                "icon": "󰜉",
                "title": "Reiniciar equipo",
                "subtitle": "Energía · systemctl reboot",
                "search": "reiniciar reboot energia sistema",
                "command": "systemctl reboot",
            },
            {
                "kind": "action",
                "icon": "󰐥",
                "title": "Apagar equipo",
                "subtitle": "Energía · systemctl poweroff",
                "search": "apagar shutdown poweroff energia sistema",
                "command": "systemctl poweroff",
            },
            {
                "kind": "action",
                "icon": "󰍹",
                "title": "Abrir WPE Menu",
                "subtitle": "Wallpapers · Wallpaper Engine",
                "search": "wallpaper wpe fondos workshop steam",
                "command": "wpe-menu",
            },
            {
                "kind": "action",
                "icon": "󰉋",
                "title": "Abrir archivos",
                "subtitle": "Archivos · Nautilus",
                "search": "archivos files nautilus explorador carpetas",
                "command": "nautilus",
            },
            {
                "kind": "action",
                "icon": "󰆍",
                "title": "Abrir terminal",
                "subtitle": "Terminal · Alacritty",
                "search": "terminal consola shell alacritty fish",
                "command": "alacritty",
            },
            {
                "kind": "action",
                "icon": "󰈹",
                "title": "Abrir navegador",
                "subtitle": "Web · navegador por defecto",
                "search": "browser navegador web internet",
                "command": "xdg-open https://www.google.com",
            },
            {
                "kind": "action",
                "icon": "󰕾",
                "title": "Abrir control de audio",
                "subtitle": "Audio · Pavucontrol",
                "search": "audio sonido volumen pavucontrol pipewire",
                "command": "pavucontrol",
            },
            {
                "kind": "action",
                "icon": "󰝟",
                "title": "Play / Pause",
                "subtitle": "Media · playerctl",
                "search": "media musica play pause reproducir pausar",
                "command": "playerctl play-pause",
            },
            {
                "kind": "action",
                "icon": "󰒮",
                "title": "Canción anterior",
                "subtitle": "Media · playerctl previous",
                "search": "media musica anterior previous",
                "command": "playerctl previous",
            },
            {
                "kind": "action",
                "icon": "󰒭",
                "title": "Canción siguiente",
                "subtitle": "Media · playerctl next",
                "search": "media musica siguiente next",
                "command": "playerctl next",
            },
            {
                "kind": "action",
                "icon": "󰝝",
                "title": "Silenciar audio",
                "subtitle": "Audio · toggle mute",
                "search": "audio mute silenciar sonido volumen",
                "command": "pamixer -t",
            },
            {
                "kind": "action",
                "icon": "󰕾",
                "title": "Subir volumen",
                "subtitle": "Audio · +5%",
                "search": "audio subir volumen mas",
                "command": "pamixer -i 5",
            },
            {
                "kind": "action",
                "icon": "󰖀",
                "title": "Bajar volumen",
                "subtitle": "Audio · -5%",
                "search": "audio bajar volumen menos",
                "command": "pamixer -d 5",
            },
            {
                "kind": "action",
                "icon": "󰄀",
                "title": "Capturar zona",
                "subtitle": "Captura · grim + slurp + satty",
                "search": "screenshot captura zona recorte satty grim slurp",
                "command": "grim -g \"$(slurp)\" - | satty --filename -",
            },
            {
                "kind": "action",
                "icon": "󰹑",
                "title": "Capturar pantalla completa",
                "subtitle": "Captura · guardar en Imágenes",
                "search": "screenshot captura pantalla completa grim",
                "command": "mkdir -p ~/Pictures/Screenshots && grim ~/Pictures/Screenshots/screenshot-$(date +%Y%m%d-%H%M%S).png",
            },
            {
                "kind": "action",
                "icon": "󰅌",
                "title": "Historial del portapapeles",
                "subtitle": "Clipboard · cliphist",
                "search": "clipboard portapapeles historial copiar cliphist",
                "command": "cliphist list | rofi -dmenu | cliphist decode | wl-copy",
            },
            {
                "kind": "action",
                "icon": "󰖩",
                "title": "Abrir redes Wi-Fi",
                "subtitle": "Red · Impala si está instalado",
                "search": "wifi red redes network internet impala",
                "command": "alacritty -e impala",
            },
            {
                "kind": "action",
                "icon": "󰂯",
                "title": "Abrir Bluetooth",
                "subtitle": "Bluetooth · Bluetui si está instalado",
                "search": "bluetooth dispositivos auriculares mando bluetui",
                "command": "alacritty -e bluetui",
            },
            {
                "kind": "action",
                "icon": "󰖟",
                "title": "Mostrar IP local",
                "subtitle": "Red · notificación",
                "search": "ip local red network direccion",
                "command": "notify-send \"IP local\" \"$(hostname -I | xargs)\"",
            },
            {
                "kind": "action",
                "icon": "󰖲",
                "title": "Cerrar ventana activa",
                "subtitle": "Ventanas · hyprctl killactive",
                "search": "cerrar ventana activa close window killactive",
                "command": "hyprctl dispatch killactive",
            },
            {
                "kind": "action",
                "icon": "󰹑",
                "title": "Pantalla completa",
                "subtitle": "Ventanas · toggle fullscreen",
                "search": "fullscreen pantalla completa ventana",
                "command": "hyprctl dispatch fullscreen 1",
            },
            {
                "kind": "action",
                "icon": "󰉈",
                "title": "Ventana flotante",
                "subtitle": "Ventanas · toggle floating",
                "search": "floating flotante ventana toggle",
                "command": "hyprctl dispatch togglefloating",
            },
            {
                "kind": "action",
                "icon": "󰏘",
                "title": "Centrar ventana activa",
                "subtitle": "Ventanas · centerwindow",
                "search": "centrar ventana center window",
                "command": "hyprctl dispatch centerwindow",
            },
            {
                "kind": "action",
                "icon": "󰓓",
                "title": "Abrir Steam",
                "subtitle": "Gaming · Steam",
                "search": "steam gaming juegos biblioteca",
                "command": "steam",
            },
            {
                "kind": "action",
                "icon": "󰓅",
                "title": "Modo Gaming",
                "subtitle": "Gaming · Próximamente",
                "search": "gaming juegos gamemode mangohud rendimiento",
                "command": "notify-send \"Nodalix Gaming\" \"Modo Gaming todavía no está implementado\"",
            },
            {
                "kind": "action",
                "icon": "󰨞",
                "title": "Abrir Zed",
                "subtitle": "Dev · Zed",
                "search": "dev zed zed editor programar",
                "command": "zed",
            },
            {
                "kind": "action",
                "icon": "󰊤",
                "title": "Abrir GitHub",
                "subtitle": "Dev · GitHub web",
                "search": "github git repos repositorios dev",
                "command": "xdg-open https://github.com",
            },
            {
                "kind": "action",
                "icon": "󰚩",
                "title": "IA / OpenCode",
                "subtitle": "IA · Próximamente",
                "search": "ia ai opencode asistente voz pantalla",
                "command": "notify-send \"Nodalix AI\" \"IA todavía no está implementada\"",
            },
            {
                "kind": "action",
                "icon": "󰟐",
                "title": "Abrir Home Assistant",
                "subtitle": "Domótica · URL local por configurar",
                "search": "home assistant domotica hass casa nodalia",
                "command": "xdg-open http://homeassistant.local:8123",
            },
            {
                "kind": "action",
                "icon": "󰌾",
                "title": "Centro de privacidad",
                "subtitle": "Privacidad · Próximamente",
                "search": "privacidad privacy micro camara pantalla permisos",
                "command": "notify-send \"Nodalix Privacy\" \"Centro de privacidad todavía no está implementado\"",
            },
        ]




def command_bar_filters(items):
    base = ["Todo", "Apps", "Acciones", "Ventanas", "Workspaces", "Sistema", "Web"]
    preferred = [
        "Nodalix",
        "Sistema",
        "Archivos",
        "Tiempo",
        "Productividad",
        "Documentos",
        "Imágenes",
        "Utilidades",
        "Energía",
        "Audio",
        "Red",
        "Capturas",
        "Wallpapers",
        "Gaming",
        "Dev",
        "IA",
        "Home Assistant",
        "Privacidad",
    ]

    categories = []
    seen = set()

    for item in items:
        category = str(item.get("category") or "").strip()

        if not category or category in seen:
            continue

        seen.add(category)
        categories.append(category)

    ordered = []

    for category in preferred:
        if category in seen:
            ordered.append(category)

    for category in sorted(categories):
        if category not in ordered:
            ordered.append(category)

    return base + [category for category in ordered if category not in base]


class CommandBar(Gtk.Window):
    def __init__(self):
        super().__init__(title=APP_TITLE)

        self.set_default_size(860, 590)
        self.set_resizable(False)
        self.set_decorated(False)
        self.set_app_paintable(True)
        self.set_position(Gtk.WindowPosition.CENTER)
        self.connect("destroy", Gtk.main_quit)
        self.connect("key-press-event", self.on_key)

        screen = self.get_screen()
        visual = screen.get_rgba_visual()

        if visual is not None and screen.is_composited():
            self.set_visual(visual)

        self.active_filter = "Todo"
        self.all_items = []
        self.filtered_items = []

        self.load_items()

        root = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=14)
        root.set_name("root")
        self.add(root)

        header = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        header.set_name("header")
        root.pack_start(header, False, False, 0)

        brand = make_brand_widget("Nodalix", 28)
        header.pack_start(brand, True, True, 0)

        header_hint = Gtk.Label(label="Command Bar", xalign=1)
        header_hint.set_name("header_hint")
        header.pack_end(header_hint, False, False, 0)

        self.search = Gtk.SearchEntry()
        self.search.set_name("search")
        self.search.set_placeholder_text("Buscar apps, ajustes, ventanas o la web…")
        self.search.connect("search-changed", lambda _e: self.refresh())
        self.search.connect("activate", lambda _e: self.activate_selected())
        root.pack_start(self.search, False, False, 0)

        chips_scroller = Gtk.ScrolledWindow()
        chips_scroller.set_policy(Gtk.PolicyType.AUTOMATIC, Gtk.PolicyType.NEVER)
        chips_scroller.set_size_request(-1, 58)
        chips_scroller.set_overlay_scrolling(False)
        root.pack_start(chips_scroller, False, False, 0)

        chips = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=7)
        chips.set_name("chips")
        chips.set_margin_bottom(12)
        chips_scroller.add(chips)

        self.chip_buttons = {}

        for label in command_bar_filters(self.all_items):
            btn = Gtk.Button(label=label)
            btn.set_name("chip_active" if label == "Todo" else "chip")
            btn.connect("clicked", lambda _b, l=label: self.set_filter(l))
            chips.pack_start(btn, False, False, 0)
            self.chip_buttons[label] = btn

        scroller = Gtk.ScrolledWindow()
        scroller.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        root.pack_start(scroller, True, True, 0)

        results = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        scroller.add(results)

        self.listbox = Gtk.ListBox()
        self.listbox.set_selection_mode(Gtk.SelectionMode.SINGLE)
        self.listbox.connect("row-activated", lambda _lb, _row: self.activate_selected())
        results.pack_start(self.listbox, True, True, 0)

        self.empty_state = Gtk.Label(
            label="No hay resultados. Prueba con otra búsqueda.",
            xalign=0.5,
            yalign=0.5,
        )
        self.empty_state.set_name("empty_state")
        self.empty_state.set_no_show_all(True)
        results.pack_start(self.empty_state, True, True, 0)

        footer = Gtk.Label(xalign=0)
        footer.set_name("footer")
        footer.set_text("Enter abrir · Esc cerrar · ↑↓ navegar · búsqueda web con el navegador predeterminado")
        root.pack_end(footer, False, False, 0)

        self.refresh()
        GLib.idle_add(self.focus_search)

    def focus_search(self):
        self.present()
        self.search.grab_focus()

        from gi.repository import GLib

        def late_focus():
            try:
                self.present()
                self.search.grab_focus()
                self.search.grab_focus_without_selecting()
            except Exception:
                pass
            return False

        GLib.timeout_add(120, late_focus)
        return False

    def on_key(self, _w, event):
        if event.keyval == Gdk.KEY_Escape:
            Gtk.main_quit()
            return True

        if event.keyval == Gdk.KEY_Down:
            self.move_selection(1)
            return True

        if event.keyval == Gdk.KEY_Up:
            self.move_selection(-1)
            return True

        return False

    def move_selection(self, delta):
        rows = self.listbox.get_children()

        if not rows:
            return

        selected = self.listbox.get_selected_row()

        if selected:
            index = selected.get_index() + delta
        else:
            index = 0

        index = max(0, min(len(rows) - 1, index))
        self.listbox.select_row(rows[index])
        rows[index].grab_focus()

    def set_filter(self, label):
        self.active_filter = label

        for name, btn in self.chip_buttons.items():
            btn.set_name("chip_active" if name == label else "chip")

        self.refresh()

    def load_items(self):
        self.all_items = []
        self.all_items.extend(load_actions())
        self.all_items.extend(load_apps())
        self.all_items.extend(load_windows())
        self.all_items.extend(load_workspaces())

    def item_matches_filter(self, item):
        f = self.active_filter
        kind = item.get("kind")
        category = str(item.get("category") or "").strip()

        if f == "Todo":
            return True

        if f == "Apps":
            return kind == "app"
        if f == "Acciones":
            return kind == "action"
        if f == "Ventanas":
            return kind == "window"
        if f == "Workspaces":
            return kind == "workspace"
        if f == "Sistema":
            return category in ("Sistema", "Nodalix", "Energía")
        if f == "Web":
            return item.get("kind") in ("web_search", "web_open") or category in (
                "Web",
                "Home Assistant",
            ) or "xdg-open http" in item.get("command", "")

        # Categorías dinámicas desde actions.json y categorías de apps .desktop.
        return category == f

    def refresh(self):
        query = self.search.get_text().strip().lower()

        for row in self.listbox.get_children():
            self.listbox.remove(row)

        self.filtered_items = rank_search_results(
            query,
            self.all_items,
            self.item_matches_filter,
        )

        for item in self.filtered_items:
            self.listbox.add(self.make_row(item))

        self.listbox.show_all()

        rows = self.listbox.get_children()

        if rows:
            self.listbox.show()
            self.empty_state.hide()
            self.listbox.select_row(rows[0])
        else:
            self.listbox.hide()
            self.empty_state.show()


    def make_row(self, item):
        row = Gtk.ListBoxRow()
        row.item = item

        box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=14)
        row.add(box)

        icon_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        icon_box.set_name("icon_box")
        icon_box.set_halign(Gtk.Align.CENTER)
        icon_box.set_valign(Gtk.Align.CENTER)
        box.pack_start(icon_box, False, False, 0)

        icon = make_icon_widget(item)
        icon_box.pack_start(icon, True, True, 0)

        text = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
        box.pack_start(text, True, True, 0)

        title = Gtk.Label(xalign=0)
        title.set_name("item_title")
        title.set_ellipsize(3)
        title.set_max_width_chars(64)
        title.set_text(item.get("title", "Item"))
        text.pack_start(title, False, False, 0)

        subtitle = Gtk.Label(xalign=0)
        subtitle.set_name("item_subtitle")
        subtitle.set_ellipsize(3)
        subtitle.set_max_width_chars(88)
        subtitle.set_text(item.get("subtitle", ""))
        text.pack_start(subtitle, False, False, 0)

        category = item.get("category")

        if not category:
            kind = item.get("kind", "item")
            category = {
                "app": "App",
                "window": "Ventana",
                "workspace": "Workspace",
                "action": "Acción",
                "web_search": "Web",
                "web_open": "Web",
            }.get(kind, "Item")

        if item.get("disabled"):
            badge_text = "No disponible"
            badge_name = "badge_disabled"
        elif item.get("danger"):
            badge_text = "Peligro"
            badge_name = "badge_danger"
        else:
            badge_text = str(category)
            badge_name = "badge"

        badge = Gtk.Label(label=badge_text)
        badge.set_name(badge_name)
        badge.set_valign(Gtk.Align.CENTER)
        box.pack_end(badge, False, False, 0)

        return row


    def activate_selected(self):
        row = self.listbox.get_selected_row()

        if not row:
            query = self.search.get_text().strip()
            ranked = rank_search_results(
                query,
                self.all_items,
                self.item_matches_filter,
                max_results=1,
            )
            if ranked and ranked[0].get("kind") in ("web_search", "web_open"):
                open_in_default_browser(ranked[0].get("url", ""))
                Gtk.main_quit()
            return

        item = row.item

        if item.get("kind") in ("web_search", "web_open"):
            open_in_default_browser(item.get("url", ""))
            Gtk.main_quit()
            return

        cmd = item.get("command")

        if not cmd:
            return

        if item.get("kind") == "action":
            if item.get("disabled"):
                missing = ", ".join(item.get("missing_requires") or [])
                run_cmd('notify-send "Nodalix" "Acción no disponible. Falta: ' + missing + '"')
                return

            if not confirm_dangerous_action(item):
                return

        if item.get("kind") == "app":
            base = item.get("exec_base", "").lower()

            if base not in ("zen", "zen-browser", "zen-bin"):
                if focus_existing_app(item):
                    Gtk.main_quit()
                    return

        run_cmd(cmd)
        Gtk.main_quit()


if __name__ == "__main__":
    if not acquire_single_instance_lock():
        raise SystemExit(0)

    setup_css()
    win = CommandBar()
    win.show_all()
    win.present()

    from gi.repository import GLib

    def force_focus():
        try:
            win.present()
            win.search.grab_focus()
        except Exception:
            pass
        return False

    GLib.timeout_add(80, force_focus)
    GLib.timeout_add(180, force_focus)

    Gtk.main()
