pub mod appearance;
pub mod audio;
pub mod bluetooth;
pub mod displays;
pub mod home;
pub mod network;
pub mod power;
pub mod session;
pub mod storage;
pub mod updates;
pub mod users;
pub mod wifi;

use gtk::prelude::*;

#[derive(Clone, Copy)]
pub enum PageId {
    Home,
    Users,
    Appearance,
    Bluetooth,
    Wifi,
    Network,
    Updates,
    Power,
    Displays,
    Audio,
    Storage,
    Session,
}

impl PageId {
    pub fn name(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Users => "users",
            Self::Appearance => "appearance",
            Self::Bluetooth => "bluetooth",
            Self::Wifi => "wifi",
            Self::Network => "network",
            Self::Updates => "updates",
            Self::Power => "power",
            Self::Displays => "displays",
            Self::Audio => "audio",
            Self::Storage => "storage",
            Self::Session => "session",
        }
    }
}

pub struct PageSpec {
    pub id: PageId,
    pub label: &'static str,
    pub icon: &'static str,
    pub build: fn() -> gtk::Widget,
}

pub fn catalog() -> Vec<PageSpec> {
    vec![
        PageSpec {
            id: PageId::Home,
            label: "Inicio",
            icon: "󰍛",
            build: home::build_home_page,
        },
        PageSpec {
            id: PageId::Users,
            label: "Usuarios",
            icon: "󰀄",
            build: users::build_users_page,
        },
        PageSpec {
            id: PageId::Appearance,
            label: "Apariencia",
            icon: "󰸌",
            build: appearance::build_appearance_page,
        },
        PageSpec {
            id: PageId::Bluetooth,
            label: "Bluetooth",
            icon: "󰂯",
            build: bluetooth::build_bluetooth_page,
        },
        PageSpec {
            id: PageId::Wifi,
            label: "Wi-Fi",
            icon: "󰖩",
            build: wifi::build_wifi_page,
        },
        PageSpec {
            id: PageId::Network,
            label: "Red",
            icon: "󰈀",
            build: network::build_network_page,
        },
        PageSpec {
            id: PageId::Updates,
            label: "Actualizaciones",
            icon: "󰚰",
            build: updates::build_updates_page,
        },
        PageSpec {
            id: PageId::Power,
            label: "Modo de energía",
            icon: "󰌪",
            build: power::build_power_page,
        },
        PageSpec {
            id: PageId::Displays,
            label: "Pantallas",
            icon: "󰍹",
            build: displays::build_displays_page,
        },
        PageSpec {
            id: PageId::Audio,
            label: "Audio",
            icon: "󰕾",
            build: audio::build_audio_page,
        },
        PageSpec {
            id: PageId::Storage,
            label: "Almacenamiento",
            icon: "󰋊",
            build: storage::build_storage_page,
        },
        PageSpec {
            id: PageId::Session,
            label: "Sesión",
            icon: "󰗽",
            build: session::build_session_page,
        },
    ]
}

pub fn page(title: &str, subtitle: &str) -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 18);
    page.add_css_class("page");
    page.set_margin_top(34);
    page.set_margin_bottom(34);
    page.set_margin_start(36);
    page.set_margin_end(36);

    let title_label = gtk::Label::new(Some(title));
    title_label.set_xalign(0.0);
    title_label.add_css_class("page-title");
    page.append(&title_label);

    let subtitle_label = gtk::Label::new(Some(subtitle));
    subtitle_label.set_xalign(0.0);
    subtitle_label.set_wrap(true);
    subtitle_label.add_css_class("page-subtitle");
    page.append(&subtitle_label);
    page
}

pub fn scrolled_page(content: gtk::Box) -> gtk::Widget {
    let scrolled = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .overlay_scrolling(true)
        .propagate_natural_height(true)
        .build();
    scrolled.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scrolled.set_child(Some(&content));
    scrolled.upcast()
}

pub fn power_profile_bar(status: &crate::widgets::StatusStrip) -> gtk::Box {
    use crate::system::power;
    use crate::widgets::{run_bg, SegmentedControl, SegmentedOption};

    let wrap = gtk::Box::new(gtk::Orientation::Vertical, 10);
    let options = [
        SegmentedOption {
            id: "performance",
            icon: "󰓅",
            label: "Rendimiento",
        },
        SegmentedOption {
            id: "balanced",
            icon: "󰾆",
            label: "Equilibrado",
        },
        SegmentedOption {
            id: "power-saver",
            icon: "󰁹",
            label: "Ahorro",
        },
    ];
    let seg = SegmentedControl::new(&options);
    let active = power::active_profile().trim().to_string();
    if power::PROFILES.iter().any(|(id, _)| *id == active.as_str()) {
        seg.set_active_id(&active);
    } else {
        seg.set_active_id("balanced");
    }

    let status = status.clone();
    seg.connect_changed(move |id| {
        let profile = id.to_string();
        status.set_loading("Aplicando perfil de energía…");
        let status = status.clone();
        let profile_done = profile.clone();
        run_bg(
            move || power::set_profile(&profile),
            move |result| match result {
                Ok(()) => status.set_success(&format!("Perfil {profile_done} activo")),
                Err(err) => status.set_error(&err),
            },
        );
    });

    wrap.append(&seg.root);
    wrap
}

pub fn card(title: &str) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 12);
    card.add_css_class("settings-card");
    let label = gtk::Label::new(Some(title));
    label.set_xalign(0.0);
    label.add_css_class("card-title");
    card.append(&label);
    card
}

pub fn row_with_widget(label: &str, widget: &impl IsA<gtk::Widget>) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 18);
    row.add_css_class("info-row");
    let name = gtk::Label::new(Some(label));
    name.set_xalign(0.0);
    name.add_css_class("info-label");
    name.set_hexpand(true);
    row.append(&name);
    row.append(widget);
    row
}

pub fn row(label: &str, value: &str) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 18);
    row.add_css_class("info-row");
    let name = gtk::Label::new(Some(label));
    name.set_xalign(0.0);
    name.add_css_class("info-label");
    name.set_hexpand(true);
    let value_label = gtk::Label::new(Some(value));
    value_label.set_xalign(1.0);
    value_label.set_wrap(true);
    value_label.add_css_class("info-value");
    row.append(&name);
    row.append(&value_label);
    row
}
