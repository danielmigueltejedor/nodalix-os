-- Nodalix window borders
hl.config({
  general = {
    border_size = 3,
    ["col.active_border"] = "rgba(cba6f7ee)",
    ["col.inactive_border"] = "rgba(31324455)",
  },
})

-- Change the default Omarchy look'n'feel.

-- https://wiki.hypr.land/Configuring/Basics/Variables/#general
-- hl.config({
--   general = {
--     -- No gaps between windows or borders.
--     gaps_in = 0,
--     gaps_out = 0,
--     border_size = 3,
--
--     -- Change to niri-like side-scrolling layout.
--     layout = "scrolling",
--   },
-- })

-- https://wiki.hypr.land/Configuring/Basics/Variables/#decoration
-- hl.config({
--   decoration = {
--     -- Use round window corners.
--     rounding = 8,
--
--     -- Dim unfocused windows (0.0 = no dim, 1.0 = fully dimmed).
--     dim_inactive = true,
--     dim_strength = 0.15,
--   },
-- })

-- https://wiki.hypr.land/Configuring/Basics/Variables/#animations
-- hl.config({
--   animations = {
--     -- Disable all animations.
--     enabled = false,
--   },
-- })

-- https://wiki.hypr.land/Configuring/Basics/Variables/#layout
-- hl.config({
--   layout = {
--     -- Avoid overly wide single-window layouts on wide screens.
--     single_window_aspect_ratio = { 1, 1 },
--   },
-- })

-- https://wiki.hypr.land/Configuring/Layouts/Scrolling-Layout/
-- hl.config({
--   scrolling = {
--     -- See only one column per screen instead of two.
--     column_width = 0.97,
--   },
-- })
-- https://wiki.hypr.land/Configuring/Basics/Variables/#decoration
hl.config({
  decoration = {
    -- Use round window corners.
    rounding = 12,
    rounding_power = 2,

    -- Dim unfocused windows.
    dim_inactive = true,
    dim_strength = 0.15,

    shadow = {
      enabled = true,
      range = 12,
      render_power = 3,
    },
  },
})
-- Dwindle layout: automatic split direction.
hl.config({
  general = {
    layout = "dwindle",
  },

  dwindle = {
    preserve_split = false,
    smart_split = false,
    force_split = 0,
  },
})




-- Nodalia WPE Menu como popup flotante usando reglas nativas de Omarchy
o.window({ class = "^wpe-menu-window$", title = "^Nodalia WPE Menu$" }, {
  float = true,
  center = true,
  size = { 940, 560 },
})

-- Nodalix right-island popups: macOS-style position under Waybar island
local nodalix_right_popup_rules = {
  { class = "^nodalix-wifi-menu-window$", title = "^Nodalix Network$" },
  { class = "^nodalix-bt-menu-window$" },
  { class = "^nodalix-audio-menu-window$" },
  { class = "^nodalix-volume-menu-window$" },
}

for _, popup in ipairs(nodalix_right_popup_rules) do
  o.window(
    popup.title and { class = popup.class, title = popup.title } or { class = popup.class },
    {
      float = true,
      size = { 615, 562 },
      move = { "1913", "60" },
      animation = "popin"
    }
  )
end

-- Nodalix Files popup: macOS-style position under right Waybar island
o.window({ class = "^nodalix-files$", title = "^Nodalix Files$" }, {
  float = true,
  size = { 800, 562 },
  move = { "1728", "60" },
  animation = "popin"
})

-- Nodalix media popup: anchored under center Waybar island
o.window({ class = "^media-popup-window$" }, {
  float = true,
  size = { 560, 306 },
  move = { "((monitor_w-window_w)/2)", "60" },
  animation = "popin"
})


-- Nodalix Keybindings popup: quick keyboard shortcut reference
o.window({ class = "^nodalix-keybindings-menu-window$", title = "^Nodalix Keybindings$" }, {
  float = true,
  center = true,
  size = { 940, 620 },
  animation = "popin"
})

-- Nodalix Menu: System Settings style popup under left island
o.window({ class = "^nodalix-menu-window$" }, {
  tag = "-floating-window",
  float = true,
  size = { 1120, 740 },
  move = { "32", "60" },
  animation = "popin"
})

