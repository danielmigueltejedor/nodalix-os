use crate::pages;
use gtk::prelude::*;

pub fn build_appearance_page() -> gtk::Widget {
    let page = pages::page("Apariencia", "Tema, color de acento y fondos de Nodalix. Los cambios están preparados como placeholders seguros.");
    let card = pages::card("Tema");
    card.append(&pages::row("Modo", "Oscuro activo · Claro pendiente"));
    card.append(&pages::row("Color de acento", "#cba6f7"));
    card.append(&pages::row(
        "Wallpaper",
        "/etc/nodalix/wallpapers/current/wallpaper.png",
    ));
    card.append(&pages::row(
        "Lock screen",
        "/etc/nodalix/wallpapers/current/lockscreen.png",
    ));
    card.append(&pages::row(
        "Greeter",
        "/etc/nodalix/wallpapers/current/greeter-wallpaper.png",
    ));
    card.append(&pages::button_bar(&[
        "Cambiar wallpaper",
        "Generar blur lock/greeter",
    ]));
    page.append(&card);
    pages::scrolled_page(page)
}
