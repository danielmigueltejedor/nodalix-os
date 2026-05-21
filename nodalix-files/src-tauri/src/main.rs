#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod files;
mod folder_customization;
mod localsend;
mod platform;
mod sidebar;

use platform::app_start;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::thread;
use tauri::{Emitter, Manager, WindowEvent};

fn socket_path() -> PathBuf {
    let user = std::env::var("UID")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "default".into());
    let profile = if cfg!(debug_assertions) {
        "dev"
    } else {
        "release"
    };
    std::env::temp_dir().join(format!("nodalix-files-{user}-{profile}.sock"))
}

fn launch_path_from_args(args: &[String]) -> Option<String> {
    args.iter()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .cloned()
}

fn send_to_existing_instance(message: &str) -> bool {
    let path = socket_path();
    let Ok(mut stream) = UnixStream::connect(path) else {
        return false;
    };
    stream.write_all(message.as_bytes()).is_ok()
}

fn bind_single_instance_socket() -> std::io::Result<UnixListener> {
    let path = socket_path();
    match UnixListener::bind(&path) {
        Ok(listener) => Ok(listener),
        Err(first_error) => {
            if UnixStream::connect(&path).is_ok() {
                return Err(first_error);
            }
            let _ = std::fs::remove_file(&path);
            UnixListener::bind(path)
        }
    }
}

fn show_main_window(app: &tauri::AppHandle, path: Option<String>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
    if let Some(path) = path {
        let _ = app.emit("nodalix-open-path", path);
    }
}

fn spawn_single_instance_server(app: tauri::AppHandle, listener: UnixListener) {
    thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let mut raw = String::new();
            let _ = stream.read_to_string(&mut raw);
            let message = raw.trim();
            if message == "quit" {
                app.exit(0);
                continue;
            }
            let path = message
                .strip_prefix("open\t")
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned);
            show_main_window(&app, path);
        }
    });
}

fn main() {
    app_start();
    platform::debug_log("main");
    let args: Vec<String> = std::env::args().collect();
    let background = args.iter().any(|arg| arg == "--background");
    let quit = args.iter().any(|arg| arg == "--quit");
    let hide_on_close = !cfg!(debug_assertions);
    let launch_path = launch_path_from_args(&args);

    if quit {
        let _ = send_to_existing_instance("quit\n");
        return;
    }

    let listener = match bind_single_instance_socket() {
        Ok(listener) => Some(listener),
        Err(_) => {
            if background {
                return;
            }
            let message = launch_path
                .as_ref()
                .map(|path| format!("open\t{path}\n"))
                .unwrap_or_else(|| "show\n".into());
            let _ = send_to_existing_instance(&message);
            return;
        }
    };

    tauri::Builder::default()
        .setup(move |app| {
            if let Some(listener) = listener {
                spawn_single_instance_server(app.handle().clone(), listener);
            }
            if background {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            Ok(())
        })
        .on_window_event(move |window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if hide_on_close {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            platform::get_platform_info,
            platform::quit_app,
            files::startup_bundle,
            files::get_special_dirs,
            files::list_directory,
            files::list_disks,
            files::list_network_locations,
            files::copy_text_to_clipboard,
            files::connect_network_location,
            files::open_path,
            files::open_with,
            files::get_open_with_apps,
            files::create_folder,
            files::create_document,
            files::rename_path,
            files::trash_path,
            files::trash_paths,
            files::delete_paths_permanently,
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
            sidebar::set_sidebar_icon,
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
