use crate::{pages, system};
use gtk::prelude::*;

pub fn build_wifi_page() -> gtk::Widget {
    let page = pages::page(
        "Wi-Fi",
        "Consulta básica de conexión. No se cambian redes ni contraseñas.",
    );
    let card = pages::card("Conexión");
    card.append(&pages::row("SSID actual", &system::network::current_ssid()));
    card.append(&pages::row(
        "Estado",
        "Lectura mediante nmcli si está disponible",
    ));
    card.append(&pages::row("Redes disponibles", "Placeholder"));
    page.append(&card);
    pages::scrolled_page(page)
}
