use crate::{
    pages,
    system::bluetooth,
    widgets::{run_bg, StatusStrip},
};
use gtk::glib;
use gtk::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

pub fn build_bluetooth_page() -> gtk::Widget {
    let page = pages::page(
        "Bluetooth",
        "Controla la radio Bluetooth y consulta dispositivos conectados.",
    );
    let status = StatusStrip::new();
    page.append(&status.root);

    let card = pages::card("Estado");
    let power_value = gtk::Label::new(Some("—"));
    power_value.set_xalign(1.0);
    power_value.add_css_class("info-value");
    card.append(&pages::row_with_widget("Radio", &power_value));

    let devices_value = gtk::Label::new(Some("—"));
    devices_value.set_xalign(1.0);
    devices_value.set_wrap(true);
    devices_value.add_css_class("info-value");
    card.append(&pages::row_with_widget("Dispositivos", &devices_value));

    let switch = gtk::Switch::new();
    let syncing = Rc::new(Cell::new(false));
    card.append(&pages::row_with_widget("Bluetooth", &switch));

    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    bar.add_css_class("button-bar");
    let refresh_btn = gtk::Button::with_label("Actualizar");
    refresh_btn.add_css_class("pill-button");
    let menu_btn = gtk::Button::with_label("Abrir dispositivos");
    menu_btn.add_css_class("pill-button");
    bar.append(&refresh_btn);
    bar.append(&menu_btn);
    card.append(&bar);
    page.append(&card);

    let reload: Rc<dyn Fn()> = Rc::new({
        let power_value = power_value.clone();
        let devices_value = devices_value.clone();
        let switch = switch.clone();
        let syncing = syncing.clone();
        let status = status.clone();
        move || {
            status.set_loading("Leyendo Bluetooth…");
            run_bg(bluetooth::status, {
                let power_value = power_value.clone();
                let devices_value = devices_value.clone();
                let switch = switch.clone();
                let syncing = syncing.clone();
                let status = status.clone();
                move |result| match result {
                    Ok(st) => {
                        syncing.set(true);
                        switch.set_active(st.powered);
                        syncing.set(false);
                        power_value.set_text(if st.powered {
                            "Activado"
                        } else {
                            "Desactivado"
                        });
                        let devices_text = if st.connected_devices.is_empty() {
                            "Ninguno".to_string()
                        } else {
                            st.connected_devices.join(", ")
                        };
                        devices_value.set_text(&devices_text);
                        status.set_success("Bluetooth actualizado");
                    }
                    Err(err) => status.set_error(&err),
                }
            });
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
                "Activando Bluetooth…"
            } else {
                "Desactivando Bluetooth…"
            });
            run_bg(move || bluetooth::set_bluetooth_power(on), {
                let status_sw = status_sw.clone();
                let reload = reload.clone();
                move |result| {
                    switch.set_sensitive(true);
                    match result {
                        Ok(()) => {
                            status_sw.set_success(if on {
                                "Bluetooth activado"
                            } else {
                                "Bluetooth desactivado"
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
            status.set_loading("Abriendo menú…");
            run_bg(bluetooth::open_bt_menu, {
                let status = status.clone();
                move |result| match result {
                    Ok(()) => status.set_success("Menú Bluetooth abierto"),
                    Err(err) => status.set_error(&err),
                }
            });
        }
    });

    pages::scrolled_page(page)
}
