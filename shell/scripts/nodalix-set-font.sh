#!/bin/sh
set -eu

font_family=${1:-}

case "$font_family" in
    "JetBrainsMono Nerd Font"|"JetBrainsMono Nerd Font Propo"|"Noto Sans"|"Adwaita Sans"|"Noto Sans Mono"|"Geist") ;;
    *)
        printf '%s\n' "Unsupported font: $font_family" >&2
        exit 2
        ;;
esac

if ! fc-list -f '%{family}\n' | tr ',' '\n' | sed 's/^ *//; s/ *$//' | grep -Fx "$font_family" >/dev/null; then
    printf '%s\n' "Font is not installed: $font_family" >&2
    exit 3
fi

font_size=11
mono_family="JetBrainsMono Nerd Font"
case "$font_family" in
    *Mono*) mono_family=$font_family ;;
esac

if command -v gsettings >/dev/null 2>&1; then
    gsettings set org.gnome.desktop.interface font-name "$font_family $font_size" 2>/dev/null || true
    gsettings set org.gnome.desktop.interface document-font-name "$font_family $font_size" 2>/dev/null || true
    gsettings set org.gnome.desktop.interface monospace-font-name "$mono_family $font_size" 2>/dev/null || true
fi

fontconfig_dir=${XDG_CONFIG_HOME:-"$HOME/.config"}/fontconfig/conf.d
fontconfig_file=$fontconfig_dir/60-nodalix-system-font.conf
mkdir -p "$fontconfig_dir"
temp_file=$(mktemp "$fontconfig_dir/.nodalix-font.XXXXXX")
trap 'rm -f "$temp_file"' EXIT HUP INT TERM

{
    printf '%s\n' '<?xml version="1.0"?>'
    printf '%s\n' '<!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">'
    printf '%s\n' '<fontconfig>'
    printf '%s\n' '  <alias>'
    printf '%s\n' '    <family>system-ui</family>'
    printf '    <prefer><family>%s</family><family>JetBrainsMono Nerd Font</family><family>Fluent Emoji Color</family><family>Noto Color Emoji</family></prefer>\n' "$font_family"
    printf '%s\n' '  </alias>'
    printf '%s\n' '  <alias>'
    printf '%s\n' '    <family>sans-serif</family>'
    printf '    <prefer><family>%s</family><family>JetBrainsMono Nerd Font</family><family>Fluent Emoji Color</family><family>Noto Color Emoji</family></prefer>\n' "$font_family"
    printf '%s\n' '  </alias>'
    printf '%s\n' '  <alias>'
    printf '%s\n' '    <family>monospace</family>'
    printf '    <prefer><family>%s</family><family>Fluent Emoji Color</family><family>Noto Color Emoji</family></prefer>\n' "$mono_family"
    printf '%s\n' '  </alias>'
    printf '%s\n' '</fontconfig>'
} > "$temp_file"

mv "$temp_file" "$fontconfig_file"
trap - EXIT HUP INT TERM
fc-cache -f >/dev/null 2>&1 || true
