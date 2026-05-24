use crate::{pages, system};
use gtk::prelude::*;

pub fn build_audio_page() -> gtk::Widget {
    let page = pages::page(
        "Audio",
        "Resumen de salida y entrada. No cambia volumen ni dispositivos.",
    );
    let output = pages::card("Salida");
    output.append(&pages::row("Dispositivo", &system::audio::default_sink()));
    output.append(&pages::row("Volumen", "Placeholder"));
    page.append(&output);
    let input = pages::card("Entrada");
    input.append(&pages::row("Micrófono", &system::audio::default_source()));
    input.append(&pages::row("Nivel", "Placeholder"));
    page.append(&input);
    pages::scrolled_page(page)
}
