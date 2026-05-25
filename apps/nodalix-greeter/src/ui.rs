use crate::{greetd, logging, theme, users};
use adw::prelude::AdwApplicationWindowExt;
use chrono::{Datelike, Local, Timelike};
use gtk::{gdk, glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    path::{Path, PathBuf},
    rc::Rc,
    sync::mpsc,
    thread,
    time::Duration,
};

#[derive(Clone, Debug)]
pub enum GreeterMode {
    Demo,
    Greetd { socket: String },
    MissingGreetdSocket,
}

enum AuthEvent {
    Started,
    Failed(greetd::AuthOutcome),
}

pub fn build(
    app: &adw::Application,
    mode: GreeterMode,
    config: theme::GreeterConfig,
    mut greeter_users: Vec<users::GreeterUser>,
) {
    load_css(&config);

    if greeter_users.is_empty() {
        greeter_users = users::demo_users();
    }

    let selected_index = Rc::new(Cell::new(selected_user_index(
        &greeter_users,
        &config.default_user,
    )));
    let greeter_users = Rc::new(greeter_users);

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Nodalix Greeter")
        .default_width(1366)
        .default_height(768)
        .build();
    window.fullscreen();

    let root = gtk::Overlay::new();
    root.add_css_class("greeter-root");
    window.set_content(Some(&root));

    let background = gtk::Picture::for_filename(&config.background);
    background.set_content_fit(gtk::ContentFit::Cover);
    background.add_css_class("background");
    root.set_child(Some(&background));

    let dim = gtk::Box::new(gtk::Orientation::Vertical, 0);
    dim.add_css_class("background-dim");
    root.add_overlay(&dim);

    match mode.clone() {
        GreeterMode::MissingGreetdSocket => build_error_screen(&root),
        GreeterMode::Demo | GreeterMode::Greetd { .. } => {
            build_login_screen(&root, &window, mode, config, greeter_users, selected_index);
        }
    }

    window.present();
}

fn load_css(config: &theme::GreeterConfig) {
    let provider = gtk::CssProvider::new();
    let css = format!("{}\n{}", theme::load_css(), config.css_overrides());
    provider.load_from_string(&css);

    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn build_error_screen(root: &gtk::Overlay) {
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 12);
    panel.set_halign(gtk::Align::Center);
    panel.set_valign(gtk::Align::Center);
    panel.add_css_class("error-panel");

    let title = gtk::Label::new(Some("No se pudo iniciar el greeter"));
    title.add_css_class("error-title");

    let message = gtk::Label::new(Some(
        "GREETD_SOCK no está definido. Ejecuta con --demo para previsualizar la interfaz fuera de greetd.",
    ));
    message.set_wrap(true);
    message.set_justify(gtk::Justification::Center);
    message.add_css_class("error-message");

    panel.append(&title);
    panel.append(&message);
    root.add_overlay(&panel);
}

fn build_login_screen(
    root: &gtk::Overlay,
    window: &adw::ApplicationWindow,
    mode: GreeterMode,
    config: theme::GreeterConfig,
    greeter_users: Rc<Vec<users::GreeterUser>>,
    selected_index: Rc<Cell<usize>>,
) {
    let center = gtk::Box::new(gtk::Orientation::Vertical, 10);
    center.set_halign(gtk::Align::Center);
    center.set_valign(gtk::Align::Center);
    center.set_margin_top(24);
    center.set_margin_bottom(24);
    center.set_margin_start(24);
    center.set_margin_end(24);
    center.add_css_class("center-stack");

    let clock = gtk::Label::new(None);
    clock.add_css_class("clock");
    let date = gtk::Label::new(None);
    date.add_css_class("date");
    let brand = build_brand_widget(&config);
    let subtitle = gtk::Label::new(Some("Inicia sesión para continuar"));
    subtitle.add_css_class("subtitle");

    let password = gtk::Entry::new();
    password.set_placeholder_text(Some("Contraseña"));
    password.set_visibility(false);
    gtk::prelude::EntryExt::set_alignment(&password, 0.5);
    password.set_width_chars(24);
    password.add_css_class("password-pill");

    let error = gtk::Label::new(None);
    error.add_css_class("auth-error");
    error.set_opacity(0.0);

    center.append(&clock);
    center.append(&date);
    center.append(&brand);
    center.append(&subtitle);
    center.append(&password);
    center.append(&error);
    root.add_overlay(&center);

    let hint = gtk::Label::new(Some("Pulsa Enter para iniciar sesión · Esc para cancelar"));
    hint.set_halign(gtk::Align::Center);
    hint.set_valign(gtk::Align::End);
    hint.set_margin_bottom(30);
    hint.add_css_class("hint");
    root.add_overlay(&hint);

    let loading = build_loading_overlay(root);
    loading.container.set_visible(false);

    let user_button = gtk::Button::new();
    user_button.set_halign(gtk::Align::Start);
    user_button.set_valign(gtk::Align::End);
    user_button.set_margin_start(34);
    user_button.set_margin_bottom(28);
    user_button.add_css_class("user-button");
    root.add_overlay(&user_button);

    let user_content = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    user_content.set_valign(gtk::Align::Center);
    let user_avatar_slot = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let user_text = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let user_display = gtk::Label::new(None);
    user_display.set_xalign(0.0);
    user_display.add_css_class("user-display");
    let user_name = gtk::Label::new(None);
    user_name.set_xalign(0.0);
    user_name.add_css_class("user-name");
    user_text.append(&user_display);
    user_text.append(&user_name);
    user_content.append(&user_avatar_slot);
    user_content.append(&user_text);
    user_button.set_child(Some(&user_content));

    update_user_button(
        &user_avatar_slot,
        &user_display,
        &user_name,
        &greeter_users[selected_index.get()],
    );

    let popover = build_user_popover(
        &user_button,
        greeter_users.clone(),
        selected_index.clone(),
        user_avatar_slot.clone(),
        user_display.clone(),
        user_name.clone(),
    );

    let popover_for_click = popover.clone();
    user_button.connect_clicked(move |_| {
        popover_for_click.popup();
    });

    update_clock(&clock, &date);
    let clock_for_tick = clock.clone();
    let date_for_tick = date.clone();
    glib::timeout_add_local(Duration::from_secs(1), move || {
        update_clock(&clock_for_tick, &date_for_tick);
        glib::ControlFlow::Continue
    });

    let auth_pending = Rc::new(Cell::new(false));
    let auth_receiver: Rc<RefCell<Option<mpsc::Receiver<AuthEvent>>>> = Rc::new(RefCell::new(None));

    {
        let password = password.clone();
        let error = error.clone();
        let loading = loading.clone();
        let greeter_users = greeter_users.clone();
        let selected_index = selected_index.clone();
        let auth_pending = auth_pending.clone();
        let auth_receiver = auth_receiver.clone();
        password.connect_activate(move |entry| {
            if auth_pending.get() {
                return;
            }

            let password_text = entry.text().to_string();
            if password_text.is_empty() {
                show_error(&error, "Contraseña incorrecta");
                return;
            }

            error.set_opacity(0.0);
            auth_pending.set(true);
            entry.set_sensitive(false);

            let user = greeter_users[selected_index.get()].clone();
            logging::log_event(format!("UI state: Authenticating user {}", user.username));
            loading.show(&user, "Iniciando sesión…");

            match mode.clone() {
                GreeterMode::Demo => {
                    let ok = password_text == "nodalix" || password_text == "password";
                    let (tx, rx) = mpsc::channel();
                    *auth_receiver.borrow_mut() = Some(rx);
                    thread::spawn(move || {
                        thread::sleep(Duration::from_millis(700));
                        let event = if ok {
                            AuthEvent::Started
                        } else {
                            AuthEvent::Failed(greetd::AuthOutcome::InvalidPassword)
                        };
                        let _ = tx.send(event);
                    });
                }
                GreeterMode::Greetd { socket } => {
                    let session_command = config.session_command.clone();
                    let (tx, rx) = mpsc::channel();
                    *auth_receiver.borrow_mut() = Some(rx);
                    thread::spawn(move || {
                        let outcome = greetd::authenticate_and_start(
                            &socket,
                            &user.username,
                            password_text,
                            &session_command,
                        );
                        let event = match outcome {
                            greetd::AuthOutcome::Success => AuthEvent::Started,
                            other => AuthEvent::Failed(other),
                        };
                        let _ = tx.send(event);
                    });
                }
                GreeterMode::MissingGreetdSocket => {}
            }
        });
    }

    {
        let password = password.clone();
        let error = error.clone();
        let loading = loading.clone();
        let auth_pending = auth_pending.clone();
        let auth_receiver = auth_receiver.clone();
        glib::timeout_add_local(Duration::from_millis(100), move || {
            let event = {
                let mut receiver_ref = auth_receiver.borrow_mut();
                let Some(receiver) = receiver_ref.as_mut() else {
                    return glib::ControlFlow::Continue;
                };
                receiver.try_recv()
            };

            match event {
                Ok(AuthEvent::Started) => {
                    logging::log_event("UI state: SuccessTransition");
                    loading.set_text("Sesión aceptada · preparando escritorio…");
                    password.set_sensitive(false);
                    *auth_receiver.borrow_mut() = None;
                    glib::ControlFlow::Continue
                }
                Ok(AuthEvent::Failed(outcome)) => {
                    loading.hide();
                    auth_pending.set(false);
                    password.set_sensitive(true);
                    match outcome {
                        greetd::AuthOutcome::InvalidPassword => {
                            logging::log_event("UI state: FailedAuth");
                            password.set_text("");
                            password.grab_focus();
                        }
                        _ => {
                            logging::log_event(format!("UI state: FailedSession ({outcome})"));
                        }
                    }
                    show_error(&error, outcome.ui_message());
                    *auth_receiver.borrow_mut() = None;
                    glib::ControlFlow::Continue
                }
                Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => {
                    logging::log_event("UI state: auth worker disconnected unexpectedly");
                    loading.hide();
                    auth_pending.set(false);
                    password.set_sensitive(true);
                    show_error(&error, "No se pudo iniciar sesión");
                    *auth_receiver.borrow_mut() = None;
                    glib::ControlFlow::Continue
                }
            }
        });
    }

    let controller = gtk::EventControllerKey::new();
    {
        let popover = popover.clone();
        let password = password.clone();
        let greeter_users = greeter_users.clone();
        let selected_index = selected_index.clone();
        let user_avatar_slot = user_avatar_slot.clone();
        let user_display = user_display.clone();
        let user_name = user_name.clone();
        controller.connect_key_pressed(move |_, key, _, _| match key {
            gdk::Key::Escape => {
                if popover.is_visible() {
                    popover.popdown();
                } else {
                    password.set_text("");
                }
                glib::Propagation::Stop
            }
            gdk::Key::Down => {
                if popover.is_visible() {
                    select_relative_user(
                        1,
                        &greeter_users,
                        &selected_index,
                        &user_avatar_slot,
                        &user_display,
                        &user_name,
                    );
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            }
            gdk::Key::Up => {
                if popover.is_visible() {
                    select_relative_user(
                        -1,
                        &greeter_users,
                        &selected_index,
                        &user_avatar_slot,
                        &user_display,
                        &user_name,
                    );
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            }
            _ => glib::Propagation::Proceed,
        });
    }
    window.add_controller(controller);

    password.grab_focus();
}

fn build_brand_widget(config: &theme::GreeterConfig) -> gtk::Box {
    let brand = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    brand.set_halign(gtk::Align::Center);
    brand.set_valign(gtk::Align::Center);
    brand.add_css_class("brand");

    if let Some(path) = resolve_logo_path(config) {
        let logo = gtk::Image::from_file(path);
        logo.set_pixel_size(42);
        logo.set_size_request(32, 32);
        logo.add_css_class("brand-logo");
        brand.append(&logo);
    }

    let label = gtk::Label::new(Some("Nodalix OS"));
    label.add_css_class("brand-text");
    brand.append(&label);
    brand
}

fn resolve_logo_path(config: &theme::GreeterConfig) -> Option<PathBuf> {
    let configured = config.logo_path.trim();
    let candidates = [
        configured,
        "/etc/nodalix/brand/nodalix-logo-symbol.svg",
        "/usr/share/nodalix/brand/nodalix-logo-symbol.svg",
        "/home/dani/Projects/nodalix-os/assets/brand/nodalix-logo-symbol.svg",
    ];
    candidates
        .iter()
        .filter(|path| !path.is_empty())
        .map(Path::new)
        .find(|path| path.is_file())
        .map(Path::to_path_buf)
}

#[derive(Clone)]
struct LoadingOverlay {
    container: gtk::Box,
    avatar_slot: gtk::Box,
    text: gtk::Label,
}

impl LoadingOverlay {
    fn show(&self, user: &users::GreeterUser, message: &str) {
        replace_child(&self.avatar_slot, avatar_widget(user, 86));
        self.set_text(message);
        self.container.set_visible(true);
    }

    fn set_text(&self, message: &str) {
        self.text.set_text(message);
    }

    fn hide(&self) {
        self.container.set_visible(false);
    }
}

fn build_loading_overlay(root: &gtk::Overlay) -> LoadingOverlay {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 14);
    container.set_halign(gtk::Align::Center);
    container.set_valign(gtk::Align::Center);
    container.add_css_class("loading-overlay");

    let avatar_slot = gtk::Box::new(gtk::Orientation::Vertical, 0);
    avatar_slot.set_halign(gtk::Align::Center);
    let text = gtk::Label::new(Some("Iniciando Nodalix…"));
    text.add_css_class("loading-text");
    let spinner = gtk::Spinner::new();
    spinner.start();
    spinner.add_css_class("loading-spinner");

    container.append(&avatar_slot);
    container.append(&text);
    container.append(&spinner);
    root.add_overlay(&container);

    LoadingOverlay {
        container,
        avatar_slot,
        text,
    }
}

fn build_user_popover(
    button: &gtk::Button,
    greeter_users: Rc<Vec<users::GreeterUser>>,
    selected_index: Rc<Cell<usize>>,
    user_avatar_slot: gtk::Box,
    user_display: gtk::Label,
    user_name: gtk::Label,
) -> gtk::Popover {
    let popover = gtk::Popover::new();
    popover.add_css_class("user-popover");
    popover.set_parent(button);
    popover.set_position(gtk::PositionType::Top);

    let list = gtk::ListBox::new();
    list.add_css_class("user-list");
    list.set_selection_mode(gtk::SelectionMode::None);

    for user in greeter_users.iter() {
        let row = gtk::ListBoxRow::new();
        row.add_css_class("user-row");
        let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row_box.set_margin_top(10);
        row_box.set_margin_bottom(10);
        row_box.set_margin_start(14);
        row_box.set_margin_end(16);
        row_box.append(&avatar_widget(user, 38));

        let text = gtk::Box::new(gtk::Orientation::Vertical, 1);
        let display = gtk::Label::new(Some(&user.display_name));
        display.set_xalign(0.0);
        display.add_css_class("popover-display");
        let username = gtk::Label::new(Some(&user.username));
        username.set_xalign(0.0);
        username.add_css_class("popover-name");
        text.append(&display);
        text.append(&username);
        row_box.append(&text);
        row.set_child(Some(&row_box));
        row.set_activatable(true);
        row.set_selectable(false);
        list.append(&row);
    }

    {
        let popover = popover.clone();
        list.connect_row_activated(move |_, row| {
            let index = row.index();
            if index < 0 {
                return;
            }
            let index = index as usize;
            selected_index.set(index);
            update_user_button(
                &user_avatar_slot,
                &user_display,
                &user_name,
                &greeter_users[index],
            );
            popover.popdown();
        });
    }

    popover.set_child(Some(&list));
    popover
}

fn selected_user_index(users: &[users::GreeterUser], default_user: &str) -> usize {
    if default_user.is_empty() {
        0
    } else {
        users
            .iter()
            .position(|user| user.username == default_user)
            .unwrap_or(0)
    }
}

fn select_relative_user(
    offset: isize,
    greeter_users: &[users::GreeterUser],
    selected_index: &Cell<usize>,
    user_avatar_slot: &gtk::Box,
    user_display: &gtk::Label,
    user_name: &gtk::Label,
) {
    if greeter_users.is_empty() {
        return;
    }

    let len = greeter_users.len() as isize;
    let current = selected_index.get() as isize;
    let next = (current + offset).rem_euclid(len) as usize;
    selected_index.set(next);
    update_user_button(
        user_avatar_slot,
        user_display,
        user_name,
        &greeter_users[next],
    );
}

fn update_user_button(
    avatar_slot: &gtk::Box,
    display: &gtk::Label,
    username: &gtk::Label,
    user: &users::GreeterUser,
) {
    replace_child(avatar_slot, avatar_widget(user, 52));
    display.set_text(&user.display_name);
    username.set_text(&user.username);
}

fn avatar_widget(user: &users::GreeterUser, size: i32) -> gtk::Widget {
    if let Some(path) = &user.avatar {
        let picture = gtk::Picture::for_filename(path);
        picture.set_size_request(size, size);
        picture.set_content_fit(gtk::ContentFit::Cover);
        picture.add_css_class("avatar");
        picture.upcast()
    } else {
        let label = gtk::Label::new(Some(&users::initials(&user.display_name)));
        label.set_size_request(size, size);
        label.add_css_class("avatar");
        label.add_css_class("avatar-fallback");
        label.upcast()
    }
}

fn replace_child(container: &gtk::Box, child: gtk::Widget) {
    while let Some(existing) = container.first_child() {
        container.remove(&existing);
    }
    container.append(&child);
}

fn update_clock(clock: &gtk::Label, date: &gtk::Label) {
    let now = Local::now();
    clock.set_text(&format!("{:02}:{:02}", now.hour(), now.minute()));
    date.set_text(&format!(
        "{}, {} de {}",
        weekday_es(now.weekday()),
        now.day(),
        month_es(now.month())
    ));
}

fn weekday_es(day: chrono::Weekday) -> &'static str {
    match day {
        chrono::Weekday::Mon => "lunes",
        chrono::Weekday::Tue => "martes",
        chrono::Weekday::Wed => "miércoles",
        chrono::Weekday::Thu => "jueves",
        chrono::Weekday::Fri => "viernes",
        chrono::Weekday::Sat => "sábado",
        chrono::Weekday::Sun => "domingo",
    }
}

fn month_es(month: u32) -> &'static str {
    match month {
        1 => "enero",
        2 => "febrero",
        3 => "marzo",
        4 => "abril",
        5 => "mayo",
        6 => "junio",
        7 => "julio",
        8 => "agosto",
        9 => "septiembre",
        10 => "octubre",
        11 => "noviembre",
        12 => "diciembre",
        _ => "",
    }
}

fn show_error(label: &gtk::Label, message: &str) {
    label.set_text(message);
    label.set_opacity(1.0);
}
