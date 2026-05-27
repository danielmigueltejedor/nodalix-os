use crate::{
    pages,
    system::theme::{self, ACCENT_PRESETS, ThemeFile},
    widgets::{run_bg, StatusStrip},
};
use gtk::prelude::*;

pub fn build_appearance_page() -> gtk::Widget {
    let page = pages::page(
        "Apariencia",
        "Color de acento Nodalix y rutas de fondos del sistema.",
    );
    let status = StatusStrip::new();
    page.append(&status.root);

    let current = theme::load_current();
    let card = pages::card("Color de acento");
    let current_label = gtk::Label::new(None);
    current_label.set_xalign(0.0);
    current_label.add_css_class("home-card-detail");
    update_current_label(&current_label, &current);
    card.append(&current_label);

    let swatches = gtk::FlowBox::new();
    swatches.add_css_class("accent-swatches");
    swatches.set_selection_mode(gtk::SelectionMode::None);
    swatches.set_max_children_per_line(5);
    swatches.set_column_spacing(12);
    swatches.set_row_spacing(12);

    for (preset_id, label, hex) in ACCENT_PRESETS {
        let btn = gtk::Button::new();
        btn.add_css_class("accent-swatch");
        btn.set_tooltip_text(Some(label));
        btn.set_has_frame(false);
        let dot = gtk::Box::new(gtk::Orientation::Vertical, 0);
        dot.add_css_class("accent-swatch-dot");
        dot.add_css_class(&format!("accent-{preset_id}"));
        dot.set_size_request(44, 44);
        dot.set_tooltip_text(Some(label));
        let _hex = hex;
        btn.set_child(Some(&dot));

        let status = status.clone();
        let current_label = current_label.clone();
        let preset = preset_id.to_string();
        let btn_weak = btn.downgrade();
        btn.connect_clicked(move |_| {
            status.set_loading(&format!("Aplicando {label}…"));
            if let Some(btn) = btn_weak.upgrade() {
                btn.set_sensitive(false);
            }
            let status = status.clone();
            let current_label = current_label.clone();
            let preset = preset.clone();
            let btn_weak = btn_weak.clone();
            run_bg(
                move || theme::apply_accent_preset(&preset),
                move |result| {
                    if let Some(btn) = btn_weak.upgrade() {
                        btn.set_sensitive(true);
                    }
                    match result {
                        Ok(()) => {
                            update_current_label(&current_label, &theme::load_current());
                            status.set_success(&format!("Acento {label} aplicado"));
                        }
                        Err(err) => status.set_error(&err),
                    }
                },
            );
        });
        swatches.append(&btn);
    }

    card.append(&swatches);
    page.append(&card);

    let paths = pages::card("Fondos");
    paths.append(&pages::row(
        "Wallpaper",
        "/etc/nodalix/wallpapers/current/wallpaper.png",
    ));
    paths.append(&pages::row(
        "Pantalla de bloqueo",
        "/etc/nodalix/wallpapers/current/lockscreen.png",
    ));
    paths.append(&pages::row(
        "Greeter",
        "/etc/nodalix/wallpapers/current/greeter-wallpaper.png",
    ));
    page.append(&paths);

    pages::scrolled_page(page)
}

fn update_current_label(label: &gtk::Label, theme: &ThemeFile) {
    let name = theme.name.as_deref().unwrap_or("Nodalix");
    let hex = theme.hex.as_deref().unwrap_or("#cba6f7");
    label.set_text(&format!("Activo: {name} ({hex})"));
}
