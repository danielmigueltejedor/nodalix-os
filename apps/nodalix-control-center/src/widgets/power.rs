use gtk::prelude::*;
use nodalix_system_actions::{
    lock_session, logout_session, poweroff, reboot, suspend as suspend_system, SystemAction,
};

pub fn build(window: &adw::ApplicationWindow) -> gtk::Box {
    let card = super::card("Energía", "󰐥");
    let grid = gtk::Grid::new();
    grid.set_column_spacing(8);
    grid.set_row_spacing(8);

    let lock = action_button("Bloquear", "󰌾");
    lock.set_sensitive(
        nodalix_system_actions::availability()
            .into_iter()
            .find(|a| a.action == SystemAction::Lock)
            .map(|a| a.available)
            .unwrap_or(false),
    );
    lock.connect_clicked(|_| {
        if let Err(err) = lock_session() {
            eprintln!("nodalix-control-center: lock failed: {err}");
        }
    });
    grid.attach(&lock, 0, 0, 1, 1);

    let suspend_btn = action_button("Suspender", "󰤄");
    suspend_btn.set_sensitive(
        nodalix_system_actions::availability()
            .into_iter()
            .find(|a| a.action == SystemAction::Suspend)
            .map(|a| a.available)
            .unwrap_or(false),
    );
    suspend_btn.connect_clicked(|_| {
        if let Err(err) = suspend_system() {
            eprintln!("nodalix-control-center: suspend failed: {err}");
        }
    });
    grid.attach(&suspend_btn, 1, 0, 1, 1);

    for (index, (label, icon, run)) in [
        ("Reiniciar", "󰜉", reboot as fn() -> Result<(), String>),
        ("Apagar", "󰐥", poweroff),
    ]
    .into_iter()
    .enumerate()
    {
        let button = action_button(label, icon);
        let parent = window.clone();
        let label_owned = label.to_string();
        button.connect_clicked(move |_| {
            confirm_power_action(&parent, &label_owned, run);
        });
        grid.attach(&button, index as i32, 1, 1, 1);
    }

    let logout = action_button("Cerrar sesión", "󰍃");
    let parent = window.clone();
    logout.connect_clicked(move |_| {
        confirm_power_action(&parent, "Cerrar sesión", logout_session);
    });
    grid.attach(&logout, 0, 2, 2, 1);

    card.append(&grid);
    card
}

fn action_button(label: &str, icon: &str) -> gtk::Button {
    let button = gtk::Button::with_label(icon);
    button.add_css_class("quick-action");
    button.set_tooltip_text(Some(label));
    button
}

#[allow(deprecated)]
fn confirm_power_action(
    parent: &adw::ApplicationWindow,
    label: &str,
    run: fn() -> Result<(), String>,
) {
    let label_owned = label.to_string();
    let dialog = gtk::MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .message_type(gtk::MessageType::Warning)
        .buttons(gtk::ButtonsType::Cancel)
        .text(format!("¿{label_owned}?"))
        .secondary_text("Esta acción requiere confirmación.")
        .build();
    dialog.add_button(&label_owned, gtk::ResponseType::Accept);
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            if let Err(err) = run() {
                eprintln!("nodalix-control-center: {label_owned} failed: {err}");
            }
        }
        dialog.close();
    });
    dialog.present();
}
