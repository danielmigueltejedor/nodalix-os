#!/usr/bin/env fish

# =======================================
# Script Fish: Configuración Nodalix Waybar
# =======================================

# Variables de paths
set CONFIG_DIR ~/Projects/nodalix-os/config/waybar
set LOCAL_BIN ~/Projects/nodalix-os/local/bin

# ---------------------------------------
# 1) Link de configuración y estilos
# ---------------------------------------
ln -sf $CONFIG_DIR/config.jsonc ~/.config/waybar/config.jsonc
ln -sf $CONFIG_DIR/style.css ~/.config/waybar/style.css

# ---------------------------------------
# 2) Configuración de módulos Right Island
# ---------------------------------------
# Cada módulo llama a nodalix-popup-toggle con toggle
# para abrir/cerrar popups y no lanzar infinitos
set -l right_modules "custom/nodalix-privacy" "custom/nodalix-settings" "bluetooth" "network" "pulseaudio"

for module in $right_modules
    switch $module
        case "custom/nodalix-privacy"
            set on_click "nodalix-popup-toggle privacy"
        case "custom/nodalix-settings"
            set on_click "nodalix-menu"
        case "bluetooth"
            set on_click "nodalix-popup-toggle bt"
        case "network"
            set on_click "nodalix-popup-toggle wifi"
        case "pulseaudio"
            set on_click "nodalix-popup-toggle volume"
    end
    echo "Módulo $module: $on_click"
end

# ---------------------------------------
# 3) Reinicio seguro de Waybar
# ---------------------------------------
echo "Reiniciando Waybar..."
# Matar procesos Waybar existentes
pkill waybar

# Pequeña espera
sleep 0.5

# Lanzar Waybar en segundo plano con nohup para Fish
nohup waybar >/tmp/waybar.log 2>&1 &

echo "Waybar lanzado en background. Revisa /tmp/waybar.log para logs."
