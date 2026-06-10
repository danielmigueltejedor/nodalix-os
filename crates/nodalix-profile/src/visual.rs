use serde::{Deserialize, Serialize};

/// Visual preferences stored in the user profile.
/// Fields are persisted now; not all are wired to the desktop yet.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct VisualPreferences {
    /// `dark`, `light`, or `system` (future).
    pub color_scheme: String,
    /// Preset id consumed by `nodalix-accent-color` (e.g. `purple`).
    pub accent_preset: String,
    /// `compact`, `comfortable`, or `spacious`.
    pub density: String,
    /// Corner radius preset: `sharp`, `standard`, `round`.
    pub corner_radius: String,
    /// Surface transparency preset: `solid`, `glass`, `minimal`.
    pub transparency: String,
}

impl Default for VisualPreferences {
    fn default() -> Self {
        Self {
            color_scheme: "dark".to_string(),
            accent_preset: "purple".to_string(),
            density: "comfortable".to_string(),
            corner_radius: "standard".to_string(),
            transparency: "glass".to_string(),
        }
    }
}
