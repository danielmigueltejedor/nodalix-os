# Nodalix OS — Project Context Source

## Identidad del proyecto

Nodalix OS es un entorno Linux personalizado basado en CachyOS/Arch + Hyprland, orientado a una experiencia tipo macOS moderna, limpia, premium e intuitiva, pero con identidad propia.

El sistema busca construir progresivamente una shell propia:
- Nodalix Settings
- Nodalix Greeter
- Nodalix Bar
- Nodalix Control Center
- Nodalix Command Bar / Super+K
- Sistema de preferencias globales
- Identidad visual unificada

Nodalia OS queda reservado para un futuro sistema basado en Home Assistant OS/Supervisor.  
Nodalix OS es el entorno Linux de escritorio.

---

## Objetivo general

Convertir Nodalix OS en un sistema coherente, no solo una colección de dotfiles.

El sistema debe tener:
- Configuración centralizada.
- UX intuitiva.
- Apariencia moderna tipo macOS.
- Integración entre Settings, Greeter, Bar, Control Center y Command Bar.
- Acciones reales de sistema.
- Scripts robustos.
- Preferencia por soluciones limpias y mantenibles frente a hacks rápidos.

---

## Preferencias técnicas

- Distribución base: CachyOS / Arch Linux.
- Compositor: Hyprland.
- Greeter: greetd / regreet o greeter propio si el proyecto evoluciona.
- Shell: zsh.
- Fuente principal deseada: Inter.
- Paquete correcto de Inter en Arch/CachyOS: inter-font.
- No usar ttf-inter porque no existe en los repos actuales.
- Gestor JavaScript preferido: pnpm.
- Evitar npm salvo necesidad.
- Editor principal: Cursor/Zed.
- Scripts shell deben ser robustos y, si es posible, pasar shellcheck.
- No usar sudo dentro de apps gráficas para acciones de usuario.
- Usar systemd/logind/Hyprland correctamente.

---

## Fuente visual

La pila de fuentes recomendada para toda la interfaz es:

Inter, SF Pro Display, Segoe UI, Roboto, Ubuntu, Cantarell, Noto Sans, sans-serif

Inter debe aplicarse en:
- Settings
- Greeter
- Command Bar
- Bar
- Control Center
- Componentes visuales comunes
- CSS/GTK/themes si aplica

---

## Super+K / Command Bar

Super+K debe abrir la command bar principal de Nodalix OS.

La command bar debe comportarse como Spotlight/Raycast:
- Priorizar apps locales.
- Priorizar acciones del sistema.
- Priorizar settings.
- Priorizar comandos internos.
- La búsqueda web debe ser fallback, no primer resultado por defecto.

Comportamiento deseado:
- Si escribo "curs", el primer resultado debe ser Cursor.
- Si hay coincidencias locales, la búsqueda web aparece después.
- Si no hay coincidencias locales, la búsqueda web puede ser el primer resultado.

Ranking deseado:
1. App exact match.
2. App startsWith.
3. Acción del sistema exacta o startsWith.
4. Settings exacto o startsWith.
5. Coincidencia parcial local.
6. Fuzzy local.
7. Web search si hay resultados locales.
8. Web search primero solo si no hay resultados locales.

La web no debe competir como si fuera una app local.

---

## Nodalix Settings

Nodalix Settings debe evolucionar hacia una experiencia parecida a Preferencias del Sistema de macOS:
- Sidebar clara.
- Header visual de usuario.
- Tarjetas limpias.
- Controles grandes e intuitivos.
- Acciones rápidas visibles.
- Feedback inmediato.
- Diseño premium, moderno y simple.

Settings debe ser el centro de preferencias del sistema.

Debe gestionar:
- Avatar del usuario.
- Preferencias visuales.
- Color de acento futuro.
- Tema claro/oscuro futuro.
- Densidad visual futura.
- Blur/transparencia futura.
- Radio de bordes futuro.
- Acciones de energía.

---

## Imagen/avatar de usuario

La imagen seleccionada en Settings debe guardarse de forma persistente y aplicarse también en el Greeter.

Ruta sugerida si no existe una arquitectura mejor:

~/.config/nodalix/user/profile.json  
~/.config/nodalix/user/avatar.png

El perfil debe poder ampliarse en el futuro con:
- Nombre visible.
- Avatar.
- Color de acento.
- Tema.
- Preferencias visuales.
- Preferencias de shell.

Settings escribe esta configuración.  
Greeter la lee.  
Nodalix Bar y Control Center también deberían poder leerla en el futuro.

Si no hay avatar configurado, usar fallback elegante.

---

## Greeter

El Greeter debe:
- Mostrar la imagen de usuario seleccionada desde Settings.
- Usar Inter si está disponible.
- Mantener fallback si no existe avatar.
- No romper el login existente.
- Leer configuración compartida del perfil de usuario.
- Tener estética coherente con Nodalix OS.

---

## Nodalix Bar

Se quiere empezar a construir una barra propia para sustituir o complementar Waybar en el futuro.

Estructura deseada:
- Zona izquierda: logo, launcher, espacios, apps.
- Zona central: reloj, app activa, workspace o widgets.
- Zona derecha: estado del sistema y Control Center.

Debe ser modular y ampliable.

---

## Control Center

El Control Center debe abrirse desde la parte derecha de la barra, estilo macOS.

Estética deseada:
- Panel flotante.
- Blur/transparencia si el stack lo permite.
- Esquinas redondeadas.
- Sombras suaves.
- Toggles grandes.
- Sliders visuales.

Módulos iniciales:
- Wi-Fi.
- Bluetooth.
- Volumen.
- Brillo.
- Batería si existe.
- Tema claro/oscuro si existe.
- Bloquear.
- Suspender.
- Apagar/reiniciar con confirmación.

Si un módulo no está implementado todavía, usar placeholder elegante y arquitectura preparada.

---

## Acciones de energía en Settings

Los botones de Nodalix Settings deben ser funcionales, no decorativos.

Acciones reales:
- Apagar: systemctl poweroff
- Reiniciar: systemctl reboot
- Suspender: systemctl suspend
- Hibernar si está soportado: systemctl hibernate
- Bloquear: nodalix-lock
- Cerrar sesión Hyprland: hyprctl dispatch exit

Requisitos:
- No usar sudo.
- No usar comandos peligrosos sin confirmación.
- Apagar, reiniciar, cerrar sesión e hibernar deben pedir confirmación visual.
- Suspender y bloquear pueden ejecutarse directamente.
- La UI debe mostrar error si falla una acción.
- Las acciones deben pasar por un helper central, no estar hardcodeadas en botones.

Helper sugerido:
- systemActions
- powerActions
- nodalixSystem

---

## Hyprlock / bloqueo

Problema detectado:
Al desbloquear tras mucho rato, hyprlock puede volver a aparecer varias veces. Probable causa: varias instancias de hyprlock lanzadas por hypridle, DPMS, suspend/resume o bindings duplicados.

Solución deseada:
Crear wrapper único `nodalix-lock`.

Ruta recomendada:
~/.local/bin/nodalix-lock

Contenido recomendado:

#!/usr/bin/env bash
set -euo pipefail

if pgrep -x hyprlock >/dev/null 2>&1; then
  exit 0
fi

exec hyprlock

Debe instalarse con permisos:
chmod 755 ~/.local/bin/nodalix-lock

Todas las llamadas manuales a bloqueo deben pasar por nodalix-lock o loginctl lock-session con lock_cmd protegido.

---

## Hypridle

Hypridle no debe lanzar hyprlock varias veces.

Configuración conceptual deseada:

general {
    lock_cmd = pgrep -x hyprlock >/dev/null || nodalix-lock
    before_sleep_cmd = loginctl lock-session
    after_sleep_cmd = hyprctl dispatch dpms on
}

listener {
    timeout = 300
    on-timeout = loginctl lock-session
}

listener {
    timeout = 360
    on-timeout = hyprctl dispatch dpms off
    on-resume = hyprctl dispatch dpms on
}

Reglas:
- No llamar hyprlock directamente desde varios listeners.
- DPMS solo apaga/enciende pantalla.
- No iniciar hypridle dos veces.
- Revisar si hypridle se lanza por exec-once y también por systemd user.
- Dejar solo una vía clara.

---

## Error conocido de Lua

Error detectado:

hl.dispatch(exec hypridle)

Esto es sintaxis inválida.

Si se usa Lua, debe pasarse como string o mediante la API correcta, por ejemplo:

hl.dispatch('exec hypridle')

Si es configuración Hyprland normal, debe usarse:

exec-once = hypridle

Hay que buscar y corregir cualquier aparición de:
- hl.dispatch(exec hypridle)
- exec hypridle mal escapado
- hypridle duplicado

---

## Paquetes importantes

Paquetes base esperados:
- hyprland
- hyprlock
- hypridle
- waybar mientras no exista Nodalix Bar completa
- mako
- rofi o launcher actual
- xdg-desktop-portal
- xdg-desktop-portal-hyprland
- xdg-desktop-portal-gtk
- grim
- slurp
- satty
- wl-clipboard
- cliphist
- playerctl
- pamixer
- pavucontrol
- pipewire
- pipewire-pulse
- pipewire-alsa
- wireplumber
- networkmanager
- bluez
- bluez-utils
- inter-font

No usar:
- ttf-inter

---

## Estado reciente conocido

Rama vista:
feature/nodalix-greeter

Problemas recientes:
- Botones de energía de Settings no ejecutan acciones reales.
- nodalix-lock no existe en ~/.local/bin.
- ttf-inter falló porque el paquete correcto es inter-font.
- hyprlock ya estaba actualizado.
- Error de Lua con hl.dispatch(exec hypridle).
- Posible duplicidad de hyprlock/hypridle.
- Se quiere aplicar avatar de Settings en Greeter.
- Se quiere rediseñar Settings estilo macOS.
- Se quiere construir Nodalix Bar y Control Center.
- Se quiere mejorar ranking de Command Bar para priorizar local sobre web.

---

## Comandos útiles de diagnóstico

Buscar referencias de bloqueo:

grep -RInE 'hyprlock|hypridle|nodalix-lock|lock-session|dpms|suspend' ~/.config ~/Proyectos/nodalix-os 2>/dev/null

Comprobar procesos:

pgrep -a hyprlock
pgrep -a hypridle

Comprobar hyprlock:

command -v hyprlock
hyprlock --version

Comprobar wrapper:

command -v nodalix-lock
ls -l "$(command -v nodalix-lock)"

Buscar error Lua:

grep -RIn "hl.dispatch(exec hypridle)\|exec hypridle\|hypridle" ~/.config ~/Proyectos/nodalix-os 2>/dev/null

---

## Filosofía de desarrollo

Priorizar:
- Arquitectura limpia.
- Componentes reutilizables.
- Configuración compartida.
- UX intuitiva.
- Degradación elegante.
- Feedback visual.
- Validaciones reales.
- Scripts robustos.

Evitar:
- Hacks rápidos.
- Duplicar lógica.
- Comandos peligrosos sin confirmación.
- UI falsa sin backend funcional.
- Hardcodear rutas si ya existen helpers.
- Romper configuraciones existentes sin migración.

---

## Validaciones deseadas

Según el stack del repo, ejecutar lo que aplique:

pnpm lint
pnpm build
cargo fmt
cargo check
cargo test
shellcheck scripts/**/*.sh

Si una validación falla, explicar:
- qué comando falló
- por qué falló
- archivo implicado
- propuesta concreta de arreglo

---

## Prompt base para ChatGPT/Cursor

Cuando trabajes sobre Nodalix OS, usa este archivo como fuente de contexto del proyecto. Antes de modificar código:
1. Lee este archivo.
2. Respeta las decisiones aquí descritas.
3. Busca implementaciones existentes antes de crear nuevas.
4. No dupliques lógica.
5. Mantén compatibilidad con Hyprland/CachyOS/Arch.
6. Da siempre resumen final de archivos modificados y comandos de prueba.
