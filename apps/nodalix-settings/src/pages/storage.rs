use crate::{pages, system};
use gtk::prelude::*;

pub fn build_storage_page() -> gtk::Widget {
    let page = pages::page(
        "Almacenamiento",
        "Uso de discos y sistemas montados. No modifica particiones ni montajes.",
    );
    let card = pages::card("Sistemas de archivos");
    card.append(&pages::row(
        "Montajes",
        &system::storage::filesystem_summary(),
    ));
    card.append(&pages::row(
        "Disco raíz",
        "Detectado mediante df si está disponible",
    ));
    card.append(&pages::row("Discos de datos", "Placeholder"));
    page.append(&card);
    pages::scrolled_page(page)
}
