use crate::manifest::BundleManifest;

pub fn validate(manifest: &BundleManifest) -> Result<(), String> {
    let allowed = [
        "app",
        "theme",
        "wallpaper-pack",
        "icon-pack",
        "settings-profile",
        "hyprland-config",
        "waybar-config",
        "engineering-template",
        "document-template",
    ];

    if !allowed.contains(&manifest.bundle_type.as_str()) {
        return Err(format!("Unsupported bundle type: {}", manifest.bundle_type));
    }

    if !manifest.version.chars().any(|ch| ch == '.') {
        return Err("Version should use semantic style such as 0.1.0".to_string());
    }

    Ok(())
}
