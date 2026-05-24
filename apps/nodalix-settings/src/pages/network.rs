use crate::{pages, system};
use gtk::prelude::*;

pub fn build_network_page() -> gtk::Widget {
    let page = pages::page(
        "Red",
        "Resumen de interfaces, IP y servicios. Configuración de red desactivada en v0.",
    );
    let card = pages::card("Resumen");
    card.append(&pages::row("Interfaces", &system::network::link_summary()));
    card.append(&pages::row(
        "Direcciones IP",
        &system::network::ip_summary(),
    ));
    card.append(&pages::row("DNS", "Placeholder"));
    card.append(&pages::row("VPN / Tailscale", "Placeholder"));
    page.append(&card);
    pages::scrolled_page(page)
}
