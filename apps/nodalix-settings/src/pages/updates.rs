use crate::{
    pages,
    system::{info, updates},
    widgets::{run_bg, StatusStrip},
};
use gtk::prelude::*;

pub fn build_updates_page() -> gtk::Widget {
    let page = pages::page(
        "Actualizaciones",
        "Comprueba paquetes pendientes y abre una terminal segura para actualizar (sudo cuando haga falta).",
    );
    let status = StatusStrip::new();
    page.append(&status.root);

    let card = pages::card("Sistema");
    card.append(&pages::row("Nodalix", &info::nodalix_version()));
    let summary = gtk::Label::new(Some("Pulsa «Comprobar» para analizar repositorios."));
    summary.set_xalign(0.0);
    summary.set_wrap(true);
    summary.add_css_class("home-card-detail");
    card.append(&summary);

    let list = gtk::ListBox::new();
    list.add_css_class("network-list");
    card.append(&list);

    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    bar.add_css_class("button-bar");
    let check_btn = gtk::Button::with_label("Comprobar");
    check_btn.add_css_class("pill-button");
    let update_btn = gtk::Button::with_label("Actualizar en terminal");
    update_btn.add_css_class("pill-button");
    update_btn.add_css_class("pill-button-accent");
    bar.append(&check_btn);
    bar.append(&update_btn);
    card.append(&bar);
    page.append(&card);

    let run_check: std::rc::Rc<dyn Fn()> = std::rc::Rc::new({
        let summary = summary.clone();
        let list = list.clone();
        let status = status.clone();
        let check_btn = check_btn.clone();
        move || {
            check_btn.set_sensitive(false);
            status.set_loading("Comprobando actualizaciones…");
            run_bg(
                updates::check_updates,
                {
                    let summary = summary.clone();
                    let list = list.clone();
                    let status = status.clone();
                    let check_btn = check_btn.clone();
                    move |result| {
                        check_btn.set_sensitive(true);
                        while let Some(row) = list.row_at_index(0) {
                            list.remove(&row);
                        }
                        match result {
                            Ok(report) => {
                                summary.set_text(&format!(
                                    "{} paquete(s) en repositorios · {} AUR",
                                    report.pacman_pending, report.aur_pending
                                ));
                                if report.lines.is_empty() {
                                    let row = gtk::ListBoxRow::new();
                                    let label = gtk::Label::new(Some("No hay actualizaciones pendientes"));
                                    label.set_xalign(0.0);
                                    label.set_margin_top(8);
                                    label.set_margin_bottom(8);
                                    label.set_margin_start(10);
                                    row.set_child(Some(&label));
                                    list.append(&row);
                                } else {
                                    for line in report.lines.iter().take(40) {
                                        let row = gtk::ListBoxRow::new();
                                        let label = gtk::Label::new(Some(line));
                                        label.set_xalign(0.0);
                                        label.set_margin_top(8);
                                        label.set_margin_bottom(8);
                                        label.set_margin_start(10);
                                        row.set_child(Some(&label));
                                        list.append(&row);
                                    }
                                }
                                status.set_success(&report.summary());
                            }
                            Err(err) => status.set_error(&err),
                        }
                    }
                },
            );
        }
    });

    check_btn.connect_clicked({
        let run_check = std::rc::Rc::clone(&run_check);
        move |_| run_check()
    });

    update_btn.connect_clicked({
        let status = status.clone();
        move |_| {
            status.set_loading("Abriendo terminal de actualización…");
            run_bg(
                updates::launch_update_terminal,
                {
                    let status = status.clone();
                    move |result| match result {
                        Ok(()) => status.set_success(
                            "Terminal abierta. Se pedirá contraseña para paquetes del sistema.",
                        ),
                        Err(err) => status.set_error(&err),
                    }
                },
            );
        }
    });

    pages::scrolled_page(page)
}
