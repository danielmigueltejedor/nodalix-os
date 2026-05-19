-- Learn how to configure Hyprland: https://wiki.hypr.land/Configuring/Start/

-- Nodalix Hyprland entrypoint.
-- User modules are loaded from ~/.config.
-- Legacy Omarchy defaults are vendored under ~/.local/share/nodalix/vendor/omarchy
-- until they are migrated module by module.

local home = os.getenv("HOME")
local nodalix_path = os.getenv("NODALIX_PATH") or (home .. "/.local/share/nodalix")
local omarchy_vendor_path = nodalix_path .. "/vendor/omarchy"

package.path = home
  .. "/.config/?.lua;"
  .. nodalix_path
  .. "/?.lua;"
  .. omarchy_vendor_path
  .. "/?.lua;"
  .. package.path

-- Temporary vendor compatibility layer.
require("default.hypr.nodalix")

-- User / Nodalix overrides.
require("hypr.monitors")
require("hypr.input")
require("hypr.bindings")
require("hypr.looknfeel")
require("hypr.autostart")

-- Temporary vendor toggles until migrated.
require("default.hypr.toggles")

-- Add any other personal Hyprland configuration below.
-- o.window("qemu", { workspace = "5" })
