use crate::{pages, system};
use gtk::prelude::*;

pub fn build_bluetooth_page() -> gtk::Widget {
    let page = pages::page(
        "Bluetooth",
        "Estado del servicio y dispositivos. Esta versión no empareja ni modifica dispositivos.",
    );
    let card = pages::card("Estado");
    card.append(&pages::row(
        "Servicio",
        &system::network::bluetooth_status(),
    ));
    card.append(&pages::row("Interruptor", "Placeholder"));
    card.append(&pages::row(
        "Dispositivos",
        "La lista real se añadirá en una versión posterior",
    ));
    page.append(&card);
    pages::scrolled_page(page)
}
