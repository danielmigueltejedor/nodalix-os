//! Color: the devices colord manages (screens, printers, scanners, cameras) and the profile each
//! one uses. Assigning or calibrating profiles stays in the previous panel.

use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{glib, prelude::*},
};

use super::{bus, column, gs, scroller};
use crate::i18n::tr;
use crate::rows;

const COLORD: &str = "org.freedesktop.ColorManager";
const COLORD_PATH: &str = "/org/freedesktop/ColorManager";
const DEVICE: &str = "org.freedesktop.ColorManager.Device";
const PROFILE: &str = "org.freedesktop.ColorManager.Profile";

fn kind_icon(kind: &str) -> &'static str {
    match kind {
        "display" => "video-display-symbolic",
        "printer" => "printer-symbolic",
        "scanner" => "scanner-symbolic",
        "camera" | "webcam" => "camera-photo-symbolic",
        _ => "preferences-color-symbolic",
    }
}

fn kind_name(kind: &str) -> &'static str {
    match kind {
        "display" => "Screen",
        "printer" => "Printer",
        "scanner" => "Scanner",
        "camera" | "webcam" => "Camera",
        _ => "Device",
    }
}

pub fn build() -> gtk::ScrolledWindow {
    let content = column();
    let group = ndl::PreferencesGroup::builder()
        .title(tr("Devices"))
        .build();
    let placeholder = ndl::ActionRow::builder()
        .title(tr("Looking for devices…"))
        .build();
    group.add(&placeholder);
    content.append(&group);
    if let Some(more) = gs::group(
        "",
        rows![Some(gs::legacy_link(
            tr("Color profiles"),
            tr("Assign a profile to a device or calibrate it"),
            "color",
        ))],
    ) {
        content.append(&more);
    }

    glib::spawn_future_local(async move {
        let devices = async {
            let connection = bus::system().await?;
            let reply = bus::call(&connection, COLORD, COLORD_PATH, COLORD, "GetDevices", None)
                .await
                .ok()?;
            let (paths,) = reply.get::<(Vec<glib::variant::ObjectPath>,)>()?;
            let mut rows = Vec::new();
            for path in paths {
                let Some(all) = bus::properties(&connection, COLORD, &path, DEVICE).await else {
                    continue;
                };
                let text = |key: &str| {
                    all.get(key)
                        .and_then(|v| v.str().map(str::to_owned))
                        .unwrap_or_default()
                };
                // The first profile is the one in use; its title is the profile's own property.
                let mut profile = String::new();
                if let Some(first) = all
                    .get("Profiles")
                    .and_then(|v| v.get::<Vec<glib::variant::ObjectPath>>())
                    .and_then(|p| p.into_iter().next())
                {
                    if let Some(p) = bus::properties(&connection, COLORD, &first, PROFILE).await {
                        profile = p
                            .get("Title")
                            .and_then(|v| v.str().map(str::to_owned))
                            .unwrap_or_default();
                    }
                }
                rows.push((text("Kind"), text("Vendor"), text("Model"), profile));
            }
            Some(rows)
        }
        .await;
        group.remove(&placeholder);
        match devices {
            Some(list) if !list.is_empty() => {
                for (kind, vendor, model, profile) in list {
                    let name = format!("{vendor} {model}").trim().to_owned();
                    let row = ndl::ActionRow::builder()
                        .title(if name.is_empty() {
                            tr(kind_name(&kind)).to_owned()
                        } else {
                            name
                        })
                        .build();
                    let mut subtitle = tr(kind_name(&kind)).to_owned();
                    if !profile.is_empty() {
                        subtitle.push_str(" · ");
                        subtitle.push_str(&profile);
                    }
                    row.set_subtitle(&subtitle);
                    row.add_prefix(&gtk::Image::from_icon_name(kind_icon(&kind)));
                    group.add(&row);
                }
            }
            Some(_) => group.add(&ndl::ActionRow::builder().title(tr("No devices")).build()),
            None => group.add(
                &ndl::ActionRow::builder()
                    .title(tr("The color service is not available"))
                    .build(),
            ),
        }
    });
    scroller(&content)
}
