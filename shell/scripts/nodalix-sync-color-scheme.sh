#!/bin/sh
set -eu

mode=${1:-dark}
case "$mode" in
    dark) scheme=prefer-dark ;;
    light) scheme=default ;;
    *) exit 2 ;;
esac

if command -v gsettings >/dev/null 2>&1 \
    && gsettings writable org.gnome.desktop.interface color-scheme 2>/dev/null | grep -qx true; then
    gsettings set org.gnome.desktop.interface color-scheme "$scheme"
fi

# GTK 3 applications that do not consult the portal still honour this key.
if command -v gsettings >/dev/null 2>&1 \
    && gsettings writable org.gnome.desktop.interface gtk-theme 2>/dev/null | grep -qx true; then
    current=$(gsettings get org.gnome.desktop.interface gtk-theme 2>/dev/null | tr -d "'")
    case "$mode:$current" in
        dark:adw-gtk3) gsettings set org.gnome.desktop.interface gtk-theme adw-gtk3-dark ;;
        light:adw-gtk3-dark) gsettings set org.gnome.desktop.interface gtk-theme adw-gtk3 ;;
    esac
fi
