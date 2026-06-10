use crate::{
    pages,
    system::storage::{self, MountPoint},
    widgets::{run_bg, StatusStrip},
};
use gtk::prelude::*;
use std::rc::Rc;

pub fn build_storage_page() -> gtk::Widget {
    let page = pages::page(
        "Almacenamiento",
        "Uso de particiones montadas y acceso rápido al explorador.",
    );
    let status = StatusStrip::new();
    page.append(&status.root);

    let card = pages::card("Sistemas de archivos");
    let mounts_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
    card.append(&mounts_box);
    page.append(&card);

    let reload: Rc<dyn Fn()> = Rc::new({
        let mounts_box = mounts_box.clone();
        let status = status.clone();
        move || {
            status.set_loading("Leyendo discos…");
            run_bg(storage::list_mounts, {
                let mounts_box = mounts_box.clone();
                let status = status.clone();
                move |result| {
                    while let Some(c) = mounts_box.first_child() {
                        mounts_box.remove(&c);
                    }
                    match result {
                        Ok(mounts) => {
                            for m in mounts {
                                mounts_box.append(&mount_row(&m, &status));
                            }
                            status.set_success("Almacenamiento actualizado");
                        }
                        Err(err) => status.set_error(&err),
                    }
                }
            });
        }
    });

    reload();
    pages::scrolled_page(page)
}

fn mount_row(m: &MountPoint, status: &StatusStrip) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Vertical, 8);
    row.add_css_class("storage-row");

    let head = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let title = gtk::Label::new(Some(&m.mount));
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.add_css_class("info-label");
    let pct = gtk::Label::new(Some(&format!("{}%", m.use_percent)));
    pct.add_css_class("info-value");
    head.append(&title);
    head.append(&pct);
    row.append(&head);

    let bar = gtk::LevelBar::new();
    bar.set_min_value(0.0);
    bar.set_max_value(100.0);
    bar.set_value(m.use_percent as f64);
    bar.add_css_class("storage-bar");
    row.append(&bar);

    let sub = gtk::Label::new(Some(&format!(
        "{} · usado {} de {} (libre {})",
        m.filesystem, m.used, m.size, m.avail
    )));
    sub.set_xalign(0.0);
    sub.add_css_class("home-card-detail");
    row.append(&sub);

    let open = gtk::Button::with_label("Abrir en archivos");
    open.add_css_class("pill-button");
    let path = m.mount.clone();
    let status = status.clone();
    open.connect_clicked(move |_| {
        status.set_loading("Abriendo…");
        let path = path.clone();
        let status = status.clone();
        run_bg(
            move || storage::open_file_manager(&path),
            move |r| match r {
                Ok(()) => status.set_success("Explorador abierto"),
                Err(e) => status.set_error(&e),
            },
        );
    });
    row.append(&open);
    row
}
