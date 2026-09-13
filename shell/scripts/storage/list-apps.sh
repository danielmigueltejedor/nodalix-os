#!/bin/sh

# Nodalix Storage - Installed applications
# Emit:
# backend|package-id|installed-size|display-name|icon-name

sizes_file=$(mktemp)
seen_file=$(mktemp)

cleanup() {
    rm -f "$sizes_file" "$seen_file"
}
trap cleanup EXIT HUP INT TERM

# Cache package sizes in a single call instead of invoking expac
# once for every installed package.
if command -v expac >/dev/null 2>&1; then
    expac -Q '%n|%m' > "$sizes_file" 2>/dev/null
fi

# Ask pacman for its file database once and keep only packages that
# actually install graphical .desktop applications.
pacman -Ql 2>/dev/null |
awk '$2 ~ /^\/usr\/share\/applications\/[^/]+\.desktop$/ {
    print $1 "|" $2
}' |
while IFS='|' read -r package_id desktop_file; do
    [ -n "$package_id" ] || continue
    [ -r "$desktop_file" ] || continue

    # Skip launchers explicitly hidden from application menus.
    if grep -Eq '^(NoDisplay|Hidden)=true[[:space:]]*$' "$desktop_file" 2>/dev/null; then
        continue
    fi

    # One entry per pacman package.
    # Some packages contain several .desktop launchers.
    if grep -Fxq "$package_id" "$seen_file" 2>/dev/null; then
        continue
    fi

    printf '%s\n' "$package_id" >> "$seen_file"

    package_size=$(awk -F'|' -v pkg="$package_id" '
        $1 == pkg {
            print $2
            exit
        }
    ' "$sizes_file")

    [ -n "$package_size" ] || package_size=0

    localized_name=$(awk -F= '
        /^\[Desktop Entry\]$/ {
            active=1
            next
        }

        /^\[/ && active {
            exit
        }

        active && $1 == "Name[es_ES]" {
            sub(/^[^=]*=/, "")
            print
            exit
        }
    ' "$desktop_file")

    if [ -z "$localized_name" ]; then
        localized_name=$(awk -F= '
            /^\[Desktop Entry\]$/ {
                active=1
                next
            }

            /^\[/ && active {
                exit
            }

            active && $1 == "Name[es]" {
                sub(/^[^=]*=/, "")
                print
                exit
            }
        ' "$desktop_file")
    fi

    default_name=$(awk -F= '
        /^\[Desktop Entry\]$/ {
            active=1
            next
        }

        /^\[/ && active {
            exit
        }

        active && $1 == "Name" {
            sub(/^[^=]*=/, "")
            print
            exit
        }
    ' "$desktop_file")

    icon_name=$(awk -F= '
        /^\[Desktop Entry\]$/ {
            active=1
            next
        }

        /^\[/ && active {
            exit
        }

        active && $1 == "Icon" {
            sub(/^[^=]*=/, "")
            print
            exit
        }
    ' "$desktop_file")

    display_name=$package_id

    if [ -n "$localized_name" ]; then
        display_name=$localized_name
    elif [ -n "$default_name" ]; then
        display_name=$default_name
    fi

    [ -n "$icon_name" ] || icon_name=application-x-executable

    # Protect the line-oriented protocol used by StorageService.qml.
    display_name=$(printf '%s' "$display_name" | tr '\n|' '  ')
    icon_name=$(printf '%s' "$icon_name" | tr '\n|' '  ')

    printf 'pacman|%s|%s|%s|%s\n' \
        "$package_id" \
        "$package_size" \
        "$display_name" \
        "$icon_name"
done

# Flatpak applications.
if command -v flatpak >/dev/null 2>&1; then
    flatpak list --app --columns=application,name,size 2>/dev/null |
    while IFS="$(printf '\t')" read -r app_id display_name app_size; do
        [ -n "$app_id" ] || continue

        desktop_file=""

        for candidate in \
            "$HOME/.local/share/flatpak/exports/share/applications/$app_id.desktop" \
            "/var/lib/flatpak/exports/share/applications/$app_id.desktop"
        do
            if [ -r "$candidate" ]; then
                desktop_file=$candidate
                break
            fi
        done

        icon_name=$app_id

        if [ -n "$desktop_file" ]; then
            desktop_icon=$(awk -F= '
                /^\[Desktop Entry\]$/ {
                    active=1
                    next
                }

                /^\[/ && active {
                    exit
                }

                active && $1 == "Icon" {
                    sub(/^[^=]*=/, "")
                    print
                    exit
                }
            ' "$desktop_file")

            [ -z "$desktop_icon" ] || icon_name=$desktop_icon
        fi

        display_name=$(printf '%s' "$display_name" | tr '\n|' '  ')
        icon_name=$(printf '%s' "$icon_name" | tr '\n|' '  ')

        printf 'flatpak|%s|%s|%s|%s\n' \
            "$app_id" \
            "$app_size" \
            "$display_name" \
            "$icon_name"
    done
fi
