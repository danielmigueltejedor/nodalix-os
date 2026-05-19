require("default.hypr.bindings.media")
require("default.hypr.bindings.clipboard")
require("default.hypr.bindings.tiling-v2")
require("default.hypr.bindings.utilities")

-- Application bindings without Nodalix's preinstalled web apps, TUIs, or desktop apps.
o.bind("SUPER + RETURN", "Terminal", { nodalix = "terminal" })
o.bind("SUPER + SHIFT + RETURN", "Browser", { nodalix = "browser" })
o.bind("SUPER + SHIFT + F", "File manager", { nodalix = "nautilus" })
o.bind("SUPER + ALT + SHIFT + F", "File manager (cwd)", { nodalix = "nautilus-cwd" })
o.bind("SUPER + SHIFT + B", "Browser", { nodalix = "browser" })
o.bind("SUPER + SHIFT + ALT + B", "Browser (private)", { nodalix = "browser --private" })
o.bind("SUPER + SHIFT + N", "Editor", { nodalix = "editor" })
