use crate::{pages, system};
use gtk::prelude::*;

pub fn build_displays_page() -> gtk::Widget {
    let page = pages::page(
        "Pantallas",
        "Lectura de monitores Hyprland si está disponible. Controles preparados como placeholders.",
    );
    let card = pages::card("Monitores");
    card.append(&pages::row(
        "Detectados",
        &system::displays::monitor_summary(),
    ));
    card.append(&pages::row("Distribución", "Placeholder"));
    card.append(&pages::row("Escala", "Placeholder"));
    card.append(&pages::row("Frecuencia", "Placeholder"));
    page.append(&card);
    pages::scrolled_page(page)
}
