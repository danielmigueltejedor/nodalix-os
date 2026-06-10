use crate::{
    pages,
    system::wifi,
    widgets::{run_bg, StatusStrip},
};
use gtk::glib;
use gtk::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

pub fn build_wifi_page() -> gtk::Widget {
    let page = pages::page(
        "Wi-Fi",
        "Activa o desactiva la radio, consulta la red actual y abre el selector de redes.",
    );
    let status = StatusStrip::new();
    page.append(&status.root);

    let card = pages::card("Conexión");
    let ssid_value = gtk::Label::new(Some("—"));
    ssid_value.set_xalign(1.0);
    ssid_value.add_css_class("info-value");
    card.append(&pages::row_with_widget("SSID actual", &ssid_value));

    let state_value = gtk::Label::new(Some("—"));
    state_value.set_xalign(1.0);
    state_value.add_css_class("info-value");
    card.append(&pages::row_with_widget("Estado", &state_value));

    let switch = gtk::Switch::new();
    let syncing = Rc::new(Cell::new(false));
    card.append(&pages::row_with_widget("Wi-Fi", &switch));

    let networks = gtk::ListBox::new();
    networks.add_css_class("network-list");
    card.append(&networks);

    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    bar.add_css_class("button-bar");
    let refresh_btn = gtk::Button::with_label("Actualizar");
    refresh_btn.add_css_class("pill-button");
    let menu_btn = gtk::Button::with_label("Abrir selector");
    menu_btn.add_css_class("pill-button");
    bar.append(&refresh_btn);
    bar.append(&menu_btn);
    card.append(&bar);
    page.append(&card);

    let reload: Rc<dyn Fn()> = Rc::new({
        let ssid_value = ssid_value.clone();
        let state_value = state_value.clone();
        let switch = switch.clone();
        let syncing = syncing.clone();
        let networks = networks.clone();
        let status = status.clone();
        move || {
            status.set_loading("Leyendo Wi-Fi…");
            run_bg(
                || {
                    let st = wifi::status()?;
                    let list = wifi::nearby_networks(12);
                    Ok((st, list))
                },
                {
                    let ssid_value = ssid_value.clone();
                    let state_value = state_value.clone();
                    let switch = switch.clone();
                    let syncing = syncing.clone();
                    let networks = networks.clone();
                    let status = status.clone();
                    move |result: Result<(wifi::WifiStatus, Vec<String>), String>| {
                        while let Some(row) = networks.row_at_index(0) {
                            networks.remove(&row);
                        }
                        match result {
                            Ok((st, list)) => {
                                syncing.set(true);
                                switch.set_active(st.radio_on);
                                syncing.set(false);
                                ssid_value.set_text(&st.ssid);
                                state_value.set_text(if st.radio_on {
                                    if st.connected {
                                        "Conectado"
                                    } else {
                                        "Activado"
                                    }
                                } else {
                                    "Desactivado"
                                });
                                for line in list {
                                    let row = gtk::ListBoxRow::new();
                                    let label = gtk::Label::new(Some(&line));
                                    label.set_xalign(0.0);
                                    label.set_margin_top(8);
                                    label.set_margin_bottom(8);
                                    label.set_margin_start(10);
                                    row.set_child(Some(&label));
                                    networks.append(&row);
                                }
                                status.set_success("Wi-Fi actualizado");
                            }
                            Err(err) => status.set_error(&err),
                        }
                    }
                },
            );
        }
    });

    reload();

    let status_sw = status.clone();
    switch.connect_state_set({
        let syncing = syncing.clone();
        let reload = Rc::clone(&reload);
        move |sw, on| {
            if syncing.get() {
                return glib::Propagation::Proceed;
            }
            let switch = sw.clone();
            switch.set_sensitive(false);
            status_sw.set_loading(if on {
                "Activando Wi-Fi…"
            } else {
                "Desactivando Wi-Fi…"
            });
            run_bg(move || wifi::set_wifi_radio(on), {
                let status_sw = status_sw.clone();
                let reload = reload.clone();
                move |result| {
                    switch.set_sensitive(true);
                    match result {
                        Ok(()) => {
                            status_sw.set_success(if on {
                                "Wi-Fi activado"
                            } else {
                                "Wi-Fi desactivado"
                            });
                            reload();
                        }
                        Err(err) => status_sw.set_error(&err),
                    }
                }
            });
            glib::Propagation::Proceed
        }
    });

    refresh_btn.connect_clicked({
        let reload = Rc::clone(&reload);
        move |_| reload()
    });

    menu_btn.connect_clicked({
        let status = status.clone();
        move |_| {
            status.set_loading("Abriendo selector…");
            run_bg(wifi::open_wifi_menu, {
                let status = status.clone();
                move |result| match result {
                    Ok(()) => status.set_success("Selector abierto"),
                    Err(err) => status.set_error(&err),
                }
            });
        }
    });

    pages::scrolled_page(page)
}
