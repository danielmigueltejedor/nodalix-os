mod backends;
mod config;
mod registry;
mod service;
mod tools;

pub use service::{
    convert_file, get_conversion_settings, init_conversion_backends, preview_rename_conversion,
    ConversionSettings, RenameConversionPreview,
};

#[tauri::command]
pub fn preview_rename_with_conversion(
    old_path: String,
    new_name: String,
) -> Result<RenameConversionPreview, String> {
    preview_rename_conversion(&old_path, &new_name)
}

#[tauri::command]
pub fn convert_file_to_path(source_path: String, dest_path: String) -> Result<String, String> {
    convert_file(&source_path, &dest_path)
}

#[tauri::command]
pub fn get_file_conversion_settings() -> ConversionSettings {
    get_conversion_settings()
}
