pragma Singleton

import QtQuick
import Quickshell

Singleton {
    id: root

    // Perfil visual del pack actual.
    // No es un tamaño absoluto: es normalización óptica.
    readonly property real baseScale: 0.90
    readonly property real baseOpacity: 0.88

    // Compatibilidad entre nombres usados históricamente por Nodalix
    // y nombres Freedesktop/Adwaita reales.
    readonly property var aliases: ({
        "notification-disabled-symbolic": "notifications-disabled-symbolic",
        "notification-new-symbolic": "preferences-system-notifications-symbolic",
        "notifications-symbolic": "preferences-system-notifications-symbolic",

        "bluetooth-paired-symbolic": "bluetooth-active-symbolic",
        "brightness-high-symbolic": "display-brightness-symbolic",

        "network-wireless-disconnected-symbolic":
            "network-wireless-offline-symbolic",

        "nm-signal-0-symbolic":
            "network-wireless-signal-none-symbolic",
        "nm-signal-25-symbolic":
            "network-wireless-signal-weak-symbolic",
        "nm-signal-50-symbolic":
            "network-wireless-signal-ok-symbolic",
        "nm-signal-75-symbolic":
            "network-wireless-signal-good-symbolic",
        "nm-signal-100-symbolic":
            "network-wireless-signal-excellent-symbolic",

        "screenshooter-symbolic": "camera-photo-symbolic",
        "phonelink-symbolic": "phone-symbolic",
        "dark-mode-symbolic": "night-light-symbolic",
        "utilities-system-monitor-symbolic":
            "applications-system-symbolic",
        "application-menu-symbolic": "open-menu-symbolic",
        "disk-usage-app-symbolic": "drive-harddisk-symbolic",
        "weather-app-symbolic": "weather-clear-symbolic"
    })

    // Ajustes ópticos por icono.
    //
    // Aquí arreglamos casos como el micrófono SIN tocar Audio.qml,
    // Dashboard.qml ni ningún popup.
    readonly property var tuning: ({
        "microphone-sensitivity-high-symbolic": {
            scale: 0.80,
            opacity: 0.70
        },

        "microphone-sensitivity-muted-symbolic": {
            scale: 0.80,
            opacity: 0.70
        },

        "audio-volume-high-symbolic": {
            scale: 0.88,
            opacity: 0.84
        },

        "audio-volume-medium-symbolic": {
            scale: 0.88,
            opacity: 0.84
        },

        "audio-volume-low-symbolic": {
            scale: 0.88,
            opacity: 0.84
        },

        "audio-volume-muted-symbolic": {
            scale: 0.88,
            opacity: 0.84
        }
    })

    // API semántica de Nodalix.
    //
    // Los componentes nuevos deberían pedir "settings.appearance",
    // "audio.microphone", etc. y NO nombres concretos de Adwaita.
    readonly property var roles: ({
        "wallpaper.picker": {
            candidates: [
                "preferences-desktop-wallpaper-symbolic",
                "image-x-generic-symbolic",
                "camera-photo-symbolic"
            ]
        },

        "desktop.widget": {
            states: {
                "editor": {
                    candidates: [
                        "preferences-desktop-symbolic",
                        "applications-utilities-symbolic"
                    ]
                },
                "system": {
                    candidates: [
                        "utilities-system-monitor-symbolic",
                        "preferences-system-symbolic",
                        "computer-symbolic"
                    ]
                },
                "agenda": {
                    candidates: ["x-office-calendar-symbolic"]
                },
                "storage": {
                    candidates: ["drive-harddisk-symbolic"]
                },
                "privacy": {
                    candidates: [
                        "security-high-symbolic",
                        "system-lock-screen-symbolic"
                    ]
                },
                "calendar-empty": {
                    candidates: [
                        "x-office-calendar-symbolic",
                        "appointment-missed-symbolic"
                    ]
                }
            }
        },

        "desktop.metric": {
            states: {
                "cpu": {
                    candidates: [
                        "utilities-system-monitor-symbolic",
                        "computer-symbolic"
                    ]
                },
                "memory": {
                    candidates: [
                        "media-flash-symbolic",
                        "computer-symbolic"
                    ]
                },
                "temperature": {
                    candidates: [
                        "weather-clear-symbolic",
                        "utilities-system-monitor-symbolic"
                    ]
                }
            }
        },

        "desktop.edit": {
            states: {
                "close": {
                    candidates: ["window-close-symbolic"]
                },
                "remove": {
                    candidates: ["edit-delete-symbolic"]
                },
                "drag": {
                    candidates: [
                        "transform-move-symbolic",
                        "view-more-symbolic"
                    ]
                }
            }
        },

        "weather.condition": {
            states: {
                "clear": {
                    candidates: ["weather-clear-symbolic"]
                },
                "partly-cloudy": {
                    candidates: ["weather-few-clouds-symbolic", "weather-overcast-symbolic"]
                },
                "cloudy": {
                    candidates: ["weather-overcast-symbolic"]
                },
                "fog": {
                    candidates: ["weather-fog-symbolic", "weather-overcast-symbolic"]
                },
                "drizzle": {
                    candidates: ["weather-showers-scattered-symbolic", "weather-showers-symbolic"]
                },
                "rain": {
                    candidates: ["weather-showers-symbolic", "weather-showers-scattered-symbolic"]
                },
                "snow": {
                    candidates: ["weather-snow-symbolic"]
                },
                "storm": {
                    candidates: ["weather-storm-symbolic", "weather-severe-alert-symbolic"]
                },
                "unknown": {
                    candidates: ["weather-overcast-symbolic"]
                }
            }
        },

        "audio.microphone": {
            states: {
                "active": {
                    candidates: [
                        "microphone-sensitivity-high-symbolic",
                        "audio-input-microphone-symbolic"
                    ],
                    scale: 0.88,
                    opacity: 0.72
                },

                "muted": {
                    candidates: [
                        "microphone-sensitivity-muted-symbolic",
                        "microphone-disabled-symbolic"
                    ],
                    scale: 0.88,
                    opacity: 0.72
                }
            }
        },

        "audio.volume": {
            states: {
                "muted": {
                    candidates: [
                        "audio-volume-muted-symbolic"
                    ]
                },

                "low": {
                    candidates: [
                        "audio-volume-low-symbolic"
                    ]
                },

                "medium": {
                    candidates: [
                        "audio-volume-medium-symbolic"
                    ]
                },

                "high": {
                    candidates: [
                        "audio-volume-high-symbolic"
                    ]
                }
            }
        },

        "audio.devices": {
            candidates: [
                "audio-card-symbolic",
                "audio-speakers-symbolic",
                "multimedia-volume-control-symbolic",
                "preferences-system-symbolic"
            ],
            scale: 0.90
        },

        "media.player": {
            states: {
                "audio": {
                    candidates: [
                        "audio-x-generic-symbolic",
                        "multimedia-player-symbolic"
                    ]
                },
                "video": {
                    candidates: [
                        "video-x-generic-symbolic",
                        "multimedia-player-symbolic"
                    ]
                },
                "browser": {
                    candidates: [
                        "internet-web-browser-symbolic",
                        "web-browser-symbolic",
                        "applications-internet-symbolic"
                    ]
                },
                "generic": {
                    candidates: [
                        "multimedia-player-symbolic",
                        "audio-x-generic-symbolic",
                        "media-playback-start-symbolic"
                    ]
                }
            }
        },

        "media.placeholder": {
            candidates: [
                "audio-x-generic-symbolic",
                "multimedia-player-symbolic",
                "media-playback-start-symbolic"
            ],
            opacity: 0.72
        },

        "ui.disclosure": {
            states: {
                "open": {
                    candidates: [
                        "pan-up-symbolic",
                        "go-up-symbolic"
                    ]
                },
                "closed": {
                    candidates: [
                        "pan-down-symbolic",
                        "go-down-symbolic"
                    ]
                }
            }
        },

        "device.peer": {
            states: {
                "mobile": {
                    candidates: [
                        "phone-symbolic",
                        "computer-symbolic"
                    ]
                },
                "computer": {
                    candidates: [
                        "computer-symbolic",
                        "video-display-symbolic"
                    ]
                }
            }
        },

        "favorite": {
            states: {
                "yes": {
                    candidates: ["starred-symbolic"]
                },
                "no": {
                    candidates: [
                        "non-starred-symbolic",
                        "starred-symbolic"
                    ]
                }
            }
        },

        "capture.action": {
            states: {
                "full": {
                    candidates: ["camera-photo-symbolic"]
                },
                "region": {
                    candidates: [
                        "selection-mode-symbolic",
                        "edit-select-all-symbolic"
                    ]
                }
            }
        },

        "calendar.detail": {
            states: {
                "time": {
                    candidates: [
                        "appointment-soon-symbolic",
                        "preferences-system-time-symbolic"
                    ]
                },
                "location": {
                    candidates: [
                        "find-location-symbolic",
                        "mark-location-symbolic"
                    ]
                },
                "reminder": {
                    candidates: [
                        "alarm-symbolic",
                        "emblem-important-symbolic"
                    ]
                }
            }
        },

        "user.avatar": {
            candidates: [
                "avatar-default-symbolic",
                "system-users-symbolic"
            ]
        },

        "settings.power-profile": {
            "states": {
                "power-saver": {
                    "candidates": [
                        "power-profile-power-saver-symbolic",
                        "battery-good-symbolic",
                        "preferences-system-power-symbolic"
                    ]
                },
                "balanced": {
                    "candidates": [
                        "power-profile-balanced-symbolic",
                        "preferences-system-power-symbolic"
                    ]
                },
                "performance": {
                    "candidates": [
                        "power-profile-performance-symbolic",
                        "system-run-symbolic",
                        "preferences-system-power-symbolic"
                    ]
                }
            }
        },

        "settings.privacy-sensor": {
            "states": {
                "microphone": {
                    "candidates": [
                        "audio-input-microphone-symbolic",
                        "audio-input-microphone-muted-symbolic"
                    ]
                },
                "camera": {
                    "candidates": [
                        "camera-web-symbolic",
                        "camera-photo-symbolic"
                    ]
                },
                "location": {
                    "candidates": [
                        "find-location-symbolic",
                        "mark-location-symbolic"
                    ]
                }
            }
        },

        "settings.system-info": {
            "states": {
                "device": {
                    "candidates": [
                        "computer-symbolic",
                        "video-display-symbolic"
                    ]
                },
                "os": {
                    "candidates": [
                        "applications-system-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "kernel": {
                    "candidates": [
                        "preferences-system-symbolic"
                    ]
                },
                "cpu": {
                    "candidates": [
                        "computer-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "memory": {
                    "candidates": [
                        "media-flash-symbolic",
                        "drive-harddisk-symbolic"
                    ]
                },
                "gpu": {
                    "candidates": [
                        "video-display-symbolic",
                        "preferences-desktop-display-symbolic"
                    ]
                }
            }
        },

        "navigation.back": {
            "candidates": [
                "go-previous-symbolic",
                "pan-start-symbolic"
            ]
        },

        "settings.dependency": {
            "states": {
                "ok": {
                    "candidates": [
                        "emblem-ok-symbolic",
                        "object-select-symbolic"
                    ]
                },
                "missing": {
                    "candidates": [
                        "dialog-error-symbolic",
                        "process-stop-symbolic"
                    ]
                }
            }
        },

        "audio.device-selection": {
            "states": {
                "selected": {
                    "candidates": [
                        "object-select-symbolic",
                        "emblem-ok-symbolic"
                    ]
                },
                "unselected": {
                    "candidates": [
                        "audio-card-symbolic",
                        "audio-speakers-symbolic"
                    ]
                }
            }
        },

        "settings.update-channel": {
            "states": {
                "stable": {
                    "candidates": [
                        "emblem-ok-symbolic",
                        "object-select-symbolic",
                        "software-update-available-symbolic"
                    ]
                },
                "beta": {
                    "candidates": [
                        "software-update-available-symbolic",
                        "dialog-information-symbolic"
                    ]
                }
            }
        },

        "settings.update-component": {
            "states": {
                "shell": {
                    "candidates": [
                        "preferences-system-symbolic",
                        "utilities-system-monitor-symbolic"
                    ]
                },
                "apps": {
                    "candidates": [
                        "view-app-grid-symbolic",
                        "applications-system-symbolic"
                    ]
                },
                "themes": {
                    "candidates": [
                        "preferences-desktop-wallpaper-symbolic",
                        "applications-graphics-symbolic",
                        "preferences-system-symbolic"
                    ]
                }
            }
        },

        "settings.update-source": {
            "states": {
                "system": {
                    "candidates": [
                        "applications-system-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "aur": {
                    "candidates": [
                        "package-x-generic-symbolic",
                        "applications-system-symbolic"
                    ]
                },
                "flatpak": {
                    "candidates": [
                        "package-x-generic-symbolic",
                        "view-app-grid-symbolic",
                        "applications-system-symbolic"
                    ]
                },
                "firmware": {
                    "candidates": [
                        "drive-harddisk-symbolic",
                        "preferences-system-symbolic"
                    ]
                }
            }
        },

        "wallpaper.media": {
            "states": {
                "video": {
                    "candidates": [
                        "video-x-generic-symbolic",
                        "media-playback-start-symbolic"
                    ]
                }
            }
        },

        "wallpaper.rotation": {
            "candidates": [
                "view-refresh-symbolic",
                "media-playlist-repeat-symbolic"
            ]
        },

        "wallpaper.download": {
            "candidates": [
                "folder-download-symbolic",
                "document-save-symbolic",
                "network-receive-symbolic"
            ]
        },

        "settings.item": {
            "states": {
                "personal-group": {
                    "candidates": [
                        "system-users-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "appearance-group": {
                    "candidates": [
                        "dark-mode-symbolic",
                        "preferences-desktop-wallpaper-symbolic",
                        "preferences-desktop-appearance-symbolic"
                    ]
                },
                "connections-group": {
                    "candidates": [
                        "preferences-system-network-symbolic",
                        "network-wireless-signal-excellent-symbolic"
                    ]
                },
                "devices-group": {
                    "candidates": [
                        "preferences-desktop-display-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "applications-group": {
                    "candidates": [
                        "view-app-grid-symbolic",
                        "preferences-desktop-apps-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "services-group": {
                    "candidates": [
                        "preferences-system-symbolic"
                    ]
                },
                "system-group": {
                    "candidates": [
                        "utilities-system-monitor-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "user": {
                    "candidates": [
                        "system-users-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "security": {
                    "candidates": [
                        "system-lock-screen-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "date-time": {
                    "candidates": [
                        "preferences-system-time-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "appearance": {
                    "candidates": [
                        "dark-mode-symbolic",
                        "preferences-desktop-wallpaper-symbolic",
                        "preferences-desktop-appearance-symbolic"
                    ]
                },
                "bar": {
                    "candidates": [
                        "application-menu-symbolic",
                        "view-grid-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "media": {
                    "candidates": [
                        "audio-speakers-symbolic",
                        "multimedia-volume-control-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "widgets": {
                    "candidates": [
                        "view-grid-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "wallpaper": {
                    "candidates": [
                        "preferences-desktop-wallpaper-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "connectivity": {
                    "candidates": [
                        "preferences-system-network-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "phone-link": {
                    "candidates": [
                        "phone-symbolic",
                        "computer-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "localsend": {
                    "candidates": [
                        "network-transmit-symbolic",
                        "send-to-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "hyprland": {
                    "candidates": [
                        "preferences-desktop-display-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "sound": {
                    "candidates": [
                        "audio-speakers-symbolic",
                        "multimedia-volume-control-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "power": {
                    "candidates": [
                        "preferences-system-power-symbolic",
                        "power-profile-balanced-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "default-apps": {
                    "candidates": [
                        "application-menu-symbolic",
                        "preferences-desktop-apps-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "recovery": {
                    "candidates": [
                        "document-revert-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "storage": {
                    "candidates": [
                        "drive-harddisk-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "nodalix-updates": {
                    "candidates": [
                        "software-update-available-symbolic",
                        "network-receive-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "updates": {
                    "candidates": [
                        "software-update-available-symbolic",
                        "network-receive-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "tray": {
                    "candidates": [
                        "open-menu-symbolic",
                        "application-menu-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "notifications": {
                    "candidates": [
                        "preferences-system-notifications-symbolic",
                        "notification-new-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "privacy": {
                    "candidates": [
                        "preferences-system-privacy-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "weather": {
                    "candidates": [
                        "weather-clear-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "keybindings": {
                    "candidates": [
                        "preferences-desktop-keyboard-shortcuts-symbolic",
                        "preferences-desktop-keyboard-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "tools": {
                    "candidates": [
                        "applications-utilities-symbolic",
                        "utilities-terminal-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "dependencies": {
                    "candidates": [
                        "applications-system-symbolic",
                        "preferences-system-symbolic"
                    ]
                },
                "advanced": {
                    "candidates": [
                        "preferences-system-symbolic"
                    ]
                },
                "about": {
                    "candidates": [
                        "preferences-system-details-symbolic",
                        "utilities-system-monitor-symbolic",
                        "preferences-system-symbolic"
                    ]
                }
            }
        },

        "dashboard.tab": {
            states: {
                "home": {
                    candidates: [
                        "go-home-symbolic",
                        "user-home-symbolic"
                    ]
                },
                "media": {
                    candidates: [
                        "applications-multimedia-symbolic",
                        "media-playback-start-symbolic"
                    ]
                }
            }
        },

        "media.shuffle": {
            candidates: ["media-playlist-shuffle-symbolic"]
        },

        "media.previous": {
            candidates: ["media-skip-backward-symbolic"]
        },

        "media.playback": {
            states: {
                "play": {
                    candidates: ["media-playback-start-symbolic"]
                },
                "pause": {
                    candidates: ["media-playback-pause-symbolic"]
                }
            }
        },

        "media.next": {
            candidates: ["media-skip-forward-symbolic"]
        },

        "media.repeat": {
            states: {
                "off": {
                    candidates: ["media-playlist-repeat-symbolic"]
                },
                "all": {
                    candidates: ["media-playlist-repeat-symbolic"]
                },
                "one": {
                    candidates: [
                        "media-playlist-repeat-song-symbolic",
                        "media-playlist-repeat-symbolic"
                    ]
                }
            }
        },

        "media.window": {
            candidates: [
                "video-display-symbolic",
                "preferences-desktop-display-symbolic"
            ]
        },

        "media.raise": {
            candidates: [
                "window-new-symbolic",
                "go-up-symbolic"
            ]
        },

        "display.brightness": {
            candidates: ["display-brightness-symbolic"]
        },

        "display.monitor": {
            candidates: [
                "video-display-symbolic",
                "preferences-desktop-display-symbolic"
            ]
        },

        "layout.mode": {
            states: {
                "desktop": {
                    candidates: [
                        "preferences-desktop-display-symbolic",
                        "video-display-symbolic",
                        "window-new-symbolic"
                    ]
                },
                "tiling": {
                    candidates: [
                        "view-grid-symbolic",
                        "view-app-grid-symbolic",
                        "applications-other-symbolic"
                    ]
                },
                "scrolling": {
                    candidates: [
                        "view-list-symbolic",
                        "go-next-symbolic",
                        "pan-end-symbolic"
                    ]
                }
            }
        },

        "capture.screen": {
            states: {
                "idle": {
                    candidates: ["camera-photo-symbolic"]
                },
                "recording": {
                    candidates: ["media-record-symbolic"]
                }
            }
        },

        "theme.mode": {
            candidates: [
                "night-light-symbolic",
                "weather-clear-night-symbolic"
            ]
        },

        "power.action": {
            states: {
                "lock": {
                    candidates: ["system-lock-screen-symbolic"]
                },
                "suspend": {
                    candidates: [
                        "system-suspend-symbolic",
                        "media-playback-pause-symbolic"
                    ]
                },
                "hibernate": {
                    candidates: [
                        "system-suspend-hibernate-symbolic",
                        "weather-clear-night-symbolic",
                        "system-suspend-symbolic"
                    ]
                },
                "logout": {
                    candidates: ["system-log-out-symbolic"]
                },
                "reboot": {
                    candidates: [
                        "system-reboot-symbolic",
                        "view-refresh-symbolic"
                    ]
                },
                "shutdown": {
                    candidates: [
                        "system-shutdown-symbolic",
                        "system-log-out-symbolic"
                    ]
                }
            }
        },

        "power.profile": {
            states: {
                "saver": {
                    candidates: [
                        "power-profile-power-saver-symbolic",
                        "battery-good-symbolic",
                        "preferences-system-power-symbolic"
                    ]
                },
                "balanced": {
                    candidates: [
                        "power-profile-balanced-symbolic",
                        "preferences-system-power-symbolic"
                    ]
                },
                "performance": {
                    candidates: [
                        "power-profile-performance-symbolic",
                        "utilities-system-monitor-symbolic"
                    ]
                },
                "active": {
                    candidates: [
                        "object-select-symbolic",
                        "emblem-ok-symbolic"
                    ]
                }
            }
        },

        "session.action": {
            states: {
                "lock": {
                    candidates: ["system-lock-screen-symbolic"]
                },
                "suspend": {
                    candidates: [
                        "system-suspend-symbolic",
                        "media-playback-pause-symbolic"
                    ]
                },
                "logout": {
                    candidates: ["system-log-out-symbolic"]
                },
                "reboot": {
                    candidates: ["system-reboot-symbolic"]
                },
                "shutdown": {
                    candidates: ["system-shutdown-symbolic"]
                }
            }
        },

        "calendar.action": {
            states: {
                "add": {
                    candidates: ["list-add-symbolic"]
                },
                "close": {
                    candidates: ["window-close-symbolic"]
                }
            }
        },

        "calendar.nav": {
            states: {
                "previous": {
                    candidates: ["go-previous-symbolic"]
                },
                "next": {
                    candidates: ["go-next-symbolic"]
                }
            }
        },

        "navigation.next": {
            candidates: ["go-next-symbolic"]
        },

        "network.wifi.signal": {
            states: {
                "excellent": {
                    candidates: ["network-wireless-signal-excellent-symbolic"]
                },
                "good": {
                    candidates: ["network-wireless-signal-good-symbolic"]
                },
                "ok": {
                    candidates: ["network-wireless-signal-ok-symbolic"]
                },
                "weak": {
                    candidates: ["network-wireless-signal-weak-symbolic"]
                }
            }
        },

        "network.wifi.connection": {
            states: {
                "connected": {
                    candidates: [
                        "network-wireless-signal-excellent-symbolic"
                    ]
                },
                "disconnected": {
                    candidates: [
                        "network-wireless-offline-symbolic"
                    ]
                }
            }
        },

        "network.vpn.connection": {
            states: {
                "active": {
                    candidates: ["network-vpn-symbolic"]
                },
                "inactive": {
                    candidates: ["network-vpn-symbolic"]
                }
            }
        },

        "bluetooth.device": {
            states: {
                "connected": {
                    candidates: ["bluetooth-active-symbolic"]
                },
                "disconnected": {
                    candidates: [
                        "bluetooth-disconnected-symbolic",
                        "bluetooth-active-symbolic"
                    ]
                }
            }
        },

        "bluetooth.scan": {
            states: {
                "idle": {
                    candidates: ["view-refresh-symbolic"]
                },
                "active": {
                    candidates: ["view-refresh-symbolic"]
                }
            }
        },

        "action.reveal": {
            states: {
                "hidden": {
                    candidates: ["view-conceal-symbolic"]
                },
                "revealed": {
                    candidates: ["view-reveal-symbolic"]
                }
            }
        },

        "action.forget": {
            candidates: ["edit-delete-symbolic"]
        },

        "action.trust": {
            states: {
                "trusted": {
                    candidates: [
                        "object-select-symbolic",
                        "emblem-ok-symbolic",
                        "preferences-system-privacy-symbolic"
                    ]
                },
                "untrusted": {
                    candidates: [
                        "preferences-system-privacy-symbolic",
                        "dialog-password-symbolic"
                    ]
                }
            }
        },

        "action.save": {
            candidates: [
                "document-save-symbolic",
                "emblem-ok-symbolic",
                "preferences-system-symbolic"
            ]
        },

        "selection": {
            states: {
                "selected": {
                    candidates: [
                        "object-select-symbolic",
                        "emblem-ok-symbolic"
                    ]
                },
                "empty": {
                    candidates: []
                }
            }
        },

        "network.wifi": {
            candidates: [
                "network-wireless-signal-excellent-symbolic"
            ]
        },

        "network.ethernet": {
            candidates: [
                "network-wired-symbolic"
            ]
        },

        "network.vpn": {
            candidates: [
                "network-vpn-symbolic"
            ]
        },

        "bluetooth": {
            candidates: [
                "bluetooth-active-symbolic"
            ]
        },

        "notifications": {
            candidates: [
                "preferences-system-notifications-symbolic"
            ]
        },

        "notifications.dnd": {
            candidates: [
                "notifications-disabled-symbolic"
            ]
        },

        "settings": {
            candidates: [
                "preferences-system-symbolic"
            ]
        },

        "settings.appearance": {
            candidates: [
                "applications-graphics-symbolic",
                "preferences-desktop-appearance-symbolic",
                "preferences-desktop-wallpaper-symbolic"
            ],
            scale: 0.84
        },

        "settings.keyboard": {
            candidates: [
                "preferences-desktop-keyboard-shortcuts-symbolic",
                "input-keyboard-symbolic"
            ]
        },

        "settings.tools": {
            candidates: [
                "applications-utilities-symbolic",
                "preferences-system-symbolic"
            ]
        },

        "settings.dependencies": {
            candidates: [
                "applications-system-symbolic"
            ]
        },

        "settings.about": {
            candidates: [
                "preferences-system-details-symbolic",
                "help-about-symbolic"
            ]
        }
    })

    function canonical(name) {
        if (!name)
            return ""

        return aliases[name] !== undefined
            ? aliases[name]
            : name
    }

    function firstAvailable(names) {
        if (!names)
            return ""

        for (let i = 0; i < names.length; ++i) {
            const name = canonical(names[i])

            if (name !== "" && Quickshell.iconPath(name, true) !== "")
                return name
        }

        return ""
    }

    function spec(role, state, requestedName) {
        const roleSpec =
            role !== "" && roles[role] !== undefined
                ? roles[role]
                : null

        let stateSpec = null

        if (roleSpec
                && roleSpec.states !== undefined
                && state !== ""
                && roleSpec.states[state] !== undefined)
            stateSpec = roleSpec.states[state]

        let candidates = []

        if (stateSpec && stateSpec.candidates)
            candidates = stateSpec.candidates
        else if (roleSpec && roleSpec.candidates)
            candidates = roleSpec.candidates
        else if (requestedName !== "")
            candidates = [requestedName]

        const name = firstAvailable(candidates)

        const tune =
            name !== "" && tuning[name] !== undefined
                ? tuning[name]
                : null

        let scale = baseScale
        let opacity = baseOpacity

        if (tune && tune.scale !== undefined)
            scale = tune.scale
        if (tune && tune.opacity !== undefined)
            opacity = tune.opacity

        if (roleSpec && roleSpec.scale !== undefined)
            scale = roleSpec.scale
        if (roleSpec && roleSpec.opacity !== undefined)
            opacity = roleSpec.opacity

        if (stateSpec && stateSpec.scale !== undefined)
            scale = stateSpec.scale
        if (stateSpec && stateSpec.opacity !== undefined)
            opacity = stateSpec.opacity

        return {
            name: name,
            scale: scale,
            opacity: opacity
        }
    }

    function source(role, requestedName) {
        const s = spec(role, "", requestedName)

        return s.name !== ""
            ? Quickshell.iconPath(s.name, true)
            : ""
    }
}
