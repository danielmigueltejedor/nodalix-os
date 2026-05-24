use crate::pages;
use gtk::prelude::*;

pub fn build_session_page() -> gtk::Widget {
    let page = pages::page(
        "Sesión",
        "Acciones grandes para la sesión. Por seguridad están desactivadas en esta versión.",
    );
    let card = pages::card("Acciones");
    let grid = gtk::Grid::new();
    grid.add_css_class("session-grid");
    grid.set_row_spacing(12);
    grid.set_column_spacing(12);
    for (index, label) in [
        "Bloquear",
        "Cerrar sesión",
        "Suspender",
        "Reiniciar",
        "Apagar",
    ]
    .iter()
    .enumerate()
    {
        let button = gtk::Button::with_label(label);
        button.add_css_class("session-button");
        button.connect_clicked(|_| show_disabled_dialog());
        grid.attach(&button, (index % 2) as i32, (index / 2) as i32, 1, 1);
    }
    card.append(&grid);
    page.append(&card);
    pages::scrolled_page(page)
}

fn show_disabled_dialog() {
    let enabled = std::env::var("NODALIX_SETTINGS_ENABLE_SESSION_ACTIONS")
        .ok()
        .as_deref()
        == Some("1");
    let text = if enabled {
        "Acciones reales de sesión pendientes de implementación segura."
    } else {
        "Acción de sesión desactivada en esta versión de desarrollo."
    };
    let window = gtk::Window::builder()
        .title("Nodalix Settings")
        .default_width(420)
        .default_height(140)
        .modal(true)
        .build();
    let body = gtk::Box::new(gtk::Orientation::Vertical, 14);
    body.set_margin_top(22);
    body.set_margin_bottom(22);
    body.set_margin_start(24);
    body.set_margin_end(24);
    let label = gtk::Label::new(Some(text));
    label.set_wrap(true);
    label.add_css_class("dialog-label");
    let close = gtk::Button::with_label("Cerrar");
    close.add_css_class("placeholder-button");
    let window_for_close = window.clone();
    close.connect_clicked(move |_| window_for_close.close());
    body.append(&label);
    body.append(&close);
    window.set_child(Some(&body));
    window.present();
}
