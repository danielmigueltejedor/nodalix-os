#!/usr/bin/fish

set mount_point "$HOME/.local/share/nodalix/mounts/iCloud Drive"

# rclone notifica a systemd antes de que el montaje sea visible para GVfs.
# Esperar brevemente evita que Nautilus conserve el icono de advertencia.
for attempt in (seq 1 40)
    if /usr/bin/findmnt --mountpoint "$mount_point" >/dev/null 2>&1
        /usr/bin/gio set "$mount_point" metadata::custom-icon-name nodalix-icloud-drive-symbolic
        exit 0
    end

    /usr/bin/sleep 0.25
end

exit 1
