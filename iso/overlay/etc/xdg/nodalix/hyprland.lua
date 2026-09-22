-- Portable defaults, with monitor modes selected by the compositor.
hl.monitor({ output = "", mode = "preferred", position = "auto", scale = "auto" })
hl.env("XDG_CURRENT_DESKTOP", "Hyprland")
hl.env("XCURSOR_SIZE", "24")
hl.config({
    general = { gaps_in = 6, gaps_out = 10, border_size = 2, layout = "dwindle" },
    decoration = { rounding = 18 },
    input = { kb_layout = os.getenv("XKB_DEFAULT_LAYOUT") or "us", touchpad = { natural_scroll = true } },
    misc = { disable_hyprland_logo = true, disable_splash_rendering = true, allow_session_lock_restore = true },
})
hl.bind("SUPER + RETURN", hl.dsp.exec_cmd("foot"), { description = "Terminal" })
hl.bind("SUPER + E", hl.dsp.exec_cmd("thunar"), { description = "Files" })
hl.bind("SUPER + Q", hl.dsp.window.close(), { description = "Close window" })
hl.on("hyprland.start", function()
    hl.exec_cmd("sh -c 'dbus-update-activation-environment --systemd WAYLAND_DISPLAY HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP; systemctl --user start nodalix-shell.service nodalix-phone-link.service'")
    hl.exec_cmd("/usr/lib/polkit-gnome/polkit-gnome-authentication-agent-1")
    if os.getenv("NODALIX_LIVE") == "1" then
        hl.exec_cmd("foot --title='Instalar Nodalix' sudo /usr/local/bin/nodalix-install")
    end
end)
loadfile("/etc/xdg/quickshell/nodalix/hypr/quickshell.lua")()
