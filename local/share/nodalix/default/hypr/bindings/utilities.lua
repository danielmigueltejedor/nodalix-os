-- nodalix-disabled: o.bind("SUPER + SPACE", "Launch apps", { nodalix = "walker" })
o.bind("SUPER + CTRL + E", "Emoji picker", { nodalix = "walker -m symbols" })
o.bind_menu("SUPER + CTRL + C", "Capture menu", "capture")
o.bind_menu("SUPER + CTRL + O", "Toggle menu", "toggle")
o.bind_menu("SUPER + CTRL + H", "Hardware menu", "hardware")
o.bind_menu("SUPER + ALT + SPACE", "Nodalix menu", nil)
o.bind_menu("SUPER + SHIFT + code:201", "Nodalix menu", nil)
o.bind_menu("SUPER + ESCAPE", "System menu", "system")
o.bind_menu("XF86PowerOff", "Power menu", "system", { locked = true })
o.bind("SUPER + K", "Show key bindings", "nodalix-menu-keybindings")
o.bind("SUPER + ALT + K", "Show Tmux key bindings", "nodalix-menu-tmux-keybindings")
o.bind("XF86Calculator", "Calculator", "gnome-calculator")

o.bind("SUPER + SHIFT + SPACE", "Toggle top bar", "nodalix-toggle-waybar")
o.bind("SUPER + SHIFT + CTRL + UP", "Move Waybar to top", "nodalix-style-waybar-position top")
o.bind("SUPER + SHIFT + CTRL + DOWN", "Move Waybar to bottom", "nodalix-style-waybar-position bottom")
o.bind("SUPER + SHIFT + CTRL + LEFT", "Move Waybar to left", "nodalix-style-waybar-position left")
o.bind("SUPER + SHIFT + CTRL + RIGHT", "Move Waybar to right", "nodalix-style-waybar-position right")
o.bind_menu("SUPER + CTRL + SPACE", "Background switcher", "background")
o.bind_menu("SUPER + SHIFT + CTRL + SPACE", "Theme menu", "theme")
o.bind("SUPER + BACKSPACE", "Toggle window transparency", "notify-send 'Nodalix' 'Transparency toggle pendiente de migrar'")
o.bind("SUPER + SHIFT + BACKSPACE", "Toggle window gaps", "notify-send 'Nodalix' 'Gaps toggle pendiente de migrar'")
o.bind("SUPER + CTRL + BACKSPACE", "Toggle single-window square aspect", "notify-send 'Nodalix' 'Aspect toggle pendiente de migrar'")

o.bind("SUPER + COMMA", "Dismiss last notification", "makoctl dismiss")
o.bind("SUPER + SHIFT + COMMA", "Dismiss all notifications", "makoctl dismiss --all")
o.bind("SUPER + CTRL + COMMA", "Toggle silencing notifications", "nodalix-toggle-notification-silencing")
o.bind("SUPER + ALT + COMMA", "Invoke last notification", "makoctl invoke")
o.bind("SUPER + SHIFT + ALT + COMMA", "Restore last notification", "makoctl restore")

o.bind_toggle("SUPER + CTRL + I", "Toggle locking on idle", "idle")
o.bind_toggle("SUPER + CTRL + N", "Toggle nightlight", "nightlight")
o.bind("SUPER + CTRL + Delete", "Toggle laptop display", "notify-send 'Nodalix' 'Internal monitor toggle pendiente de migrar' toggle")
o.bind("SUPER + CTRL + ALT + Delete", "Toggle laptop display mirroring", "notify-send 'Nodalix' 'Internal mirror pendiente de migrar' toggle")
o.bind("switch:on:Lid Switch", nil, "true && notify-send 'Nodalix' 'Internal monitor toggle pendiente de migrar' off", { locked = true })
o.bind("switch:off:Lid Switch", nil, "notify-send 'Nodalix' 'Internal monitor toggle pendiente de migrar' on", { locked = true })

o.bind("PRINT", "Screenshot", [[sh -c 'grim -g "$(slurp)" - | wl-copy']])
o.bind_menu("ALT + PRINT", "Screenrecording", "screenrecord")
o.bind("SUPER + PRINT", "Color picker", "pkill hyprpicker || hyprpicker -a")
o.bind("SUPER + CTRL + PRINT", "Extract text (OCR) from screenshot", "notify-send 'Nodalix' 'OCR capture pendiente de migrar'")

o.bind_menu("SUPER + CTRL + S", "Share", "share")

o.bind("SUPER + CTRL + PERIOD", "Transcode", "notify-send 'Nodalix' 'Transcode pendiente de migrar'")

o.bind_menu("SUPER + CTRL + R", "Set reminder", "reminder-set")
o.bind("SUPER + CTRL + ALT + R", "Show reminders", "notify-send 'Nodalix' 'Reminders pendiente de migrar' show")
o.bind("SUPER + SHIFT + CTRL + R", "Clear reminders", "notify-send 'Nodalix' 'Reminders pendiente de migrar' clear")

o.bind("SUPER + CTRL + ALT + T", "Show time", "date '+%H:%M' | xargs -I{} notify-send 'Hora' '{}'")
o.bind("SUPER + CTRL + ALT + B", "Show battery remaining", "notify-send 'Nodalix' 'Battery info no disponible en desktop'")
o.bind("SUPER + CTRL + ALT + W", "Show weather", "notify-send 'Nodalix' 'Weather pendiente de migrar'")

o.bind("SUPER + CTRL + A", "Audio controls", { nodalix = "audio" })
o.bind("SUPER + CTRL + B", "Bluetooth controls", { nodalix = "bluetooth" })
o.bind("SUPER + CTRL + W", "Wifi controls", { nodalix = "wifi" })
o.bind("SUPER + CTRL + T", "Activity", { tui = "btop" })

o.bind("SUPER + CTRL + X", "Toggle dictation", "voxtype record toggle")
o.bind("F9", "Start dictation (push-to-talk)", "voxtype record start")
o.bind("F9", "Stop dictation (push-to-talk)", "voxtype record stop", { release = true })

o.bind("SUPER + CTRL + Z", "Zoom in", function()
  local zoom = hl.get_config("cursor.zoom_factor") or 1
  hl.config({ cursor = { zoom_factor = zoom + 1 } })
end)

o.bind("SUPER + CTRL + ALT + Z", "Reset zoom", function()
  hl.config({ cursor = { zoom_factor = 1 } })
end)

o.bind("SUPER + CTRL + L", "Lock system", "nodalix-lock")
