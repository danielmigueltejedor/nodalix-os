# Nodalix OS Roadmap

## Phase 1 — Nodalix Layer

- [x] Create reproducible repository structure
- [x] Import current package lists
- [ ] Import current Hyprland/Waybar/WPE/media configuration
- [x] Create installer script
- [x] Add package lists
- [ ] Add restore/defaults system
- [ ] Test on current CachyOS machine
- [ ] Test in Proxmox VM

## Phase 2 — Core Desktop

- [ ] Replace Omarchy dependencies
- [ ] Create Nodalix Command Bar
- [ ] Create Nodalix Settings
- [ ] Create Nodalix Privacy Center
- [ ] Create smart window daemon
- [ ] Add privacy indicators to Waybar
- [ ] Add display/monitor profile system

## Phase 3 — Privacy and Security

- [ ] Microphone usage indicator
- [ ] Camera usage indicator
- [ ] Screen sharing indicator
- [ ] AI activity indicator
- [ ] Firewall profile
- [ ] Secrets/keyring integration
- [ ] Permission model

## Phase 4 — AI / Voice

- [ ] Optional AI provider setup
- [ ] OpenCode/OpenAI/Ollama provider abstraction
- [ ] Voice command system
- [ ] Screen reading with explicit permission
- [ ] Safe command execution with confirmation

## Phase 5 — ISO

- [ ] Build experimental ISO
- [ ] Add Nodalix branding
- [ ] Add Calamares profile
- [ ] Remove desktop choice from installer
- [ ] Add hardware detection
- [ ] Add driver installation logic
- [ ] Test in Proxmox VM

# Nodalix OS Roadmap

Nodalix OS is a custom Linux desktop environment and operating system experience focused on elegance, speed, coherence, and deep integration across native apps, system services, and Hyprland-based workflows.

The goal is not just to customize an existing distribution, but to progressively build a complete Nodalix experience with its own apps, settings, greeter, lock screen, launcher, file manager, update system, and visual identity.

---

## Vision

Nodalix aims to become a polished, modern, efficient Linux desktop experience with:

* A cohesive visual language.
* Native-feeling system apps.
* A refined Hyprland-based workflow.
* Beautiful onboarding, login, lock screen, and settings.
* Deep integration between wallpapers, themes, notifications, power controls, files, updates, and system tools.
* A clean balance between power-user flexibility and simple everyday usability.

Nodalix should feel like a real operating system, not just a collection of scripts and configuration files.

---

## Core Principles

* **Elegant by default**
  Every app and system surface should feel visually consistent and carefully designed.

* **Fast and lightweight**
  Nodalix should remain responsive, efficient, and suitable for daily use.

* **Native Linux first**
  Prefer native GTK/Rust/Wayland-friendly apps over heavy webviews when possible.

* **Hyprland deeply integrated**
  Hyprland is not just the compositor; it is part of the Nodalix desktop experience.

* **Safe and recoverable**
  Critical pieces such as the greeter, updater, and system services must always provide fallback and rollback paths.

* **User-friendly without hiding power**
  Advanced users should still be able to inspect, edit, and automate everything.

---

# Minimum Native Apps and System Modules

## 1. Nodalix Greeter

Custom graphical login screen for Nodalix.

### Goals

* Replace generic greetd/ReGreet interfaces.
* Provide a fully branded Nodalix login experience.
* Visually match the Nodalix lock screen style.

### Features

* greetd integration.
* User selector.
* Circular user avatars.
* Password input with glass pill style.
* Large centered clock.
* Date and Nodalix OS branding.
* Blurred/dimmed wallpaper background.
* Elegant loading screen after successful login.
* Fallback support through ReGreet or tuigreet.

### Priority

High

---

## 2. Nodalix Lock

Nodalix lock screen based on the current Hyprlock design.

### Goals

* Provide a polished lock screen matching the greeter.
* Keep the session secure while maintaining visual consistency.

### Features

* Clock and date.
* Nodalix branding.
* Password input.
* Wallpaper blur.
* Optional media controls.
* Optional subtle notifications.
* Shared theme with Nodalix Greeter.

### Priority

High

---

## 3. Nodalix Settings

Main system settings app.

### Goals

Become the central control panel for the entire Nodalix OS experience.

### Sections

* Appearance.
* Accent color.
* Wallpaper.
* Lock screen.
* Greeter.
* Displays.
* Keyboard.
* Mouse and touchpad.
* Audio.
* Network.
* Bluetooth.
* Startup apps.
* Notifications.
* Power.
* Updates.
* Developer tools.
* System information.

### Priority

Critical

---

## 4. Nodalix Files

Native file manager for Nodalix.

### Goals

Provide a beautiful, fast, and useful file manager that replaces generic file managers in the Nodalix experience.

### Features

* Folder navigation.
* Sidebar.
* Grid/list views.
* Copy, cut, paste, rename, delete.
* Trash support.
* Drag and drop.
* Open with.
* File properties.
* Folder colors and custom icons.
* LocalSend/share integration.
* Mounts and external drives.
* Search.
* Context menus.

### Status

In development.

### Priority

High

---

## 5. Nodalix Launcher

Command bar and app launcher.

### Goals

Provide a Spotlight/Raycast/KRunner-like experience for Nodalix.

### Features

* Launch apps.
* Search files.
* Execute quick actions.
* Open settings sections.
* Run Nodalix commands.
* Calculator.
* Web search.
* Keyboard shortcut reference.
* Developer commands.

### Priority

Critical

---

## 6. Nodalix Control Center

Quick settings panel.

### Goals

Provide a modern quick-access panel for common system actions.

### Features

* Wi-Fi.
* Bluetooth.
* Volume.
* Brightness.
* Dark/light mode.
* Do Not Disturb.
* VPN/Tailscale.
* Screenshots.
* Screen recording.
* Media controls.
* Power profile.
* Wallpaper Engine controls.

### Priority

High

---

## 7. Nodalix Notifications

Notification center and notification UI.

### Goals

Provide a consistent notification experience across Nodalix.

### Features

* Notification popups.
* Notification history.
* Grouping by app.
* Do Not Disturb.
* Action buttons.
* Priority levels.
* Consistent Nodalix styling.
* Integration with Control Center.

### Priority

Medium-high

---

## 8. Nodalix Wallpapers

Wallpaper and background manager.

### Goals

Manage static, animated, and Wallpaper Engine backgrounds across the desktop, lock screen, and greeter.

### Features

* Static wallpapers.
* Animated wallpapers.
* Wallpaper Engine integration.
* Per-monitor wallpaper selection.
* Preview UI.
* Generate blurred lock screen backgrounds.
* Generate greeter backgrounds.
* Local wallpaper library.
* Steam Workshop integration.

### Priority

High

---

## 9. Nodalix Updater

System update manager.

### Goals

Make Arch/CachyOS-based updates safer and easier for normal users.

### Features

* Pacman updates.
* AUR updates.
* Flatpak updates.
* Snapshot integration.
* Kernel update awareness.
* Reboot-required detection.
* Update history.
* Rollback guidance.
* Safe update mode.
* Notifications.

### Priority

Critical

---

## 10. Nodalix Welcome

First-run onboarding app.

### Goals

Guide users through the initial Nodalix setup.

### Features

* Welcome screen.
* Theme selection.
* Wallpaper selection.
* Keyboard layout.
* Display setup.
* Recommended apps.
* Account setup.
* Tailscale/VPN setup.
* Backup options.
* Shortcut tour.
* Developer mode toggle.

### Priority

High

---

# Additional Native Apps and Modules

## 11. Nodalix Store

Simple software center.

### Features

* Recommended apps.
* Pacman packages.
* AUR packages.
* Flatpak apps.
* Installed/not installed state.
* App categories.
* Install/remove actions.
* Nodalix official apps section.

### Priority

Medium-high

---

## 12. Nodalix Capture

Screenshot and screen recording tool.

### Features

* Fullscreen screenshot.
* Region screenshot.
* Window screenshot.
* Copy to clipboard.
* Save to file.
* Basic annotation.
* Screen recording.
* GIF recording.

### Priority

Medium-high

---

## 13. Nodalix Audio

Audio control panel and OSD system.

### Features

* Output device selection.
* Input device selection.
* Volume.
* Per-app mixer.
* Microphone level.
* Audio profiles.
* OSD integration.

### Priority

Medium

---

## 14. Nodalix Network

Network management UI.

### Features

* Wi-Fi.
* Ethernet.
* VPN.
* Tailscale.
* DNS.
* Proxy.
* Connection status.
* Troubleshooting tools.

### Priority

Medium

---

## 15. Nodalix Bluetooth

Bluetooth management UI.

### Features

* Enable/disable Bluetooth.
* Pair devices.
* Reconnect devices.
* Forget devices.
* Device battery status.
* Audio device handling.

### Priority

Medium

---

## 16. Nodalix Power

Power and session controls.

### Features

* Shutdown.
* Reboot.
* Suspend.
* Hibernate.
* Lock.
* Logout.
* Switch user.
* Power profiles.
* Battery status.

### Priority

Medium

---

## 17. Nodalix System Monitor

System resource monitor.

### Features

* CPU usage.
* RAM usage.
* GPU usage.
* Disk usage.
* Network usage.
* Processes.
* Temperatures.
* Services.
* Kill process.

### Priority

Medium

---

## 18. Nodalix Tweaks

Advanced configuration app.

### Features

* Hyprland advanced settings.
* Window rules.
* Gaps.
* Borders.
* Animations.
* Transparency.
* Autostart.
* Systemd user services.
* Environment variables.
* Gaming options.
* Developer options.

### Priority

Medium

---

## 19. Nodalix Terminal Profile

Official terminal experience.

### Goals

Nodalix does not necessarily need to build a terminal emulator from scratch, but it should provide an official terminal profile.

### Features

* Ghostty profile.
* Nodalix theme.
* Prompt.
* Fonts.
* Shell aliases.
* Useful commands.
* Developer workflow integrations.

### Priority

Medium

---

# Development Phases

## Phase 1 — System Identity

Goal: make boot, login, lock, wallpaper, and appearance feel like Nodalix.

* Nodalix Greeter.
* Nodalix Lock.
* Nodalix Wallpapers.
* Basic Nodalix Settings.

## Phase 2 — Daily Usage

Goal: replace the most visible daily desktop interactions.

* Nodalix Files.
* Nodalix Launcher.
* Nodalix Control Center.
* Nodalix Notifications.

## Phase 3 — System Maintenance

Goal: make the system usable and maintainable by more than just advanced users.

* Nodalix Updater.
* Nodalix Welcome.
* Nodalix Store.
* Nodalix System Monitor.

## Phase 4 — Full Desktop Polish

Goal: replace remaining generic utilities with coherent Nodalix-native experiences.

* Nodalix Capture.
* Nodalix Audio.
* Nodalix Network.
* Nodalix Bluetooth.
* Nodalix Power.
* Nodalix Tweaks.
* Nodalix Terminal Profile.

---

# Minimum Set for a Real Nodalix OS

The minimum set required for Nodalix to feel like a real operating system:

1. Nodalix Greeter.
2. Nodalix Lock.
3. Nodalix Settings.
4. Nodalix Control Center.
5. Nodalix Launcher.
6. Nodalix Files.
7. Nodalix Notifications.
8. Nodalix Wallpapers.
9. Nodalix Updater.
10. Nodalix Welcome.

Once these ten pieces are coherent, Nodalix becomes more than a customized desktop. It becomes a complete operating system experience.

---

# Current Recommended Focus

The current development order should be:

1. Finish Nodalix Greeter.
2. Convert the existing Hyprlock setup into Nodalix Lock.
3. Build a basic Nodalix Settings app.
4. Integrate Nodalix Wallpapers with Greeter and Lock.
5. Consolidate Nodalix Launcher/Command Bar.
6. Continue polishing Nodalix Files.

The most important long-term app is **Nodalix Settings**, because it becomes the central place where the rest of the system is controlled.

---

# Long-Term Goal

Nodalix should eventually provide a complete, coherent desktop experience where the user can:

* Install the OS.
* Complete a beautiful first-run setup.
* Log in through a custom greeter.
* Use native Nodalix apps.
* Change system settings without editing config files.
* Update safely.
* Recover from issues.
* Customize the system deeply.
* Enjoy a visually consistent, fast, and elegant Linux desktop.

Nodalix OS should feel intentional from boot to shutdown.

# Nodalix Engineering and Productivity Suite

Beyond the core operating system experience, Nodalix aims to provide a native suite of productivity, creative, and engineering tools built specifically for Linux.

The goal is to create applications that feel deeply integrated with Nodalix OS while also being useful on any modern Linux desktop.

---

## Vision

Nodalix should not only provide a beautiful desktop environment, but also a complete set of native tools for work, study, engineering, creativity, and daily productivity.

This suite should focus on:

- Native Linux performance.
- A cohesive Nodalix visual language.
- Engineering-friendly workflows.
- Clean export formats.
- Interoperability with common standards.
- A balance between simplicity and professional power.

---

# Planned Applications

## Nodalix CAD

Native CAD application for Linux.

### Goals

Provide a modern, elegant, and engineering-focused CAD experience for Linux users.

### Features

- 2D technical drawing.
- Layers.
- Dimensions.
- Blocks.
- Snapping.
- Layouts.
- PDF export.
- DXF import/export.
- DWG compatibility research.
- AutoCAD-like command workflow.
- Nodalix-native interface.
- Engineering templates.
- Native `.nodcad` project format.

### Priority

Very high.

---

## Nodalix Pages

Document editor and technical writing app.

### Goals

Create beautiful documents, reports, academic papers, and engineering documentation.

### Features

- Rich text editing.
- Technical reports.
- Templates.
- Tables.
- Figures.
- Automatic table of contents.
- Equations.
- PDF export.
- Markdown/document hybrid mode.
- Engineering report templates.

### Priority

Medium-high.

---

## Nodalix Cells

Spreadsheet application.

### Goals

Provide a spreadsheet app with strong support for engineering calculations.

### Features

- Tables.
- Formulas.
- Charts.
- CSV support.
- XLSX compatibility research.
- PDF export.
- Engineering templates.
- Unit-aware calculations.
- Physical magnitudes.
- Technical plotting.

### Example

`10 m + 25 cm = 10.25 m`

### Priority

Medium-high.

---

## Nodalix Point

Presentation app.

### Goals

Create elegant technical and academic presentations.

### Features

- Slides.
- Nodalix templates.
- Presentation mode.
- PDF export.
- Basic animations.
- Charts.
- Diagrams.
- Engineering presentation templates.
- Project pitch templates.

### Priority

Medium.

---

## Nodalix Drop

Local file transfer app.

### Goals

Provide an AirDrop-like experience for Nodalix and Linux devices.

### Features

- Send files over local network.
- Receive files.
- Device discovery.
- QR pairing.
- Transfer history.
- Nodalix Files integration.
- Mobile interoperability research.

### Priority

High.

---

## Nodalix Photo

Image viewer and lightweight editor.

### Goals

Provide a fast and elegant image experience for Nodalix.

### Features

- Image viewing.
- Crop.
- Rotate.
- Brightness/contrast.
- Basic filters.
- Annotations.
- Format conversion.
- Simple library view.

### Priority

Medium.

---

## Nodalix Video

Video player and lightweight editor.

### Goals

Provide a simple video tool for playback, trimming, conversion, and basic editing.

### Features

- Video playback.
- Trim clips.
- Convert formats.
- Extract audio.
- Export GIF.
- Capture frames.
- Compression.
- Subtitle support.
- Future timeline editing.

### Priority

Medium.

---

## Nodalix Draw

Vector drawing and diagramming app.

### Goals

Create diagrams, technical illustrations, architecture sketches, and clean vector graphics.

### Features

- Shapes.
- Lines.
- Arrows.
- Connectors.
- Text.
- Layers.
- SVG export.
- PDF export.
- PNG export.
- Network diagrams.
- Software architecture diagrams.
- Engineering figures.

### Priority

Medium-high.

---

## Nodalix Notes

Markdown-based notes and knowledge app.

### Goals

Provide a clean native notes app for study, projects, and technical documentation.

### Features

- Markdown notes.
- Folders.
- Tags.
- Search.
- Code blocks.
- Equations.
- Images.
- Internal links.
- Project notebooks.

### Priority

Medium.

---

## Nodalix Lab

Engineering and scientific tools app.

### Goals

Provide native engineering utilities for calculations, plotting, unit conversion, and scientific workflows.

### Features

- Scientific calculator.
- Unit converter.
- 2D plotting.
- Equation solving.
- Matrices.
- Interpolation.
- Numerical integration.
- Numerical derivatives.
- Atmospheric tables.
- Aerodynamic utilities.
- Airfoil tools.
- Basic structural calculations.

### Priority

High.

---

# Suite Priorities

The first productivity and engineering apps should be:

1. Nodalix CAD.
2. Nodalix Drop.
3. Nodalix Lab.
4. Nodalix Draw.
5. Nodalix Pages.
6. Nodalix Cells.
7. Nodalix Point.
8. Nodalix Photo.
9. Nodalix Video.
10. Nodalix Notes.

---

# Long-Term Goal

The long-term goal is to make Nodalix not only a desktop operating system, but also a complete native productivity and engineering platform for Linux.

Nodalix should become a place where users can:

- Design.
- Calculate.
- Write.
- Present.
- Transfer files.
- Edit media.
- Build engineering projects.
- Manage technical documentation.
- Work inside a coherent Linux-native ecosystem.

The Nodalix app suite should feel intentional, integrated, and professional.
