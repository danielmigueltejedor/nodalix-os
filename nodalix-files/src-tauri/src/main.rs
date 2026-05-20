#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod files;
mod folder_customization;
mod localsend;
mod platform;
mod sidebar;

use platform::app_start;

fn main() {
    app_start();
    platform::debug_log("main");

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            platform::get_platform_info,
            files::startup_bundle,
            files::get_special_dirs,
            files::list_directory,
            files::open_path,
            files::open_with,
            files::get_open_with_apps,
            files::create_folder,
            files::rename_path,
            files::trash_path,
            files::trash_paths,
            files::copy_paths,
            files::cut_paths,
            files::paste_into,
            files::clipboard_has_content,
            files::move_to,
            files::get_properties,
            files::open_terminal_here,
            sidebar::get_sidebar_items,
            sidebar::save_sidebar_items,
            sidebar::pin_sidebar_path,
            sidebar::unpin_sidebar_path,
            sidebar::hide_builtin_sidebar,
            sidebar::show_builtin_sidebar,
            sidebar::rename_sidebar_access,
            sidebar::remove_sidebar_access,
            folder_customization::get_folder_customizations,
            folder_customization::set_folder_color,
            folder_customization::set_folder_icon,
            localsend::get_localsend_info,
            localsend::send_with_localsend,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
