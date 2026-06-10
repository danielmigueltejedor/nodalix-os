use crate::{
    pages,
    system::{
        profile,
        theme::{self, ThemeFile, ACCENT_PRESETS},
    },
    widgets::{run_bg, StatusStrip},
};
use gtk::prelude::*;

#[derive(Clone, Copy)]
enum VisualField {
    ColorScheme,
    Density,
    CornerRadius,
    Transparency,
}

pub fn build_appearance_page() -> gtk::Widget {
    let page = pages::page(
        "Apariencia",
        "Color de acento, preferencias visuales del perfil y rutas de fondos del sistema.",
    );
    let status = StatusStrip::new();
    page.append(&status.root);

    page.append(&visual_preferences_section(&status));
    page.append(&shell_preferences_section(&status));

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
                move || {
                    theme::apply_accent_preset(&preset)?;
                    profile::update_visual(|visual| visual.accent_preset = preset.clone())
                },
                move |result| {
                    if let Some(btn) = btn_weak.upgrade() {
                        btn.set_sensitive(true);
                    }
                    match result {
                        Ok(_) => {
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

    let paths = pages::card("Fondos del sistema");
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

fn shell_preferences_section(status: &StatusStrip) -> gtk::Box {
    let card = pages::card("Shell de Nodalix");
    let profile_data = profile::load_user_profile();
    let active_bar = profile_data.shell.active_bar;

    let note = gtk::Label::new(Some(
        "Migración gradual: Nodalix Bar puede ser la barra principal, manteniendo Waybar como fallback si falla o si prefieres seguir usando Waybar.",
    ));
    note.set_xalign(0.0);
    note.set_wrap(true);
    note.add_css_class("home-card-detail");
    card.append(&note);

    card.append(&bar_pref_row(&active_bar, status));

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.add_css_class("settings-actions-row");

    for (label, command) in [
        ("Abrir Control Center", "nodalix-control-center"),
        ("Reiniciar barra", "nodalix-session-bar --restart"),
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
                        .spawn()
                        .map(|_| ())
                        .map_err(|e| format!("No se pudo ejecutar {command}: {e}"))
                },
                {
                    let status = status.clone();
                    move |result| match result {
                        Ok(()) => status.set_success("Comando enviado"),
                        Err(err) => status.set_error(&err),
                    }
                },
            );
        });
        actions.append(&button);
    }

    card.append(&actions);
    card
}

fn bar_pref_row(active_bar: &str, status: &StatusStrip) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Vertical, 10);
    row.add_css_class("visual-pref-row");

    let label = gtk::Label::new(Some("Barra principal"));
    label.set_xalign(0.0);
    label.add_css_class("info-label");
    row.append(&label);

    let group = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    group.add_css_class("segmented-group");

    for (option_label, option_value) in [
        ("Nodalix Bar", "nodalix-bar"),
        ("Waybar fallback", "waybar"),
    ] {
        let btn = gtk::ToggleButton::with_label(option_label);
        btn.add_css_class("segmented-option");
        btn.set_active(option_value == active_bar);
        let status = status.clone();
        let value = option_value.to_string();
        btn.connect_toggled(move |button| {
            if !button.is_active() {
                return;
            }
            status.set_loading("Guardando barra principal…");
            let value = value.clone();
            run_bg(
                move || profile::update_shell(|shell| shell.active_bar = value.clone()),
                {
                    let status = status.clone();
                    move |result: Result<std::path::PathBuf, String>| match result {
                        Ok(path) => status.set_success(&format!(
                            "Barra guardada. Reinicia la barra para aplicar ({})",
                            path.display()
                        )),
                        Err(err) => status.set_error(&err),
                    }
                },
            );
        });
        group.append(&btn);
    }

    row.append(&group);
    row
}

fn visual_preferences_section(status: &StatusStrip) -> gtk::Box {
    let card = pages::card("Preferencias visuales");
    let profile_data = profile::load_user_profile();
    let visual = profile_data.visual;

    let note = gtk::Label::new(Some(
        "Guardadas en ~/.config/nodalix/user/profile.json. El acento ya se aplica al sistema; tema, densidad y transparencia se conectarán en fases siguientes.",
    ));
    note.set_xalign(0.0);
    note.set_wrap(true);
    note.add_css_class("home-card-detail");
    card.append(&note);

    card.append(&pref_row(
        "Esquema de color",
        &[
            ("Oscuro", "dark"),
            ("Claro", "light"),
            ("Sistema", "system"),
        ],
        &visual.color_scheme,
        status,
        VisualField::ColorScheme,
    ));
    card.append(&pref_row(
        "Densidad",
        &[
            ("Compacta", "compact"),
            ("Cómoda", "comfortable"),
            ("Amplia", "spacious"),
        ],
        &visual.density,
        status,
        VisualField::Density,
    ));
    card.append(&pref_row(
        "Esquinas",
        &[
            ("Rectas", "sharp"),
            ("Estándar", "standard"),
            ("Redondeadas", "round"),
        ],
        &visual.corner_radius,
        status,
        VisualField::CornerRadius,
    ));
    card.append(&pref_row(
        "Superficies",
        &[
            ("Sólido", "solid"),
            ("Cristal", "glass"),
            ("Mínimo", "minimal"),
        ],
        &visual.transparency,
        status,
        VisualField::Transparency,
    ));

    card
}

fn pref_row(
    title: &str,
    options: &[(&str, &str)],
    active_value: &str,
    status: &StatusStrip,
    field: VisualField,
) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Vertical, 10);
    row.add_css_class("visual-pref-row");

    let label = gtk::Label::new(Some(title));
    label.set_xalign(0.0);
    label.add_css_class("info-label");
    row.append(&label);

    let group = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    group.add_css_class("segmented-group");

    for (option_label, option_value) in options {
        let btn = gtk::ToggleButton::with_label(option_label);
        btn.add_css_class("segmented-option");
        btn.set_active(*option_value == active_value);
        let status = status.clone();
        let title = title.to_string();
        let value = option_value.to_string();
        btn.connect_toggled(move |button| {
            if !button.is_active() {
                return;
            }
            status.set_loading(&format!("Guardando {title}…"));
            let value = value.clone();
            run_bg(
                move || {
                    profile::update_visual(|visual| match field {
                        VisualField::ColorScheme => visual.color_scheme = value.clone(),
                        VisualField::Density => visual.density = value.clone(),
                        VisualField::CornerRadius => visual.corner_radius = value.clone(),
                        VisualField::Transparency => visual.transparency = value.clone(),
                    })
                },
                {
                    let status = status.clone();
                    move |result: Result<std::path::PathBuf, String>| match result {
                        Ok(path) => status
                            .set_success(&format!("Preferencia guardada ({})", path.display())),
                        Err(err) => status.set_error(&err),
                    }
                },
            );
        });
        group.append(&btn);
    }

    row.append(&group);
    row
}

fn update_current_label(label: &gtk::Label, theme: &ThemeFile) {
    let name = theme.name.as_deref().unwrap_or("Nodalix");
    let hex = theme.hex.as_deref().unwrap_or("#cba6f7");
    label.set_text(&format!("Activo: {name} ({hex})"));
}
