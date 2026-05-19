#!/bin/bash
# Pin walker to omarchy repo to prevent CachyOS version conflict
if ! grep -q "^IgnorePkg.*walker" /etc/pacman.conf 2>/dev/null; then
  if grep -q "^IgnorePkg" /etc/pacman.conf; then
    sudo sed -i 's/^IgnorePkg = \(.*\)/IgnorePkg = \1 walker/' /etc/pacman.conf
  else
    sudo sed -i '/^\[options\]/a IgnorePkg = walker' /etc/pacman.conf
  fi
fi


# Ensure Walker service is started automatically on boot
mkdir -p ~/.config/autostart/
cp $OMARCHY_PATH/default/walker/walker.desktop ~/.config/autostart/

# And is restarted if it crashes or is killed
mkdir -p ~/.config/systemd/user/app-walker@autostart.service.d/
cp $OMARCHY_PATH/default/walker/restart.conf ~/.config/systemd/user/app-walker@autostart.service.d/restart.conf

# Create pacman hook to restart walker after updates
sudo mkdir -p /etc/pacman.d/hooks
sudo tee /etc/pacman.d/hooks/walker-restart.hook > /dev/null << EOF
[Trigger]
Type = Package
Operation = Upgrade
Target = walker
Target = walker-debug
Target = elephant*

[Action]
Description = Restarting Walker services after system update
When = PostTransaction
Exec = $OMARCHY_PATH/bin/omarchy-restart-walker
EOF

# Link the visual theme menu config
mkdir -p ~/.config/elephant/menus
ln -snf $OMARCHY_PATH/default/elephant/omarchy_themes.lua ~/.config/elephant/menus/omarchy_themes.lua
ln -snf $OMARCHY_PATH/default/elephant/omarchy_background_selector.lua ~/.config/elephant/menus/omarchy_background_selector.lua
ln -snf $OMARCHY_PATH/default/elephant/omarchy_unlocks.lua ~/.config/elephant/menus/omarchy_unlocks.lua
