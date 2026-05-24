use gtk::prelude::*;

pub fn build(window: &adw::ApplicationWindow) -> gtk::Box {
    let card = super::card("Energía", "󰐥");
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);

    let lock = action_button("Bloquear", "󰌾");
    lock.connect_clicked(|_| {
        if crate::system::command_exists("nodalix-system-lock") {
            let _ = crate::system::run_command("nodalix-system-lock", &[]);
        }
    });
    row.append(&lock);

    let suspend = action_button("Suspender", "󰤄");
    suspend.connect_clicked(|_| {
        if crate::system::command_exists("systemctl") {
            let _ = crate::system::run_command("systemctl", &["suspend"]);
        }
    });
    row.append(&suspend);

    for (label, icon, action) in [("Reiniciar", "󰜉", "reboot"), ("Apagar", "󰐥", "poweroff")] {
        let button = action_button(label, icon);
        let parent = window.clone();
        button.connect_clicked(move |_| confirm_power_action(&parent, label, action));
        row.append(&button);
    }

    card.append(&row);
    card
}

fn action_button(label: &str, icon: &str) -> gtk::Button {
    let button = gtk::Button::with_label(&format!("{icon} {label}"));
    button.add_css_class("quick-action");
    button
}

#[allow(deprecated)]
fn confirm_power_action(parent: &adw::ApplicationWindow, label: &str, action: &str) {
    let dialog = gtk::MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .message_type(gtk::MessageType::Warning)
        .buttons(gtk::ButtonsType::Cancel)
        .text(format!("¿{} el sistema?", label))
        .secondary_text("Esta acción requiere confirmación.")
        .build();
    dialog.add_button(label, gtk::ResponseType::Accept);
    let action = action.to_string();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept && crate::system::command_exists("systemctl") {
            let _ = crate::system::run_command("systemctl", &[&action]);
        }
        dialog.close();
    });
    dialog.present();
}
