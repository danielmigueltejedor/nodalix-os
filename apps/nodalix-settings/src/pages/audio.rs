use crate::{
    pages,
    system::audio::{self, AudioDevice},
    widgets::{run_bg, StatusStrip},
};
use gtk::prelude::*;
use std::rc::Rc;

pub fn build_audio_page() -> gtk::Widget {
    let page = pages::page(
        "Audio",
        "Salida, entrada, volumen y panel avanzado (PipeWire / PulseAudio).",
    );
    let status = StatusStrip::new();
    page.append(&status.root);

    let sinks_card = pages::card("Altavoces / salida");
    let sinks_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
    sinks_card.append(&sinks_box);
    page.append(&sinks_card);

    let sources_card = pages::card("Micrófonos / entrada");
    let sources_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
    sources_card.append(&sources_box);
    page.append(&sources_card);

    let spatial = pages::card("Audio espacial / 3D");
    spatial.append(&pages::row(
        "Perfil avanzado",
        "Usa el panel de volumen Nodalix o pavucontrol para EQ y rutas 3D.",
    ));
    let open_btn = gtk::Button::with_label("Abrir panel de audio");
    open_btn.add_css_class("pill-button");
    open_btn.add_css_class("pill-button-accent");
    let status_open = status.clone();
    open_btn.connect_clicked(move |_| {
        status_open.set_loading("Abriendo panel…");
        run_bg(audio::open_audio_panel, {
            let status_open = status_open.clone();
            move |result| match result {
                Ok(()) => status_open.set_success("Panel abierto"),
                Err(err) => status_open.set_error(&err),
            }
        });
    });
    spatial.append(&open_btn);
    page.append(&spatial);

    let reload: Rc<dyn Fn()> = Rc::new({
        let sinks_box = sinks_box.clone();
        let sources_box = sources_box.clone();
        let status = status.clone();
        move || {
            status.set_loading("Leyendo dispositivos de audio…");
            run_bg(
                || {
                    let sinks = audio::list_sinks()?;
                    let sources = audio::list_sources()?;
                    Ok((sinks, sources))
                },
                {
                    let sinks_box = sinks_box.clone();
                    let sources_box = sources_box.clone();
                    let status = status.clone();
                    move |result| {
                        clear_box(&sinks_box);
                        clear_box(&sources_box);
                        match result {
                            Ok((sinks, sources)) => {
                                for dev in sinks {
                                    sinks_box.append(&device_row(&dev, true, &status));
                                }
                                for dev in sources {
                                    sources_box.append(&device_row(&dev, false, &status));
                                }
                                status.set_success("Audio actualizado");
                            }
                            Err(err) => status.set_error(&err),
                        }
                    }
                },
            );
        }
    });

    reload();
    pages::scrolled_page(page)
}

fn clear_box(b: &gtk::Box) {
    while let Some(c) = b.first_child() {
        b.remove(&c);
    }
}

fn device_row(dev: &AudioDevice, is_sink: bool, status: &StatusStrip) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Vertical, 8);
    row.add_css_class("audio-device-row");

    let head = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let title = gtk::Label::new(Some(&dev.description));
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.add_css_class("info-label");
    let badge = if dev.is_default { "Predeterminado" } else { "" };
    let badge_label = gtk::Label::new(Some(badge));
    badge_label.add_css_class("info-value");
    head.append(&title);
    head.append(&badge_label);
    row.append(&head);

    let vol_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let vol_label = gtk::Label::new(Some("Volumen"));
    vol_label.set_hexpand(true);
    vol_label.set_xalign(0.0);
    let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 150.0, 1.0);
    scale.set_value(dev.volume_percent as f64);
    scale.set_draw_value(true);
    scale.add_css_class("volume-scale");
    vol_row.append(&vol_label);
    vol_row.append(&scale);
    row.append(&vol_row);

    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let default_btn = gtk::Button::with_label("Usar por defecto");
    default_btn.add_css_class("pill-button");
    let id = dev.id.clone();
    let name = dev.name.clone();
    let status = status.clone();
    let name_done = name.clone();
    let status_default = status.clone();
    default_btn.connect_clicked(move |_| {
        status_default.set_loading("Cambiando dispositivo…");
        let status = status_default.clone();
        let id = id.clone();
        let name_done = name_done.clone();
        run_bg(
            move || {
                if is_sink {
                    audio::set_default_sink(&id)
                } else {
                    audio::set_default_source(&id)
                }
            },
            move |result| match result {
                Ok(()) => status.set_success(&format!("{name_done} es ahora el predeterminado")),
                Err(err) => status.set_error(&err),
            },
        );
    });

    let status_vol = status.clone();
    scale.connect_value_changed({
        let id = dev.id.clone();
        let status = status_vol.clone();
        move |s| {
            let pct = s.value().round() as u32;
            let id = id.clone();
            let status = status.clone();
            run_bg(
                move || audio::set_volume(&id, pct),
                move |result| {
                    if let Err(err) = result {
                        status.set_error(&err);
                    }
                },
            );
        }
    });

    bar.append(&default_btn);
    row.append(&bar);
    row
}
