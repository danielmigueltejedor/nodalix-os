use crate::{
    pages,
    system::network::{self, NetConnection, NetDevice},
    widgets::{run_bg, StatusStrip},
};
use gtk::prelude::*;
use std::rc::Rc;

pub fn build_network_page() -> gtk::Widget {
    let page = pages::page(
        "Red",
        "Interfaces, conexiones guardadas, IP y DNS vía NetworkManager.",
    );
    let status = StatusStrip::new();
    page.append(&status.root);

    let summary = pages::card("Resumen");
    summary.append(&pages::row("Direcciones IP", &network::ip_summary()));
    summary.append(&pages::row("DNS", &network::dns_servers()));
    page.append(&summary);

    let devices_card = pages::card("Interfaces");
    let devices_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
    devices_card.append(&devices_box);
    page.append(&devices_card);

    let conn_card = pages::card("Conexiones");
    let conn_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
    conn_card.append(&conn_box);
    page.append(&conn_card);

    let tools = pages::card("Herramientas");
    let editor_btn = gtk::Button::with_label("Editor de conexiones");
    editor_btn.add_css_class("pill-button");
    let status_ed = status.clone();
    editor_btn.connect_clicked(move |_| {
        status_ed.set_loading("Abriendo editor…");
        run_bg(network::open_network_menu, {
            let status_ed = status_ed.clone();
            move |result| match result {
                Ok(()) => status_ed.set_success("Editor abierto"),
                Err(err) => status_ed.set_error(&err),
            }
        });
    });
    tools.append(&editor_btn);
    page.append(&tools);

    let reload: Rc<dyn Fn()> = Rc::new({
        let devices_box = devices_box.clone();
        let conn_box = conn_box.clone();
        let status = status.clone();
        move || {
            status.set_loading("Leyendo red…");
            run_bg(
                || {
                    let devs = network::list_devices()?;
                    let conns = network::list_connections()?;
                    Ok((devs, conns))
                },
                {
                    let devices_box = devices_box.clone();
                    let conn_box = conn_box.clone();
                    let status = status.clone();
                    move |result| {
                        clear(&devices_box);
                        clear(&conn_box);
                        match result {
                            Ok((devs, conns)) => {
                                for d in devs {
                                    devices_box.append(&device_row(&d, &status));
                                }
                                for c in conns {
                                    conn_box.append(&connection_row(&c, &status));
                                }
                                status.set_success("Red actualizada");
                            }
                            Err(err) => status.set_error(&err),
                        }
                    }
                },
            );
        }
    });

    reload();
    pages::scrolled_page(page)
}

fn clear(b: &gtk::Box) {
    while let Some(c) = b.first_child() {
        b.remove(&c);
    }
}

fn device_row(dev: &NetDevice, status: &StatusStrip) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.add_css_class("info-row");

    let text = gtk::Box::new(gtk::Orientation::Vertical, 2);
    text.set_hexpand(true);
    let name = gtk::Label::new(Some(&dev.name));
    name.set_xalign(0.0);
    name.add_css_class("info-label");
    let sub = gtk::Label::new(Some(&format!(
        "{} · {} · {}",
        dev.kind, dev.state, dev.connection
    )));
    sub.set_xalign(0.0);
    sub.add_css_class("info-value");
    text.append(&name);
    text.append(&sub);
    row.append(&text);

    if dev.state == "connected" || dev.state == "connected (externally)" {
        let disc = gtk::Button::with_label("Desconectar");
        disc.add_css_class("pill-button");
        let device = dev.name.clone();
        let status = status.clone();
        disc.connect_clicked(move |_| {
            status.set_loading("Desconectando…");
            let status = status.clone();
            let device = device.clone();
            run_bg(
                move || network::disconnect_device(&device),
                move |r| match r {
                    Ok(()) => status.set_success("Desconectado"),
                    Err(e) => status.set_error(&e),
                },
            );
        });
        row.append(&disc);
    }

    row
}

fn connection_row(conn: &NetConnection, status: &StatusStrip) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.add_css_class("info-row");

    let text = gtk::Box::new(gtk::Orientation::Vertical, 2);
    text.set_hexpand(true);
    let name = gtk::Label::new(Some(&conn.name));
    name.set_xalign(0.0);
    name.add_css_class("info-label");
    let sub = gtk::Label::new(Some(&format!(
        "{} · {} {}",
        conn.kind,
        if conn.active { "activa" } else { "inactiva" },
        conn.device
    )));
    sub.set_xalign(0.0);
    sub.add_css_class("info-value");
    text.append(&name);
    text.append(&sub);
    row.append(&text);

    if !conn.active {
        let up = gtk::Button::with_label("Conectar");
        up.add_css_class("pill-button");
        let cname = conn.name.clone();
        let status = status.clone();
        up.connect_clicked(move |_| {
            status.set_loading("Conectando…");
            let status = status.clone();
            let cname = cname.clone();
            run_bg(
                move || network::activate_connection(&cname),
                move |r| match r {
                    Ok(()) => status.set_success("Conexión activada"),
                    Err(e) => status.set_error(&e),
                },
            );
        });
        row.append(&up);
    }

    row
}
