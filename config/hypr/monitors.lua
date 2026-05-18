-- See https://wiki.hypr.land/Configuring/Basics/Monitors/
-- List current monitors and resolutions possible:
-- hyprctl monitors all

-- =============================================================================
-- Monitor setup - Daniel
-- =============================================================================

local omarchy_monitor_scale = 1

-- Monitor izquierdo/secundario: MSI MAG241C
-- Disponible: 1920x1080@143.85Hz
hl.monitor({
  output = "DP-2",
  mode = "1920x1080@143.85",
  position = "0x0",
  scale = omarchy_monitor_scale,
})

-- Monitor derecho/principal: MSI G273CQ
-- Disponible: 2560x1440@165Hz
hl.monitor({
  output = "DP-3",
  mode = "2560x1440@165",
  position = "1920x0",
  scale = omarchy_monitor_scale,
})
