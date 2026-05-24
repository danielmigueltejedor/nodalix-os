use crate::{pages, system};
use gtk::prelude::*;

pub fn build_power_page() -> gtk::Widget {
    let page = pages::page(
        "Modo de energía",
        "Perfiles disponibles para rendimiento y batería. Los cambios están desactivados.",
    );
    let card = pages::card("Perfil actual");
    card.append(&pages::row("Activo", &system::power::active_profile()));
    card.append(&pages::row("Rendimiento", "Placeholder"));
    card.append(&pages::row("Equilibrado", "Placeholder"));
    card.append(&pages::row("Ahorro", "Placeholder"));
    page.append(&card);
    pages::scrolled_page(page)
}
