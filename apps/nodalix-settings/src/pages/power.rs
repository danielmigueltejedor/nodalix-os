use crate::{pages, system::power, widgets::StatusStrip};
use gtk::prelude::*;

pub fn build_power_page() -> gtk::Widget {
    let page = pages::page(
        "Modo de energía",
        "Perfil de rendimiento del sistema vía powerprofilesctl.",
    );
    let status = StatusStrip::new();
    page.append(&status.root);

    let card = pages::card("Perfil activo");
    card.append(&pages::row("Actual", &power::active_profile()));
    card.append(&pages::power_profile_bar(&status));
    page.append(&card);

    pages::scrolled_page(page)
}
