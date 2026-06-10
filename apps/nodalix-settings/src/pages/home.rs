use crate::{
    pages,
    system::{bluetooth, info, power, updates, users, wifi},
    widgets::{
        avatar_widget, confirm_destructive, initials_from_name, run_bg, window_ancestor,
        StatusStrip, TogglePill,
    },
};
use gtk::prelude::*;
use nodalix_system_actions::SystemAction;
use std::rc::Rc;

pub fn build_home_page() -> gtk::Widget {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 22);
    page.add_css_class("page");
    page.add_css_class("home-page");
    page.set_margin_top(28);
    page.set_margin_bottom(28);
    page.set_margin_start(32);
    page.set_margin_end(32);

    let current = users::current_user();
    let status = StatusStrip::new();
    page.append(&home_hero(&current));
    page.append(&quick_actions_section(&status));
    page.append(&status.root);

    page.append(&section_heading("Conectividad"));
    page.append(&connectivity_row(&status));
    page.append(&section_heading("Energía"));
    page.append(&power_section(&status));
    page.append(&section_heading("Sistema"));
    page.append(&system_row(&status));

    pages::scrolled_page(page)
}

fn home_hero(user: &users::LocalUser) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Horizontal, 20);
    card.add_css_class("settings-card");
    card.add_css_class("home-hero");

    card.append(&avatar_widget(
        96,
        user.avatar.as_deref(),
        &initials_from_name(&user.display_name),
    ));

    let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    body.set_valign(gtk::Align::Center);
    body.set_hexpand(true);

    let greeting = gtk::Label::new(Some(&format!("Hola, {}", user.display_name)));
    greeting.set_xalign(0.0);
    greeting.add_css_class("home-hero-title");

    let subtitle = gtk::Label::new(Some(
        "Panel central de Nodalix OS — cuenta, energía, conectividad y sistema.",
    ));
    subtitle.set_xalign(0.0);
    subtitle.set_wrap(true);
    subtitle.add_css_class("page-subtitle");

    let device = gtk::Label::new(None);
    device.set_xalign(0.0);
    device.add_css_class("home-device");
    device.set_text(&format!(
        "{} · Nodalix {} · @{}",
        info::hostname(),
        info::nodalix_version(),
        user.username
    ));

    body.append(&greeting);
    body.append(&subtitle);
    body.append(&device);
    card.append(&body);
    card
}

fn section_heading(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    label.add_css_class("section-heading");
    label
}

fn quick_actions_section(status: &StatusStrip) -> gtk::Box {
    let wrap = gtk::Box::new(gtk::Orientation::Vertical, 12);
    wrap.add_css_class("quick-actions-wrap");

    let title = gtk::Label::new(Some("Acciones del sistema"));
    title.set_xalign(0.0);
    title.add_css_class("card-title");
    wrap.append(&title);

    let hint = gtk::Label::new(Some(
        "Apagado, reinicio, suspensión y bloqueo — siempre accesibles desde el inicio.",
    ));
    hint.set_xalign(0.0);
    hint.add_css_class("home-card-detail");
    wrap.append(&hint);

    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    bar.add_css_class("quick-actions-bar");
    bar.set_halign(gtk::Align::Fill);
    bar.set_homogeneous(true);

    let specs: [(
        &str,
        &str,
        &str,
        bool,
        SystemAction,
        fn() -> Result<(), String>,
    ); 5] = [
        (
            "󰐥",
            "Apagar",
            "Apaga el equipo",
            true,
            SystemAction::Shutdown,
            power::poweroff,
        ),
        (
            "󰑐",
            "Reiniciar",
            "Reinicia el sistema",
            true,
            SystemAction::Reboot,
            power::reboot,
        ),
        (
            "󰒲",
            "Suspender",
            "Suspende la sesión",
            false,
            SystemAction::Suspend,
            power::suspend,
        ),
        (
            "󰌾",
            "Bloquear",
            "Bloquea la pantalla",
            false,
            SystemAction::Lock,
            power::lock_session,
        ),
        (
            "󰍃",
            "Cerrar sesión",
            "Cierra la sesión actual",
            true,
            SystemAction::Logout,
            power::logout_session,
        ),
    ];

    for (icon, label, detail, destructive, system_action, action) in specs {
        let (available, reason) = power::action_available(system_action);

        let tile = gtk::Button::new();
        tile.add_css_class("power-tile");
        if destructive {
            tile.add_css_class("power-tile-destructive");
        }
        tile.set_sensitive(available);
        if let Some(reason) = reason {
            tile.set_tooltip_text(Some(&reason));
        }

        let inner = gtk::Box::new(gtk::Orientation::Vertical, 8);
        inner.set_halign(gtk::Align::Center);

        let icon_label = gtk::Label::new(Some(icon));
        icon_label.add_css_class("power-tile-icon");
        let text = gtk::Label::new(Some(label));
        text.add_css_class("power-tile-label");
        let sub = gtk::Label::new(Some(detail));
        sub.add_css_class("power-tile-detail");
        sub.set_wrap(true);
        sub.set_max_width_chars(16);

        inner.append(&icon_label);
        inner.append(&text);
        inner.append(&sub);
        tile.set_child(Some(&inner));

        let label_owned = label.to_string();
        let status_handle = status.clone();
        tile.connect_clicked({
            let label_owned = label_owned.clone();
            move |btn| {
                if !available {
                    return;
                }
                let parent = window_ancestor(btn);
                let run_action = {
                    let status_handle = status_handle.clone();
                    let label_owned = label_owned.clone();
                    move || {
                        status_handle.set_loading(&format!("{label_owned}…"));
                        let status_done = status_handle.clone();
                        run_bg(action, move |result| match result {
                            Ok(()) => status_done.set_success(&format!("{label_owned} enviado")),
                            Err(err) => status_done.set_error(&err),
                        });
                    }
                };
                if destructive {
                    let run_action = run_action;
                    let status_cancel = status_handle.clone();
                    confirm_destructive(
                        parent.as_ref(),
                        &format!("¿{label_owned}?"),
                        "Confirma esta acción del sistema.",
                        label,
                        move |ok| {
                            if ok {
                                run_action();
                            } else {
                                status_cancel.set_error("Acción cancelada");
                            }
                        },
                    );
                } else {
                    run_action();
                }
            }
        });
        bar.append(&tile);
    }

    wrap.append(&bar);
    wrap
}

fn power_section(status: &StatusStrip) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 14);
    card.add_css_class("settings-card");
    card.set_hexpand(true);
    let title = gtk::Label::new(Some("Modo de energía"));
    title.set_xalign(0.0);
    title.add_css_class("card-title");
    card.append(&title);
    card.append(&pages::power_profile_bar(status));
    card
}

fn connectivity_row(status: &StatusStrip) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    row.add_css_class("home-cards-row");
    row.append(&mini_wifi_card(status));
    row.append(&mini_bluetooth_card(status));
    row
}

fn mini_wifi_card(status: &StatusStrip) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 14);
    card.add_css_class("settings-card");
    card.add_css_class("home-mini-card");
    card.set_hexpand(true);

    let head = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let icon = gtk::Label::new(Some("󰖩"));
    icon.add_css_class("home-card-icon");
    let title = gtk::Label::new(Some("Wi-Fi"));
    title.set_xalign(0.0);
    title.add_css_class("card-title");
    title.set_hexpand(true);
    head.append(&icon);
    head.append(&title);

    let ssid_label = gtk::Label::new(Some("—"));
    ssid_label.set_xalign(0.0);
    ssid_label.add_css_class("home-card-detail");
    ssid_label.set_wrap(true);

    let toggle = Rc::new(TogglePill::new("Activado", "Desactivado"));

    let networks_btn = gtk::Button::with_label("Ver redes");
    networks_btn.add_css_class("pill-button");

    card.append(&head);
    card.append(&ssid_label);

    let controls = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    controls.append(&toggle.root);
    controls.append(&networks_btn);
    card.append(&controls);

    refresh_wifi_ui(&toggle, &ssid_label);

    let status_handle = status.clone();
    let toggle_cb = toggle.clone();
    toggle.connect_toggled({
        let ssid_label = ssid_label.clone();
        let toggle_root = toggle.root.clone();
        let toggle_refresh = toggle_cb.clone();
        move |on| {
            toggle_root.set_sensitive(false);
            status_handle.set_loading(if on {
                "Activando Wi-Fi…"
            } else {
                "Desactivando Wi-Fi…"
            });
            run_bg(move || wifi::set_wifi_radio(on), {
                let status_handle = status_handle.clone();
                let ssid_label = ssid_label.clone();
                let toggle_widget = toggle_root.clone();
                let toggle_refresh = toggle_refresh.clone();
                move |result| {
                    toggle_widget.set_sensitive(true);
                    match result {
                        Ok(()) => {
                            status_handle.set_success(if on {
                                "Wi-Fi activado"
                            } else {
                                "Wi-Fi desactivado"
                            });
                            refresh_wifi_ui(&toggle_refresh, &ssid_label);
                        }
                        Err(err) => {
                            status_handle.set_error(&err);
                            refresh_wifi_ui(&toggle_refresh, &ssid_label);
                        }
                    }
                }
            });
        }
    });

    networks_btn.connect_clicked({
        let status_handle = status.clone();
        move |_| {
            status_handle.set_loading("Abriendo redes…");
            run_bg(wifi::open_wifi_menu, {
                let status_handle = status_handle.clone();
                move |result| match result {
                    Ok(()) => status_handle.set_success("Menú de Wi-Fi abierto"),
                    Err(err) => status_handle.set_error(&err),
                }
            });
        }
    });

    card
}

fn refresh_wifi_ui(toggle: &Rc<TogglePill>, ssid_label: &gtk::Label) {
    let ssid_label = ssid_label.clone();
    let toggle = toggle.clone();
    run_bg(wifi::status, move |result| match result {
        Ok(st) => {
            toggle.set_active(st.radio_on);
            let detail = if st.radio_on {
                if st.connected {
                    format!("Conectado · {}", st.ssid)
                } else {
                    format!("Activado · {}", st.ssid)
                }
            } else {
                "Desactivado".to_string()
            };
            ssid_label.set_text(&detail);
        }
        Err(err) => ssid_label.set_text(&err),
    });
}

fn mini_bluetooth_card(status: &StatusStrip) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 14);
    card.add_css_class("settings-card");
    card.add_css_class("home-mini-card");
    card.set_hexpand(true);

    let head = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let icon = gtk::Label::new(Some("󰂯"));
    icon.add_css_class("home-card-icon");
    let title = gtk::Label::new(Some("Bluetooth"));
    title.set_xalign(0.0);
    title.add_css_class("card-title");
    title.set_hexpand(true);
    head.append(&icon);
    head.append(&title);

    let detail = gtk::Label::new(Some("—"));
    detail.set_xalign(0.0);
    detail.add_css_class("home-card-detail");
    detail.set_wrap(true);

    let toggle = Rc::new(TogglePill::new("Activado", "Desactivado"));

    let devices_btn = gtk::Button::with_label("Dispositivos");
    devices_btn.add_css_class("pill-button");

    card.append(&head);
    card.append(&detail);
    let controls = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    controls.append(&toggle.root);
    controls.append(&devices_btn);
    card.append(&controls);

    refresh_bt_ui(&toggle, &detail);

    let status_handle = status.clone();
    let toggle_cb = toggle.clone();
    toggle.connect_toggled({
        let detail = detail.clone();
        let toggle_root = toggle.root.clone();
        let toggle_refresh = toggle_cb.clone();
        move |on| {
            toggle_root.set_sensitive(false);
            status_handle.set_loading(if on {
                "Activando Bluetooth…"
            } else {
                "Desactivando Bluetooth…"
            });
            run_bg(move || bluetooth::set_bluetooth_power(on), {
                let status_handle = status_handle.clone();
                let detail = detail.clone();
                let toggle_root = toggle_root.clone();
                let toggle_refresh = toggle_refresh.clone();
                move |result| {
                    toggle_root.set_sensitive(true);
                    match result {
                        Ok(()) => {
                            status_handle.set_success(if on {
                                "Bluetooth activado"
                            } else {
                                "Bluetooth desactivado"
                            });
                            refresh_bt_ui(&toggle_refresh, &detail);
                        }
                        Err(err) => {
                            status_handle.set_error(&err);
                            refresh_bt_ui(&toggle_refresh, &detail);
                        }
                    }
                }
            });
        }
    });

    devices_btn.connect_clicked({
        let status_handle = status.clone();
        move |_| {
            status_handle.set_loading("Abriendo Bluetooth…");
            run_bg(bluetooth::open_bt_menu, {
                let status_handle = status_handle.clone();
                move |result| match result {
                    Ok(()) => status_handle.set_success("Menú Bluetooth abierto"),
                    Err(err) => status_handle.set_error(&err),
                }
            });
        }
    });

    card
}

fn refresh_bt_ui(toggle: &Rc<TogglePill>, detail: &gtk::Label) {
    let detail = detail.clone();
    let toggle = toggle.clone();
    run_bg(bluetooth::status, move |result| match result {
        Ok(st) => {
            toggle.set_active(st.powered);
            if !st.powered {
                detail.set_text("Desactivado");
            } else if st.connected_devices.is_empty() {
                detail.set_text("Activado · sin dispositivos conectados");
            } else {
                detail.set_text(&format!("Conectado · {}", st.connected_devices.join(", ")));
            }
        }
        Err(err) => detail.set_text(&err),
    });
}

fn system_row(status: &StatusStrip) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 14);
    card.add_css_class("settings-card");
    card.set_hexpand(true);

    let summary = gtk::Label::new(Some(
        "Pulsa comprobar para ver actualizaciones disponibles.",
    ));
    summary.set_xalign(0.0);
    summary.set_wrap(true);
    summary.add_css_class("home-card-detail");

    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let check_btn = gtk::Button::with_label("Comprobar");
    check_btn.add_css_class("pill-button");
    let update_btn = gtk::Button::with_label("Actualizar");
    update_btn.add_css_class("pill-button");
    update_btn.add_css_class("pill-button-accent");
    bar.append(&check_btn);
    bar.append(&update_btn);

    card.append(&summary);
    card.append(&bar);

    let status_check = status.clone();
    check_btn.connect_clicked({
        let summary = summary.clone();
        let check_btn = check_btn.clone();
        let status_handle = status_check.clone();
        move |_| {
            check_btn.set_sensitive(false);
            status_handle.set_loading("Comprobando actualizaciones…");
            run_bg(updates::check_updates, {
                let status_handle = status_handle.clone();
                let summary = summary.clone();
                let check_btn = check_btn.clone();
                move |result| {
                    check_btn.set_sensitive(true);
                    match result {
                        Ok(report) => {
                            summary.set_text(&report.summary());
                            status_handle.set_success(&report.summary());
                        }
                        Err(err) => status_handle.set_error(&err),
                    }
                }
            });
        }
    });

    let status_update = status.clone();
    update_btn.connect_clicked({
        let status_handle = status_update.clone();
        move |_| {
            status_handle.set_loading("Preparando actualización…");
            run_bg(updates::launch_update_terminal, {
                let status_handle = status_handle.clone();
                move |result| match result {
                    Ok(()) => status_handle.set_success(
                        "Terminal de actualización abierta (se pedirá contraseña si hace falta)",
                    ),
                    Err(err) => status_handle.set_error(&err),
                }
            });
        }
    });

    card
}
