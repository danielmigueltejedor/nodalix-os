-- Dedicated, portable compositor for the Nodalix login screen.
-- Keep this hardware-agnostic: per-machine monitor overrides may live in
-- /etc/greetd/hyprland.lua without being replaced by package upgrades.
local keyboard = os.getenv("XKB_DEFAULT_LAYOUT")
if not keyboard then
    local vconsole = io.open("/etc/vconsole.conf", "r")
    if vconsole then
        local keymap = vconsole:read("*a"):match('KEYMAP="?([%w_-]+)') or "us"
        vconsole:close()
        local layouts = {
            es = "es", latam = "latam", us = "us", uk = "gb", de = "de",
            fr = "fr", it = "it", pt = "pt", br = "br",
        }
        keyboard = layouts[keymap] or layouts[keymap:match("^(%a+)")] or "us"
    end
end

hl.monitor({
    output = "",
    mode = "preferred",
    position = "auto",
    scale = "auto",
})

hl.env("GTK_USE_PORTAL", "0")
hl.env("GDK_DEBUG", "no-portals")
hl.env("XCURSOR_THEME", "Nodalix")
hl.env("XCURSOR_SIZE", "24")

hl.config({
    input = {
        kb_layout = keyboard or "us",
        numlock_by_default = true,
    },
    general = {
        gaps_in = 0,
        gaps_out = 0,
        border_size = 0,
    },
    decoration = {
        rounding = 24,
    },
    debug = {
        disable_logs = false,
        enable_stdout_logs = true,
        disable_time = false,
    },
    misc = {
        disable_hyprland_logo = true,
        disable_splash_rendering = true,
        disable_hyprland_guiutils_check = true,
        background_color = "rgb(080b10)",
    },
})

hl.on("hyprland.start", function()
    hl.exec_cmd([[/usr/bin/regreet --config /etc/greetd/regreet.toml --style /etc/greetd/regreet.css; hyprctl dispatch "hl.dsp.exit()"]])
end)
