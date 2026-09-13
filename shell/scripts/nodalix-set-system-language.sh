#!/bin/sh
set -eu

case "${1:-}" in
    es) locale_name="es_ES.UTF-8" ;;
    en) locale_name="en_US.UTF-8" ;;
    *) echo "Unsupported language. Expected: es or en" >&2; exit 2 ;;
esac

normalized="$(printf '%s' "$locale_name" | tr '[:upper:]' '[:lower:]' | sed 's/utf-8/utf8/')"
if ! locale -a | tr '[:upper:]' '[:lower:]' | grep -qx "$normalized"; then
    echo "Locale is not generated: $locale_name" >&2
    exit 3
fi

localectl set-locale LANG="$locale_name"
