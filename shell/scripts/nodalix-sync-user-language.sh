#!/bin/sh
set -eu

# LC_ALL overrides LANG and LANGUAGE completely. Some launchers inject a
# neutral C locale, which left GNOME applications in English even when Nodalix
# and the system were set to Spanish.
unset LC_ALL LC_CTYPE LC_MESSAGES

launch_chatgpt=false
if [ "${1:-}" = "--launch-chatgpt" ]; then
    launch_chatgpt=true
    shift
    settings_file="${XDG_STATE_HOME:-$HOME/.local/state}/nodalix/settings.json"
    language="$(jq -r '.general.language // "en"' "$settings_file" 2>/dev/null || printf 'en')"
else
    language="${1:-}"
    shift || true
fi

case "$language" in
    es)
        locale_name="es_ES.UTF-8"
        language_list="es_ES:es"
        browser_locale="es-ES"
        accepted_languages="es-ES,es,en-US,en"
        ;;
    en)
        locale_name="en_US.UTF-8"
        language_list="en_US:en"
        browser_locale="en-US"
        accepted_languages="en-US,en,es-ES,es"
        ;;
    *)
        echo "Unsupported language. Expected: es or en" >&2
        exit 2
        ;;
esac

# LANG is the proper fallback; avoiding LC_ALL still permits per-category
# overrides and prevents tools from being forced to an unrelated locale.
environment_dir="$HOME/.config/environment.d"
environment_file="$environment_dir/10-locale.conf"
mkdir -p "$environment_dir"
environment_tmp="$(mktemp "$environment_dir/.10-locale.conf.XXXXXX")"
printf 'LANG=%s\nLANGUAGE=%s\n' "$locale_name" "$language_list" > "$environment_tmp"
chmod 0644 "$environment_tmp"
mv -f "$environment_tmp" "$environment_file"

systemctl --user unset-environment LC_ALL LC_CTYPE LC_MESSAGES 2>/dev/null || true
systemctl --user set-environment LANG="$locale_name" LANGUAGE="$language_list" 2>/dev/null || true
dbus-update-activation-environment --systemd LANG="$locale_name" LANGUAGE="$language_list" 2>/dev/null || true

# New applications launched by Hyprland should adopt the change immediately;
# already-running applications still need to be reopened.
hyprctl keyword env "LC_ALL," >/dev/null 2>&1 || true
hyprctl keyword env "LANG,$locale_name" >/dev/null 2>&1 || true
hyprctl keyword env "LANGUAGE,$language_list" >/dev/null 2>&1 || true

# Chromium/Electron stores spell-check selection separately from LANG. Keep
# every unrelated ChatGPT/Codex preference intact.
chatgpt_preferences="$HOME/.config/Codex/Default/Preferences"
if [ -f "$chatgpt_preferences" ] && command -v jq >/dev/null 2>&1; then
    [ -e "$chatgpt_preferences.bak-nodalix-language" ] || cp -p "$chatgpt_preferences" "$chatgpt_preferences.bak-nodalix-language"
    preferences_tmp="$(mktemp "${chatgpt_preferences}.XXXXXX")"
    if jq --arg accepted "$accepted_languages" --arg dictionary "$browser_locale" \
        '.intl.accept_languages = $accepted
         | .intl.selected_languages = $accepted
         | .spellcheck.dictionaries = [$dictionary]
         | .spellcheck.dictionary = $dictionary' \
        "$chatgpt_preferences" > "$preferences_tmp"; then
        chmod --reference="$chatgpt_preferences" "$preferences_tmp"
        mv -f "$preferences_tmp" "$chatgpt_preferences"
    else
        rm -f "$preferences_tmp"
        exit 4
    fi
fi

# Zen/Firefox reads user.js at launch. Replace only preferences managed by
# Nodalix and preserve every unrelated user preference.
for zen_preferences in "$HOME"/.zen/*/prefs.js; do
    [ -f "$zen_preferences" ] || continue
    zen_profile="$(dirname "$zen_preferences")"
    zen_user="$zen_profile/user.js"
    [ ! -f "$zen_user" ] || [ -e "$zen_user.bak-nodalix-language" ] || cp -p "$zen_user" "$zen_user.bak-nodalix-language"
    zen_tmp="$(mktemp "$zen_profile/.user.js.XXXXXX")"
    if [ -f "$zen_user" ]; then
        sed -e '/^\/\/ Nodalix: system language and spell-check defaults\.$/d' \
            -e '/^\/\/ Nodalix: Spanish UI input and spell-check defaults\.$/d' \
            -e '/user_pref("intl\.accept_languages"/d' \
            -e '/user_pref("intl\.locale\.requested"/d' \
            -e '/user_pref("spellchecker\.dictionary"/d' \
            -e '/user_pref("layout\.spellcheckDefault"/d' \
            "$zen_user" > "$zen_tmp"
    fi
    printf '%s\n' \
        '// Nodalix: system language and spell-check defaults.' \
        "user_pref(\"intl.accept_languages\", \"$accepted_languages\");" \
        "user_pref(\"intl.locale.requested\", \"$browser_locale\");" \
        "user_pref(\"spellchecker.dictionary\", \"$browser_locale\");" \
        'user_pref("layout.spellcheckDefault", 2);' >> "$zen_tmp"
    chmod 0644 "$zen_tmp"
    mv -f "$zen_tmp" "$zen_user"
done

if [ "$launch_chatgpt" = true ]; then
    # Keep ChatGPT on the same native Wayland transport as Nautilus. Running
    # it through XWayland breaks cross-toolkit drag-and-drop and makes Electron
    # draw desktop-style minimise/maximise controls that do not belong in
    # Nodalix/Hyprland.
    exec env LANG="$locale_name" LANGUAGE="$language_list" \
        /usr/lib/chatgpt/ChatGPT \
        --ozone-platform=wayland \
        --disable-features=WaylandWindowDecorations \
        --lang="$browser_locale" "$@"
fi
