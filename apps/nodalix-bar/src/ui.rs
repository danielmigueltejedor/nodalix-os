use adw::prelude::AdwApplicationWindowExt;
use gtk::{gdk, glib, prelude::*};
use std::cell::RefCell;
use std::rc::Rc;

pub fn build(app: &adw::Application) {
    load_css();

    let windows = Rc::new(RefCell::new(Vec::new()));
    rebuild_windows(app, &windows);

    if let Some(display) = gdk::Display::default() {
        let monitors = display.monitors();
        monitors.connect_items_changed({
            let app = app.clone();
            let windows = windows.clone();
            move |_, _, _, _| rebuild_windows(&app, &windows)
        });
    }
}

fn rebuild_windows(app: &adw::Application, windows: &Rc<RefCell<Vec<adw::ApplicationWindow>>>) {
    for window in windows.borrow_mut().drain(..) {
        window.close();
    }

    if let Some(display) = gdk::Display::default() {
        let monitors = display.monitors();
        let count = monitors.n_items();
        if count > 0 {
            let mut built = windows.borrow_mut();
            for index in 0..count {
                let monitor = monitors
                    .item(index)
                    .and_then(|item| item.downcast::<gdk::Monitor>().ok());
                built.push(build_window(app, monitor.as_ref(), index));
            }
            return;
        }
    }

    windows.borrow_mut().push(build_window(app, None, 0));
}

fn build_window(
    app: &adw::Application,
    monitor: Option<&gdk::Monitor>,
    index: u32,
) -> adw::ApplicationWindow {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title(format!("Nodalix Bar {index}"))
        .build();
    crate::shell::layer::configure(&window, monitor);

    let root = gtk::CenterBox::new();
    root.add_css_class("bar-root");
    root.set_hexpand(true);
    root.set_size_request(-1, crate::shell::layer::BAR_HEIGHT);
    window.set_content(Some(&root));
    let active_popover: Rc<RefCell<Option<gtk::Popover>>> = Rc::new(RefCell::new(None));

    let left = island();
    left.set_halign(gtk::Align::Start);
    let settings = gtk::Button::with_label("󰒓");
    settings.add_css_class("bar-button");
    settings.set_tooltip_text(Some("Nodalix Settings"));
    attach_popover(
        &settings,
        &active_popover,
        "Ajustes",
        &[
            ("󰒓", "Nodalix Settings", "nodalix-settings-toggle"),
            ("󰢻", "Command Bar", "nodalix-command-bar-toggle"),
            ("󰉋", "Archivos", "nodalix-files"),
        ],
    );
    left.append(&settings);
    left.append(&crate::modules::workspaces::switcher());
    root.set_start_widget(Some(&left));

    let center = island();
    center.add_css_class("center-island");
    center.set_halign(gtk::Align::Center);
    let date = gtk::Label::new(Some(&crate::modules::center::date_label()));
    date.add_css_class("center-pill");
    center.append(&date);

    let time = gtk::Label::new(Some(&crate::modules::center::time_label()));
    time.add_css_class("center-pill");
    time.add_css_class("time-pill");
    center.append(&time);
    glib::timeout_add_seconds_local(30, {
        let date = date.clone();
        let time = time.clone();
        move || {
            date.set_text(&crate::modules::center::date_label());
            time.set_text(&crate::modules::center::time_label());
            glib::ControlFlow::Continue
        }
    });

    let weather = gtk::Label::new(Some(&crate::modules::center::weather_label()));
    weather.add_css_class("center-pill");
    center.append(&weather);
    glib::timeout_add_seconds_local(300, {
        let weather = weather.clone();
        move || {
            weather.set_text(&crate::modules::center::weather_label());
            glib::ControlFlow::Continue
        }
    });

    let media = gtk::Button::new();
    media.add_css_class("bar-button");
    media.add_css_class("media-button");
    media.set_tooltip_text(Some("Media Player"));
    media.set_has_frame(false);
    refresh_media_button(&media);
    media.connect_clicked(|_| {
        crate::modules::spawn_detached("media-popup", &[]);
    });
    center.append(&media);
    glib::timeout_add_seconds_local(2, {
        let media = media.clone();
        move || {
            refresh_media_button(&media);
            glib::ControlFlow::Continue
        }
    });
    root.set_center_widget(Some(&center));

    let right = island();
    right.set_halign(gtk::Align::End);
    let network = gtk::Button::with_label("󰖩");
    network.add_css_class("bar-button");
    network.set_tooltip_text(Some("Redes WiFi / Ethernet"));
    attach_popover(
        &network,
        &active_popover,
        "Red",
        &[
            ("󰖩", "Wi-Fi", "nodalix-wifi-menu"),
            ("󰈀", "Ethernet", "nm-connection-editor"),
            ("󰒓", "Ajustes de red", "nodalix-settings-toggle"),
        ],
    );
    right.append(&network);

    let bluetooth = gtk::Button::with_label("󰂯");
    bluetooth.add_css_class("bar-button");
    bluetooth.set_tooltip_text(Some("Bluetooth"));
    attach_popover(
        &bluetooth,
        &active_popover,
        "Bluetooth",
        &[
            ("󰂯", "Dispositivos", "nodalix-bt-menu"),
            ("󰂲", "Alternar Bluetooth", "bluetoothctl power toggle"),
            ("󰒓", "Ajustes", "nodalix-settings-toggle"),
        ],
    );
    right.append(&bluetooth);

    let control = gtk::Button::with_label("󰕮");
    control.add_css_class("bar-button");
    control.set_tooltip_text(Some("Control Center"));
    control.connect_clicked({
        let active_popover = active_popover.clone();
        move |_| {
            if let Some(current) = active_popover.borrow_mut().take() {
                current.popdown();
            }
            crate::modules::spawn_detached("nodalix-control-center-toggle", &[]);
        }
    });
    right.append(&control);
    root.set_end_widget(Some(&right));

    window.present();
    window
}

fn refresh_media_button(button: &gtk::Button) {
    if let Some(media) = crate::modules::center::media_label() {
        button.set_label(&format!("|  {media}"));
        button.set_visible(true);
    } else {
        button.set_visible(false);
    }
}

fn attach_popover(
    button: &gtk::Button,
    active_popover: &Rc<RefCell<Option<gtk::Popover>>>,
    title: &str,
    actions: &[(&str, &str, &str)],
) {
    let popover = shell_popover(title, actions);
    popover.set_parent(button);

    button.connect_clicked({
        let popover = popover.clone();
        let active_popover = active_popover.clone();
        move |_| {
            if popover.is_visible() {
                popover.popdown();
                *active_popover.borrow_mut() = None;
                return;
            }

            if let Some(current) = active_popover.borrow_mut().take() {
                current.popdown();
            }
            popover.popup();
            *active_popover.borrow_mut() = Some(popover.clone());
        }
    });
}

fn shell_popover(title: &str, actions: &[(&str, &str, &str)]) -> gtk::Popover {
    let popover = gtk::Popover::new();
    popover.add_css_class("shell-popover");
    popover.set_position(gtk::PositionType::Bottom);
    popover.set_autohide(true);
    popover.set_has_arrow(false);

    let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
    root.add_css_class("shell-popover-root");

    let title = gtk::Label::new(Some(title));
    title.add_css_class("shell-popover-title");
    title.set_xalign(0.0);
    root.append(&title);

    for (icon, label, command) in actions {
        let button = gtk::Button::new();
        button.add_css_class("shell-popover-action");
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let icon_label = gtk::Label::new(Some(icon));
        icon_label.add_css_class("shell-popover-icon");
        let text = gtk::Label::new(Some(label));
        text.set_xalign(0.0);
        text.set_hexpand(true);
        row.append(&icon_label);
        row.append(&text);
        button.set_child(Some(&row));
        button.connect_clicked({
            let command = (*command).to_string();
            let popover = popover.clone();
            move |_| {
                popover.popdown();
                spawn_shell_command(&command);
            }
        });
        root.append(&button);
    }

    popover.set_child(Some(&root));
    popover
}

fn spawn_shell_command(command: &str) {
    let _ = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .spawn();
}

fn island() -> gtk::Box {
    let box_ = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    box_.add_css_class("bar-island");
    box_.set_valign(gtk::Align::Center);
    box_.set_size_request(-1, 32);
    box_
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(concat!(
        include_str!("../../../assets/styles/nodalix-fonts.css"),
        include_str!("../data/nodalix-bar.css"),
    ));
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
