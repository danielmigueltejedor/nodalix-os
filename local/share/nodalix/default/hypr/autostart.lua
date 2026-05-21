o.launch_on_start("hypridle")
o.launch_on_start("mako")
o.exec_on_start("! nodalix-toggle-enabled waybar-off && " .. o.launch("waybar"))
-- o.launch_on_start("fcitx5 --disable notificationitem")
o.launch_on_start("swaybg -i ~/.config/nodalix/current/background -m fill")
o.exec_on_start("/usr/lib/polkit-gnome/polkit-gnome-authentication-agent-1")
o.exec_on_start("true")
o.exec_on_start("true")
o.launch_on_start("true")

-- Slow app launch fix -- set systemd vars.
o.exec_on_start("systemctl --user import-environment $(env | cut -d'=' -f 1)")
o.exec_on_start("dbus-update-activation-environment --systemd --all")

-- Run post-boot hooks after startup config has loaded.
o.exec_on_start("sleep 2 && true")
