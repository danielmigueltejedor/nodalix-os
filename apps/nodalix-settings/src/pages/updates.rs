use crate::{pages, system};
use gtk::prelude::*;

pub fn build_updates_page() -> gtk::Widget {
    let page = pages::page(
        "Actualizaciones",
        "Estado de paquetes en modo lectura. No ejecuta actualizaciones.",
    );
    let card = pages::card("Sistema");
    card.append(&pages::row(
        "Pacman pendientes",
        &system::updates::pending_updates(),
    ));
    card.append(&pages::row("AUR", "Placeholder"));
    card.append(&pages::row("Flatpak", "Placeholder"));
    card.append(&pages::row("Última actualización", "Placeholder"));
    card.append(&pages::button_bar(&[
        "Comprobar",
        "Actualizar sistema",
        "Ver historial",
    ]));
    page.append(&card);
    pages::scrolled_page(page)
}
