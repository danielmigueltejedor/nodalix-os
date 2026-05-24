use serde::Serialize;
use std::time::Instant;

static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

pub fn app_start() -> Instant {
    *START.get_or_init(Instant::now)
}

pub fn debug_enabled() -> bool {
    std::env::var("NODALIX_FILES_DEBUG")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

pub fn debug_log(label: &str) {
    if debug_enabled() {
        let ms = app_start().elapsed().as_secs_f64() * 1000.0;
        eprintln!("[nodalix-files +{ms:.1}ms] {label}");
    }
}

pub fn is_hyprland() -> bool {
    if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
        return true;
    }
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|v| v.to_lowercase().contains("hyprland"))
        .unwrap_or(false)
}

#[derive(Serialize, Clone)]
pub struct PlatformInfo {
    pub is_hyprland: bool,
    pub window_controls: WindowControls,
    pub debug: bool,
}

#[derive(Serialize, Clone)]
pub struct WindowControls {
    pub show_minimize: bool,
    pub show_maximize: bool,
    pub show_close: bool,
}

#[tauri::command]
pub fn get_platform_info() -> PlatformInfo {
    debug_log("get_platform_info");
    let hypr = is_hyprland();
    PlatformInfo {
        is_hyprland: hypr,
        window_controls: WindowControls {
            show_minimize: !hypr,
            show_maximize: !hypr,
            show_close: true,
        },
        debug: debug_enabled(),
    }
}

#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}
