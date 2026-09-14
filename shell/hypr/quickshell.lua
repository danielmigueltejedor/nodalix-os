-- quickshell.lua — Hyprland integration for this Quickshell desktop shell.
--
-- REQUIRES Hyprland's Lua configuration system (the `hl.*` API). The helper
-- scripts under scripts/hypr/ dispatch `hl.dsp.*` expressions, which only the
-- Lua config interpreter understands. If you use the classic hyprlang (.conf)
-- config, you'll need to port these binds/rules and the scripts yourself.
--
-- Usage: source this at the END of your own hyprland.lua so it overrides:
--     loadfile("/etc/xdg/quickshell/nodalix/hypr/quickshell.lua")()
--
-- Everything below is namespaced to what the shell needs. Comment out anything
-- you don't want, or copy the bits you like into your own config.

local home    = os.getenv("HOME")
local mainMod = "SUPER"

------------------------------------------------------------------------------
-- Required settings
------------------------------------------------------------------------------

hl.config({
    misc = {
        -- Let the relaunched lock client (quickshell) re-adopt the session lock
        -- instead of Hyprland's "lockscreen app died" failsafe. Needed because
        -- hot-reloading the shell tears down + recreates the WlSessionLock.
        allow_session_lock_restore = true,
    },
})

------------------------------------------------------------------------------
-- Autostart
------------------------------------------------------------------------------

-- Nodalix is started and supervised by nodalix-shell.service from the main
-- Hyprland configuration.  Do not start it again here: two shell instances
-- create two exclusive top bars and two wallpaper renderers per monitor.

------------------------------------------------------------------------------
-- Keybindings
------------------------------------------------------------------------------

-- Shell action keybinds (launcher, settings, lock, tools, scratchpad) are
-- configured in-app under Settings → Keybindings and written to
-- binds.generated.lua. That generated file is the ONLY source of truth for
-- BindingService-managed shortcuts. Do not hardcode the launcher shortcut or
-- other configurable actions here — a second bind toggles the launcher twice.
local state = (os.getenv("XDG_STATE_HOME") or (home .. "/.local/state")) .. "/nodalix"
local genBinds = state .. "/binds.generated.lua"
if io.open(genBinds) then loadfile(genBinds)() end

-- macOS-style capture shortcuts requested by Nodalix.
hl.bind("CTRL + SHIFT + 3", hl.dsp.exec_cmd("qs -c nodalix ipc call capture full"), { description = "Capture full screen" })
hl.bind("CTRL + SHIFT + 4", hl.dsp.exec_cmd("qs -c nodalix ipc call capture region"), { description = "Capture a region" })

-- Hyprland setting overrides (monitors / appearance / input) configured in-app
-- under Settings → Hyprland and written to hypr.generated.lua. Loaded LAST so it
-- layers over your own config (last-wins); absent/empty = no overrides.
local genHypr = state .. "/hypr.generated.lua"
if io.open(genHypr) then loadfile(genHypr)() end

-- Window organisation profile selected from the Nodalix control centre.
-- Loaded after general overrides so the explicitly selected profile wins.
local genLayout = state .. "/layout.generated.lua"
if io.open(genLayout) then loadfile(genLayout)() end

-- Per-monitor workspaces (optional): Super+N switches to workspace N on the
-- FOCUSED monitor; Super+Shift+N moves the active window there. Each monitor
-- id k owns workspaces k*10+1 .. k*10+10. Bound by physical keycode so it works
-- on any keyboard layout (code:10..19 = number row 1..0).
for i = 1, 10 do
    local code = 9 + i
    hl.bind(mainMod .. " + code:" .. code,         hl.dsp.exec_cmd("qs -c nodalix ipc call ws go " .. i .. " switch"))
    hl.bind(mainMod .. " + SHIFT + code:" .. code, hl.dsp.exec_cmd("qs -c nodalix ipc call ws go " .. i .. " move"))
end

------------------------------------------------------------------------------
-- Apps parked in named special workspaces (toggled from the Quickshell tray).
-- These are EXAMPLES — edit the class matches / workspace names to your apps,
-- and set the matching mapping in Settings → Tray. "silent" = launch without
-- stealing focus or switching the visible workspace.
------------------------------------------------------------------------------

-- hl.window_rule({ name = "ws-spotify",  match = { class = "(spotify|Spotify)" },        workspace = "special:spotify silent" })
-- hl.window_rule({ name = "ws-discord",  match = { class = "(vesktop|discord)" },        workspace = "special:discord silent" })
-- hl.window_rule({ name = "ws-music",    match = { class = "(YouTube Music Desktop App)" }, workspace = "special:music silent" })
