//! Color: the devices colord manages (screens, printers, scanners, cameras), the profile each one
//! uses and importing profiles. Measuring a screen with an instrument is not here.

use std::cell::RefCell;
use std::rc::Rc;

use ajustes_core::color::{self as cc, Profile};
use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, glib, prelude::*},
};

use super::{bus, column, scroller};
use crate::i18n::tr;

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

/// What colord says about one device.
struct Device {
    path: String,
    kind: String,
    name: String,
    assigned: Vec<String>,
}

struct Page {
    devices: ndl::PreferencesGroup,
    rows: RefCell<Vec<gtk::Widget>>,
    toasts: ndl::ToastOverlay,
}

async fn read(connection: &gio::DBusConnection) -> Option<(Vec<Device>, Vec<Profile>)> {
    let paths = |reply: glib::Variant| {
        reply
            .get::<(Vec<glib::variant::ObjectPath>,)>()
            .map(|p| p.0)
    };
    let profile_paths = paths(
        bus::call(connection, COLORD, COLORD_PATH, COLORD, "GetProfiles", None)
            .await
            .ok()?,
    )
    .unwrap_or_default();
    let mut profiles = Vec::new();
    for path in profile_paths {
        let Some(all) = bus::properties(connection, COLORD, &path, PROFILE).await else {
            continue;
        };
        let text = |key: &str| {
            all.get(key)
                .and_then(|v| v.str().map(str::to_owned))
                .unwrap_or_default()
        };
        let title = text("Title");
        if title.is_empty() {
            continue;
        }
        profiles.push(Profile {
            path: path.to_string(),
            title,
            colorspace: text("Colorspace"),
        });
    }
    let device_paths = paths(
        bus::call(connection, COLORD, COLORD_PATH, COLORD, "GetDevices", None)
            .await
            .ok()?,
    )?;
    let mut devices = Vec::new();
    for path in device_paths {
        let Some(all) = bus::properties(connection, COLORD, &path, DEVICE).await else {
            continue;
        };
        let text = |key: &str| {
            all.get(key)
                .and_then(|v| v.str().map(str::to_owned))
                .unwrap_or_default()
        };
        let kind = text("Kind");
        devices.push(Device {
            path: path.to_string(),
            name: format!("{} {}", text("Vendor"), text("Model"))
                .trim()
                .to_owned(),
            assigned: all
                .get("Profiles")
                .and_then(|v| v.get::<Vec<glib::variant::ObjectPath>>())
                .unwrap_or_default()
                .into_iter()
                .map(|p| p.to_string())
                .collect(),
            kind,
        });
    }
    Some((devices, profiles))
}

impl Page {
    fn reload(self: &Rc<Self>) {
        let me = self.clone();
        glib::spawn_future_local(async move {
            let data = match bus::system().await {
                Some(connection) => read(&connection).await,
                None => None,
            };
            for old in me.rows.borrow_mut().drain(..) {
                me.devices.remove(&old);
            }
            let mut rows = Vec::new();
            match data {
                Some((devices, profiles)) if !devices.is_empty() => {
                    for device in devices {
                        let row = me.device_row(&device, &profiles);
                        me.devices.add(&row);
                        rows.push(row);
                    }
                }
                Some(_) => {
                    let row = ndl::ActionRow::builder().title(tr("No devices")).build();
                    me.devices.add(&row);
                    rows.push(row.upcast());
                }
                None => {
                    let row = ndl::ActionRow::builder()
                        .title(tr("The color service is not available"))
                        .build();
                    me.devices.add(&row);
                    rows.push(row.upcast());
                }
            }
            *me.rows.borrow_mut() = rows;
        });
    }

    fn device_row(self: &Rc<Self>, device: &Device, profiles: &[Profile]) -> gtk::Widget {
        let choices = cc::choices(&device.kind, &device.assigned, profiles);
        let title = if device.name.is_empty() {
            tr(kind_name(&device.kind)).to_owned()
        } else {
            device.name.clone()
        };
        let combo = ndl::ComboRow::builder()
            .title(title)
            .subtitle(tr(kind_name(&device.kind)))
            .build();
        combo.add_prefix(&gtk::Image::from_icon_name(kind_icon(&device.kind)));
        if choices.is_empty() {
            combo.set_model(Some(&gtk::StringList::new(&[tr("No profile")])));
            combo.set_sensitive(false);
            return combo.upcast();
        }
        let labels: Vec<&str> = choices.iter().map(|(p, _)| p.title.as_str()).collect();
        combo.set_model(Some(&gtk::StringList::new(&labels)));
        combo.set_selected(0);
        let (me, device_path) = (self.clone(), device.path.clone());
        combo.connect_selected_notify(move |row| {
            let Some((profile, assigned)) = choices.get(row.selected() as usize).cloned() else {
                return;
            };
            if row.selected() == 0 {
                return;
            }
            let (me, device_path) = (me.clone(), device_path.clone());
            glib::spawn_future_local(async move {
                let Some(connection) = bus::system().await else {
                    return;
                };
                let (Some(device), Some(profile_path)) = (
                    bus::object_path(&device_path),
                    bus::object_path(&profile.path),
                ) else {
                    return;
                };
                let mut result = Ok(());
                if !assigned {
                    result = bus::call(
                        &connection,
                        COLORD,
                        &device_path,
                        DEVICE,
                        "AddProfile",
                        Some(&("hard", &profile_path).to_variant()),
                    )
                    .await
                    .map(|_| ());
                }
                if result.is_ok() {
                    result = bus::call(
                        &connection,
                        COLORD,
                        &device_path,
                        DEVICE,
                        "MakeProfileDefault",
                        Some(&(&profile_path,).to_variant()),
                    )
                    .await
                    .map(|_| ());
                }
                let _ = device;
                if let Err(error) = result {
                    me.toasts.add_toast(ndl::Toast::new(error.message()));
                }
                me.reload();
            });
        });
        combo.upcast()
    }
}

/// Copies a profile into the user's profile folder, where colord picks it up.
fn import(page: &Rc<Page>, parent: Option<gtk::Window>) {
    let filter = gtk::FileFilter::new();
    filter.set_name(Some(tr("Color profiles")));
    for pattern in ["*.icc", "*.icm", "*.ICC", "*.ICM"] {
        filter.add_pattern(pattern);
    }
    let filters = gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&filter);
    let dialog = gtk::FileDialog::builder()
        .title(tr("Import a profile"))
        .filters(&filters)
        .build();
    let page = page.clone();
    glib::spawn_future_local(async move {
        let Ok(file) = dialog.open_future(parent.as_ref()).await else {
            return;
        };
        let Some(name) = file.basename().and_then(|n| n.to_str().map(str::to_owned)) else {
            return;
        };
        if !cc::is_profile_file(&name) {
            page.toasts
                .add_toast(ndl::Toast::new(tr("That is not a color profile")));
            return;
        }
        let folder = glib::user_data_dir().join("icc");
        let result = std::fs::create_dir_all(&folder).and_then(|()| {
            // A bare file name, never a path from the picker's text.
            let target = folder.join(std::path::Path::new(&name).file_name().unwrap_or_default());
            match file.path() {
                Some(source) => std::fs::copy(source, target).map(|_| ()),
                None => Err(std::io::Error::other("not a local file")),
            }
        });
        match result {
            Ok(()) => {
                page.toasts
                    .add_toast(ndl::Toast::new(tr("Profile imported")));
                // colord notices the new file a moment later.
                let page = page.clone();
                glib::timeout_add_seconds_local_once(2, move || page.reload());
            }
            Err(error) => page.toasts.add_toast(ndl::Toast::new(&error.to_string())),
        }
    });
}

pub fn build(toasts: &ndl::ToastOverlay) -> gtk::ScrolledWindow {
    let content = column();
    let devices = ndl::PreferencesGroup::builder()
        .title(tr("Devices"))
        .description(tr("Pick the profile each device uses"))
        .build();
    let loading = ndl::ActionRow::builder()
        .title(tr("Looking for devices…"))
        .build();
    devices.add(&loading);
    content.append(&devices);
    let page = Rc::new(Page {
        devices: devices.clone(),
        rows: RefCell::new(vec![loading.upcast()]),
        toasts: toasts.clone(),
    });
    let profiles = ndl::PreferencesGroup::new();
    let add = ndl::ActionRow::builder()
        .title(tr("Import a profile…"))
        .subtitle(tr("Add an ICC file for a screen or printer"))
        .activatable(true)
        .build();
    add.add_prefix(&gtk::Image::from_icon_name("document-open-symbolic"));
    add.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
    {
        let page = page.clone();
        add.connect_activated(move |row| {
            import(&page, row.root().and_downcast::<gtk::Window>());
        });
    }
    profiles.add(&add);
    content.append(&profiles);
    page.reload();
    scroller(&content)
}
