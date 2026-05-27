use crate::{
    pages,
    system::{bluetooth, info, power, updates, wifi},
    widgets::{
        confirm_destructive, icon_action_button, run_bg, window_ancestor, ActionButton,
        StatusStrip, TogglePill,
    },
};
use gtk::prelude::*;
use std::rc::Rc;

pub fn build_home_page() -> gtk::Widget {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 22);
    page.add_css_class("page");
    page.add_css_class("home-page");
    page.set_margin_top(28);
    page.set_margin_bottom(28);
    page.set_margin_start(32);
    page.set_margin_end(32);

    let header = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let title = gtk::Label::new(Some("LixSettings"));
    title.set_xalign(0.0);
    title.add_css_class("page-title");
    let subtitle = gtk::Label::new(Some(
        "Centro de ajustes de Nodalix — conectividad, apariencia y sistema.",
    ));
    subtitle.set_xalign(0.0);
    subtitle.set_wrap(true);
    subtitle.add_css_class("page-subtitle");
    let device = gtk::Label::new(None);
    device.set_xalign(0.0);
    device.add_css_class("home-device");
    device.set_text(&format!(
        "{} · Nodalix {}",
        info::hostname(),
        info::nodalix_version()
    ));
    header.append(&title);
    header.append(&subtitle);
    header.append(&device);
    page.append(&header);

    let status = StatusStrip::new();
    page.append(&status.root);

    page.append(&quick_actions_section(&status));
    page.append(&section_heading("Conectividad"));
    page.append(&connectivity_row(&status));
    page.append(&section_heading("Energía"));
    page.append(&power_section(&status));
    page.append(&section_heading("Sistema"));
    page.append(&system_row(&status));

    pages::scrolled_page(page)
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

    let title = gtk::Label::new(Some("Acciones rápidas"));
    title.set_xalign(0.0);
    title.add_css_class("card-title");
    wrap.append(&title);

    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    bar.add_css_class("quick-actions-bar");
    bar.set_halign(gtk::Align::Start);

    let specs: [(&str, &str, bool, fn() -> Result<(), String>); 5] = [
        ("󰐥", "Apagar", true, power::poweroff),
        ("󰑐", "Reiniciar", true, power::reboot),
        ("󰒲", "Suspender", false, power::suspend),
        ("󰌾", "Bloquear sesión", false, power::lock_session),
        ("󰍃", "Cerrar sesión", true, power::logout_session),
    ];

    for (icon, tooltip, destructive, action) in specs {
        let ActionButton { button } = icon_action_button(icon, tooltip, destructive);
        let status = status.clone();
        let tooltip_owned = tooltip.to_string();
        button.connect_clicked(move |btn| {
            let parent = window_ancestor(btn);
            if destructive {
                let status = status.clone();
                let heading = format!("¿{tooltip_owned}?");
                let body = format!(
                    "Confirma que quieres {}. Esta acción no se puede deshacer desde aquí.",
                    tooltip_owned.to_lowercase()
                );
                let confirm_label = tooltip_owned.clone();
                let tooltip_run = tooltip_owned.clone();
                confirm_destructive(
                    parent.as_ref(),
                    &heading,
                    &body,
                    &confirm_label,
                    move |ok| {
                        if ok {
                            run_power_action(&status, &tooltip_run, action);
                        }
                    },
                );
            } else {
                run_power_action(&status, &tooltip_owned, action);
            }
        });
        bar.append(&button);
    }

    wrap.append(&bar);
    wrap
}

fn run_power_action(status: &StatusStrip, label: &str, action: fn() -> Result<(), String>) {
    status.set_loading(&format!("{label}…"));
    let status = status.clone();
    let label = label.to_string();
    run_bg(action, move |result| match result {
        Ok(()) => status.set_success(&format!("{label} iniciado")),
        Err(err) => status.set_error(&err),
    });
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
            run_bg(
                move || wifi::set_wifi_radio(on),
                {
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
                },
            );
        }
    });

    networks_btn.connect_clicked({
        let status_handle = status.clone();
        move |_| {
            status_handle.set_loading("Abriendo redes…");
            run_bg(
                wifi::open_wifi_menu,
                {
                    let status_handle = status_handle.clone();
                    move |result| match result {
                        Ok(()) => status_handle.set_success("Menú de Wi-Fi abierto"),
                        Err(err) => status_handle.set_error(&err),
                    }
                },
            );
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
            run_bg(
                move || bluetooth::set_bluetooth_power(on),
                {
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
                },
            );
        }
    });

    devices_btn.connect_clicked({
        let status_handle = status.clone();
        move |_| {
            status_handle.set_loading("Abriendo Bluetooth…");
            run_bg(
                bluetooth::open_bt_menu,
                {
                    let status_handle = status_handle.clone();
                    move |result| match result {
                        Ok(()) => status_handle.set_success("Menú Bluetooth abierto"),
                        Err(err) => status_handle.set_error(&err),
                    }
                },
            );
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
                detail.set_text(&format!(
                    "Conectado · {}",
                    st.connected_devices.join(", ")
                ));
            }
        }
        Err(err) => detail.set_text(&err),
    });
}

fn system_row(status: &StatusStrip) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 14);
    card.add_css_class("settings-card");
    card.set_hexpand(true);

    let summary = gtk::Label::new(Some("Pulsa comprobar para ver actualizaciones disponibles."));
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
            run_bg(
                updates::check_updates,
                {
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
                },
            );
        }
    });

    let status_update = status.clone();
    update_btn.connect_clicked({
        let status_handle = status_update.clone();
        move |_| {
            status_handle.set_loading("Preparando actualización…");
            run_bg(
                updates::launch_update_terminal,
                {
                    let status_handle = status_handle.clone();
                    move |result| match result {
                        Ok(()) => status_handle.set_success(
                            "Terminal de actualización abierta (se pedirá contraseña si hace falta)",
                        ),
                        Err(err) => status_handle.set_error(&err),
                    }
                },
            );
        }
    });

    card
}
