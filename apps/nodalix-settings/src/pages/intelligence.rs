use crate::{
    pages,
    widgets::{run_bg, StatusStrip},
};
use gtk::prelude::*;

pub fn build_intelligence_page() -> gtk::Widget {
    let page = pages::page(
        "Intelligence",
        "Asistente local, permisos, índice privado y proveedores IA de Nodalix OS.",
    );
    let status = StatusStrip::new();
    page.append(&status.root);

    let overview = pages::card("Estado");
    overview.append(&pages::row(
        "Modo",
        "Local-first · provider mock/desarrollo",
    ));
    overview.append(&pages::row(
        "Permisos",
        "~/.config/nodalix/privacy/permissions.json",
    ));
    overview.append(&pages::row(
        "Auditoría",
        "~/.local/share/nodalix/privacy/audit.log",
    ));
    overview.append(&pages::row(
        "Índice",
        "~/.local/share/nodalix/assistant/context-index.json",
    ));
    page.append(&overview);

    let controls = pages::card("Controles");
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.add_css_class("settings-actions-row");

    for (label, command) in [
        ("Estado del asistente", "nodalix-assistant status"),
        ("Borrar índice local", "nodalix-assistant index-clear"),
    ] {
        let button = gtk::Button::with_label(label);
        button.add_css_class("settings-action-button");
        let status = status.clone();
        let command = command.to_string();
        button.connect_clicked(move |_| {
            status.set_loading(label);
            let command = command.clone();
            run_bg(
                move || {
                    std::process::Command::new("sh")
                        .arg("-lc")
                        .arg(&command)
                        .output()
                        .map_err(|e| format!("No se pudo ejecutar {command}: {e}"))
                        .and_then(|output| {
                            if output.status.success() {
                                Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
                            } else {
                                Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
                            }
                        })
                },
                {
                    let status = status.clone();
                    move |result: Result<String, String>| match result {
                        Ok(output) if output.is_empty() => status.set_success("Comando completado"),
                        Ok(output) => status.set_success(&output),
                        Err(err) => status.set_error(&err),
                    }
                },
            );
        });
        actions.append(&button);
    }

    controls.append(&actions);
    page.append(&controls);

    let policy = pages::card("Política");
    for text in [
        "No se indexan contactos, correo, archivos ni mensajes sin conectores y permisos explícitos.",
        "Los proveedores externos no reciben datos sensibles sin confirmación futura del usuario.",
        "Las acciones que modifican datos deben pasar por intents con confirmación.",
    ] {
        let label = gtk::Label::new(Some(text));
        label.set_xalign(0.0);
        label.set_wrap(true);
        label.add_css_class("home-card-detail");
        policy.append(&label);
    }
    page.append(&policy);

    pages::scrolled_page(page)
}
