mod greetd;
mod logging;
mod theme;
mod ui;
mod users;

use gtk::prelude::*;

fn main() {
    let raw_args: Vec<String> = std::env::args().collect();
    let demo = raw_args.iter().any(|arg| arg == "--demo");

    // GTK/GApplication rejects unknown arguments, so we remove our custom
    // Nodalix-only flag before handing argv to the GTK application.
    let gtk_args: Vec<String> = raw_args.into_iter().filter(|arg| arg != "--demo").collect();

    let config = theme::GreeterConfig::load_default();

    let mode = if demo {
        ui::GreeterMode::Demo
    } else if let Ok(sock) = std::env::var("GREETD_SOCK") {
        ui::GreeterMode::Greetd { socket: sock }
    } else {
        ui::GreeterMode::MissingGreetdSocket
    };
    logging::log_event(format!(
        "app mode: {}",
        match &mode {
            ui::GreeterMode::Demo => "demo",
            ui::GreeterMode::Greetd { .. } => "greetd",
            ui::GreeterMode::MissingGreetdSocket => "missing socket",
        }
    ));
    logging::log_event(format!("session command: {}", config.session_command));

    let users = if demo {
        users::demo_users()
    } else {
        users::load_human_users()
    };

    let app = adw::Application::builder()
        .application_id("os.nodalix.Greeter")
        .build();

    app.connect_activate(move |app| {
        ui::build(app, mode.clone(), config.clone(), users.clone());
    });

    app.run_with_args(&gtk_args);
}
