#!/usr/bin/env python3
"""Export the active Material palette to supported third-party themes."""

from __future__ import annotations

import json
import sys
from pathlib import Path


HOME = Path.home()
STATE = Path.home() / ".local/state/nodalix"
CONFIG_ROOT = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("/etc/xdg/quickshell/nodalix")
FLUENTY = HOME / ".local/share/Steam/millennium/themes/fluenty"


def read_json(path: Path) -> dict:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
        return value if isinstance(value, dict) else {}
    except (OSError, json.JSONDecodeError):
        return {}


def active_theme() -> dict:
    theme_id = str(read_json(STATE / "active.json").get("theme", "nodalix"))
    if theme_id == "wallpaper":
        path = STATE / "generated/wallpaper.json"
    elif theme_id.startswith("user:"):
        path = STATE / "custom" / f"{theme_id[5:]}.json"
    else:
        path = CONFIG_ROOT / "theme/presets" / f"{theme_id}.json"
    return read_json(path)


def palette() -> dict[str, str]:
    roles = active_theme().get("roles", {})
    defaults = {
        "primary": "#d1bcfd",
        "onPrimary": "#37265c",
        "primaryContainer": "#4e3d75",
        "onPrimaryContainer": "#eaddff",
        "secondary": "#ccc2dc",
        "onSecondary": "#332d41",
        "surfaceContainerHigh": "#2b292f",
        "onSurface": "#e7e0e8",
        "onSurfaceVariant": "#cbc4cf",
        "outline": "#948f99",
    }
    return {key: str(roles.get(key, fallback)) for key, fallback in defaults.items()}


def fluenty_css(c: dict[str, str]) -> str:
    return f"""/* Managed by Nodalix. Rebuilt whenever the Material palette changes. */
:root {{
    --nodalix-primary: {c['primary']};
    --nodalix-on-primary: {c['onPrimary']};
    --nodalix-primary-container: {c['primaryContainer']};
    --nodalix-on-primary-container: {c['onPrimaryContainer']};
    --nodalix-secondary: {c['secondary']};
    --nodalix-on-secondary: {c['onSecondary']};
    --nodalix-surface-high: {c['surfaceContainerHigh']};
    --nodalix-on-surface: {c['onSurface']};
    --nodalix-on-surface-variant: {c['onSurfaceVariant']};
    --nodalix-outline: {c['outline']};
    --nodalix-hover-surface: color-mix(in srgb, var(--nodalix-primary) 18%, var(--nodalix-surface-high));
    --nodalix-selected-surface: var(--nodalix-primary-container);
    --SystemAccentColor: var(--nodalix-primary) !important;
    --SystemAccentColorAccent: var(--nodalix-primary) !important;
    --SystemAccentColorLight1: color-mix(in srgb, var(--nodalix-primary) 86%, white) !important;
    --SystemAccentColorLight2: var(--nodalix-primary) !important;
    --SystemAccentColorLight3: var(--nodalix-primary-container) !important;
    --SystemAccentColorDark1: color-mix(in srgb, var(--nodalix-primary) 62%, var(--nodalix-surface-high)) !important;
    --SystemAccentColorDark2: var(--nodalix-primary-container) !important;
    --main-accent-default: var(--nodalix-primary) !important;
    --fill-color-accent-default: var(--nodalix-primary) !important;
    --fill-color-accent-secondary: var(--nodalix-primary-container) !important;
    --settings-sidebar-hover-bg: var(--nodalix-hover-surface) !important;
    --settings-sidebar-hover-not-select: var(--nodalix-hover-surface) !important;
    --accent-main: var(--nodalix-primary) !important;
    --accent: var(--nodalix-primary) !important;
    accent-color: var(--nodalix-primary);
}}

html body :is(.DialogCheckbox.Active, .DialogToggleField_Control.Active,
    [class*="DialogToggleField_Control"][class*="Active"],
    [class*="Toggle"][aria-checked="true"], [role="switch"][aria-checked="true"]) {{
    background-color: var(--nodalix-primary) !important;
    border-color: color-mix(in srgb, var(--nodalix-primary) 72%, white) !important;
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--nodalix-primary) 18%, transparent) !important;
}}

html body :is(.DialogCheckbox.Active, .DialogToggleField_Control.Active,
    [class*="DialogToggleField_Control"][class*="Active"],
    [class*="Toggle"][aria-checked="true"], [role="switch"][aria-checked="true"])::before,
html body :is(.DialogCheckbox.Active, .DialogToggleField_Control.Active,
    [class*="DialogToggleField_Control"][class*="Active"],
    [class*="Toggle"][aria-checked="true"], [role="switch"][aria-checked="true"]) :is(svg, path) {{
    background-color: var(--nodalix-on-primary) !important;
    color: var(--nodalix-on-primary) !important;
    fill: var(--nodalix-on-primary) !important;
}}

html body :is(.DialogButton.Primary, [class*="ProgressBar"] [class*="Bar"],
    [class*="DownloadProgress"] [class*="Progress"], [class*="playButton"]),
html body ._3cI5TXsFX3bvpR-7EBOtxq > ._2AzIX5kl9k6JnxLfR5H4kX,
html body ._3AjoLnMNKxYmNTGTJCLfgs._1_Bo2Ied5s2Od4YKYTOsau:not(._1aml4h4CSJbtrNbX4brUYs) {{
    background-color: var(--nodalix-primary) !important;
    color: var(--nodalix-on-primary) !important;
    border-color: color-mix(in srgb, var(--nodalix-primary) 70%, white) !important;
}}

html body .DialogCheckbox.Active::before {{
    color: var(--nodalix-on-primary) !important;
}}

html body :is(.DialogButton.Primary, ._3cI5TXsFX3bvpR-7EBOtxq > ._2AzIX5kl9k6JnxLfR5H4kX,
    ._3AjoLnMNKxYmNTGTJCLfgs._1_Bo2Ied5s2Od4YKYTOsau) :is(svg, path) {{
    color: var(--nodalix-on-primary) !important;
    fill: var(--nodalix-on-primary) !important;
}}

/* Selected navigation and menu items inherit Material containers, not black. */
html body :is(.activeTab, [role="tab"][aria-selected="true"],
    [role="option"][aria-selected="true"]) {{
    background: var(--nodalix-selected-surface) !important;
    color: var(--nodalix-on-primary-container) !important;
    border-color: color-mix(in srgb, var(--nodalix-primary) 55%, transparent) !important;
}}

html body :is(.contextMenuItem:hover, ._1n7Wloe5jZ6fSuvV18NNWI.contextMenuItem:hover,
    ._2oAiZidGyUxL-hfupFDQ2m:hover, .DialogButton:hover:not(.Primary),
    ._2-O4ZG0KrnSrzISHBKctFQ:hover, ._5wILZhsLODVwGfcJ0hKmJ:hover,
    .RtSv39ZoBOySnb8XQ5hJf:hover, ._161IKq84RwQO4abJSCqv7q:hover,
    ._3i62HEXIhsNTd5-Z4uL3K:hover,
    ._2jXHP0742MyApMUVUM8IFn._2uiDecKkKjAq7nimy3uLhG:hover) {{
    background: var(--nodalix-hover-surface) !important;
    color: var(--nodalix-on-surface) !important;
    border-color: color-mix(in srgb, var(--nodalix-primary) 38%, transparent) !important;
}}

html body ._1UBpAXP408Ez_L_mXhW5Q9 ._2-O4ZG0KrnSrzISHBKctFQ:hover,
html body ._1UBpAXP408Ez_L_mXhW5Q9 ._2-O4ZG0KrnSrzISHBKctFQ._3cMVyOc-9F9Jvp3uKF7_xj,
html body ._1UBpAXP408Ez_L_mXhW5Q9._2-O4ZG0KrnSrzISHBKctFQ:hover,
html body ._1UBpAXP408Ez_L_mXhW5Q9._2-O4ZG0KrnSrzISHBKctFQ._3cMVyOc-9F9Jvp3uKF7_xj,
html body ._1UBpAXP408Ez_L_mXhW5Q9._2-O4ZG0KrnSrzISHBKctFQ :hover {{
    background: var(--nodalix-hover-surface) !important;
}}

/* Focused search and text fields should remain dark, but gain a tinted surface. */
html body :is(._20QAC4WMXm8qFE8waUT5oo:focus-within,
    ._3x31AgESSlUqX3D4MTHv2m ._1JlC29Ic6L-QvL-39X_d-X:focus) {{
    background: color-mix(in srgb, var(--nodalix-primary) 10%, var(--nodalix-surface-high)) !important;
    border-bottom-color: var(--nodalix-primary) !important;
}}

html body .btn_grey_black:not(.btn_disabled):not(:disabled):hover,
html body .btn_grey_black:not(.btn_disabled):not(:disabled):hover > span {{
    background: var(--nodalix-hover-surface) !important;
    color: var(--nodalix-on-surface) !important;
    border-color: color-mix(in srgb, var(--nodalix-primary) 45%, transparent) !important;
}}
html body .btn_grey_black:is(.btn_active, .active),
html body .btn_grey_black:is(.btn_active, .active) > span {{
    background: var(--nodalix-selected-surface) !important;
    color: var(--nodalix-on-primary-container) !important;
}}
"""


def ensure_import(path: Path, import_line: str) -> None:
    if not path.is_file():
        return
    original = path.read_text(encoding="utf-8")
    lines = [line for line in original.splitlines() if line.strip() != import_line]
    # CSS imports must precede rules. Place ours after the theme's own imports
    # so equal-specificity declarations do not replace the Nodalix palette.
    position = 0
    while position < len(lines) and lines[position].lstrip().startswith("@import"):
        position += 1
    lines.insert(position, import_line)
    rendered = "\n".join(lines).rstrip() + "\n"
    if rendered != original:
        path.write_text(rendered, encoding="utf-8")


def main() -> int:
    styles = FLUENTY / "src/styles"
    if not styles.is_dir():
        return 0
    (styles / "nodalix-accent.css").write_text(fluenty_css(palette()), encoding="utf-8")
    ensure_import(styles / "variables.css", '@import url("./nodalix-accent.css");')
    ensure_import(FLUENTY / "libraryroot.custom.css", '@import url("./src/styles/nodalix-accent.css");')
    ensure_import(styles / "webkit/webkit.css", '@import url("../nodalix-accent.css");')
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
