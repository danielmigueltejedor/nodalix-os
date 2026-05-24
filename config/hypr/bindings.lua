-- Application bindings.
o.bind("SUPER + RETURN", "Terminal", { launch = "nodalix terminal" })
o.bind("SUPER + ALT + RETURN", "Tmux", { launch = "ghostty -e tmux new -A -s main", focus = "com.mitchellh.ghostty" })
o.bind("SUPER + SHIFT + RETURN", "Browser", { launch = "zen-browser --new-window" })
o.bind("SUPER + SHIFT + F", "File manager", { launch = "nodalix files", focus = "dde-file-manager" })
o.bind("SUPER + SHIFT + B", "Browser", { launch = "zen-browser --new-window about:home" })
o.bind("SUPER + ALT + SHIFT + F", "File manager (cwd)", { launch = "nodalix files-cwd", focus = "dde-file-manager" })

o.bind("SUPER + SHIFT + ALT + B", "Browser (private)", { launch = "zen-browser --private-window" })

-- Music: Apple Music web app instead of Spotify.
o.bind("SUPER + SHIFT + M", "Apple Music", { launch = "gtk-launch apple-music", focus = "Apple Music" })

o.bind("SUPER + SHIFT + ALT + M", "Music TUI", { tui = "cliamp", focus = true })
o.bind("SUPER + SHIFT + N", "Editor", { launch = "nodalix editor", focus = "dev.zed.Zed" })
o.bind("SUPER + SHIFT + D", "Docker", { tui = "lazydocker" })
o.bind("SUPER + SHIFT + G", "Signal", { launch = "signal-desktop", focus = "^signal$" })
o.bind("SUPER + SHIFT + O", "Obsidian", { launch = "obsidian", focus = "^obsidian$" })
o.bind("SUPER + SHIFT + W", "Typora", { launch = "typora --enable-wayland-ime" })
o.bind("SUPER + SHIFT + SLASH", "Passwords", { launch = "1password" })

-- Web app bindings.
o.bind("SUPER + SHIFT + A", "ChatGPT", { webapp = "https://chatgpt.com" })
o.bind("SUPER + SHIFT + ALT + A", "Grok", { webapp = "https://grok.com" })
o.bind("SUPER + SHIFT + C", "Calendar", { webapp = "https://app.hey.com/calendar/weeks/" })

-- Email: Thunderbird instead of HEY webmail.
o.bind("SUPER + SHIFT + E", "Thunderbird", { launch = "thunderbird", focus = "^thunderbird$" })

o.bind("SUPER + SHIFT + Y", "YouTube", { webapp = "https://youtube.com/" })
o.bind("SUPER + SHIFT + ALT + G", "WhatsApp", { webapp = "https://web.whatsapp.com/", focus = true })
o.bind("SUPER + SHIFT + CTRL + G", "Google Messages", { webapp = "https://messages.google.com/web/conversations", focus = true })
o.bind("SUPER + SHIFT + P", "Google Photos", { webapp = "https://photos.google.com/", focus = true })
o.bind("SUPER + SHIFT + S", "Google Maps", { webapp = "https://maps.google.com/", focus = true })
o.bind("SUPER + SHIFT + X", "X", { webapp = "https://x.com/" })
o.bind("SUPER + SHIFT + ALT + X", "X Post", { webapp = "https://x.com/compose/post" })

-- Add extra bindings below.
-- o.bind("SUPER + SHIFT + R", "SSH", "alacritty -e ssh your-server")
o.bind("CTRL + ALT + 2", "Type @", "wtype @")

-- Overwrite existing bindings with hl.unbind() first if needed.

-- Logitech MX Keys examples:
-- o.bind("SUPER + H", nil, "voxtype record toggle")

-- Nodalix

-- Nodalix Command Bar
hl.unbind("SUPER + SPACE")

-- Nodalix Command Bar
o.bind("SUPER + SPACE", "Nodalix Command Bar", { launch = "nodalix-command-bar-toggle" })
