//! Sending files to a nearby device through the session's LocalSend service.

use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use archivador_core::localsend::{self, Device};
use ndl::gtk::{self, gio, glib, prelude::*};
use nodalia_gtk::{self as ndl, ActionRow};

use crate::i18n::{fmt, tr};

/// `GetStatus`: the service's answer as JSON.
async fn status() -> Result<String, glib::Error> {
    let bus = gio::bus_get_future(gio::BusType::Session).await?;
    let reply = bus
        .call_future(
            Some(localsend::BUS_NAME),
            localsend::OBJECT_PATH,
            localsend::INTERFACE,
            "GetStatus",
            None,
            Some(glib::VariantTy::new("(s)").expect("a valid type")),
            gio::DBusCallFlags::NONE,
            3000,
        )
        .await?;
    Ok(reply.child_value(0).get::<String>().unwrap_or_default())
}

fn send(device: &Device, paths: &[PathBuf], toasts: &ndl::ToastOverlay) {
    let unit = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis())
        .to_string();
    let command = localsend::send_command(device, paths, &unit);
    let started = Command::new(&command[0]).args(&command[1..]).spawn();
    let message = match started {
        Ok(_) => fmt("Sending to {}…", &device.alias),
        Err(error) => error.to_string(),
    };
    toasts.add_toast(ndl::Toast::new(&message));
}

/// Opens the list of nearby devices; choosing one sends `paths` to it.
pub fn present(parent: &impl IsA<gtk::Widget>, paths: Vec<PathBuf>, toasts: &ndl::ToastOverlay) {
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.set_margin_start(20);
    body.set_margin_end(20);
    body.set_margin_top(20);
    body.set_margin_bottom(20);
    let title = gtk::Label::new(Some(tr("Nearby devices")));
    title.add_css_class("title-3");
    body.append(&title);
    let spinner = ndl::Spinner::new();
    spinner.set_halign(gtk::Align::Center);
    spinner.set_margin_top(24);
    spinner.set_margin_bottom(24);
    body.append(&spinner);

    let dialog = ndl::Dialog::builder()
        .title(tr("Send with LocalSend"))
        .child(&body)
        .content_width(420)
        .build();
    dialog.present(Some(parent));

    let toasts = toasts.clone();
    glib::spawn_future_local(async move {
        let answer = status().await;
        body.remove(&spinner);
        let devices = match answer {
            Ok(json) => localsend::devices(&json).unwrap_or_default(),
            Err(_) => {
                body.append(&notice(
                    "network-offline-symbolic",
                    tr("LocalSend isn't running"),
                    tr("Switch LocalSend on in Settings."),
                ));
                return;
            }
        };
        if devices.is_empty() {
            body.append(&notice(
                "network-wireless-symbolic",
                tr("No devices nearby"),
                tr("Open LocalSend on the other device, or switch it on in Settings."),
            ));
            return;
        }
        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::None);
        list.add_css_class("boxed-list");
        for device in &devices {
            let row = ActionRow::builder().title(device.label()).build();
            row.set_activatable(true);
            row.add_prefix(&gtk::Image::from_icon_name("computer-symbolic"));
            list.append(&row);
        }
        list.connect_row_activated(move |_, row| {
            if let Some(device) = usize::try_from(row.index())
                .ok()
                .and_then(|i| devices.get(i))
            {
                send(device, &paths, &toasts);
                dialog.force_close();
            }
        });
        body.append(&list);
    });
}

/// A short explanation with an icon, sized for a dialog.
fn notice(icon: &str, title: &str, description: &str) -> gtk::Box {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 8);
    column.set_margin_top(12);
    column.set_margin_bottom(12);
    let image = gtk::Image::from_icon_name(icon);
    image.set_pixel_size(40);
    image.add_css_class("dim-label");
    let heading = gtk::Label::new(Some(title));
    heading.add_css_class("title-4");
    let text = gtk::Label::new(Some(description));
    text.add_css_class("dim-label");
    for label in [&heading, &text] {
        label.set_wrap(true);
        label.set_justify(gtk::Justification::Center);
    }
    column.append(&image);
    column.append(&heading);
    column.append(&text);
    column
}
