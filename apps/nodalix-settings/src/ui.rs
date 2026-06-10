use crate::pages::{self, PageId};
use crate::system::users;
use crate::widgets::{avatar_widget, initials_from_name};
use adw::prelude::AdwApplicationWindowExt;
use gtk::{gdk, prelude::*};

pub fn build(app: &adw::Application) {
    load_css();

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Ajustes de Nodalix")
        .default_width(1180)
        .default_height(780)
        .build();

    let root = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    root.add_css_class("settings-root");
    window.set_content(Some(&root));

    let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 12);
    sidebar.add_css_class("sidebar");
    sidebar.set_size_request(300, -1);
    root.append(&sidebar);

    let title = gtk::Label::new(Some("Ajustes"));
    title.set_xalign(0.0);
    title.add_css_class("app-title");
    sidebar.append(&title);

    let current = users::current_user();
    let profile_card = sidebar_profile_card(
        &current.display_name,
        &current.username,
        current.avatar.as_deref(),
    );
    sidebar.append(&profile_card);

    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("Buscar ajustes"));
    search.add_css_class("sidebar-search");
    sidebar.append(&search);

    let nav_scroll = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .overlay_scrolling(true)
        .build();
    nav_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

    let nav = gtk::ListBox::new();
    nav.add_css_class("nav-list");
    nav.set_selection_mode(gtk::SelectionMode::Single);
    nav_scroll.set_child(Some(&nav));
    sidebar.append(&nav_scroll);

    let content = gtk::Stack::new();
    content.add_css_class("content-stack");
    content.set_hexpand(true);
    content.set_vexpand(true);
    content.set_transition_type(gtk::StackTransitionType::Crossfade);
    content.set_transition_duration(180);
    root.append(&content);

    for page in pages::catalog() {
        let row = nav_row(page.label, page.icon);
        row.set_widget_name(page.id.name());
        nav.append(&row);
        content.add_named(&(page.build)(), Some(page.id.name()));
    }

    let content_for_nav = content.clone();
    nav.connect_row_selected(move |_, row| {
        if let Some(row) = row {
            content_for_nav.set_visible_child_name(row.widget_name().as_str());
        }
    });

    if let Some(row) = nav.row_at_index(0) {
        nav.select_row(Some(&row));
    }
    content.set_visible_child_name(PageId::Home.name());

    let nav_jump = nav.clone();
    let profile_gesture = gtk::GestureClick::new();
    profile_gesture.connect_released(move |_, _, _, _| {
        for i in 0..nav_jump.observe_children().n_items() {
            if let Some(row) = nav_jump.row_at_index(i as i32) {
                if row.widget_name().as_str() == PageId::Users.name() {
                    nav_jump.select_row(Some(&row));
                    break;
                }
            }
        }
    });
    profile_card.add_controller(profile_gesture);

    window.present();
}

fn sidebar_profile_card(
    display_name: &str,
    username: &str,
    avatar: Option<&std::path::Path>,
) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    card.add_css_class("sidebar-profile");
    card.set_margin_top(4);
    card.set_margin_bottom(6);

    card.append(&avatar_widget(
        52,
        avatar,
        &initials_from_name(display_name),
    ));

    let text = gtk::Box::new(gtk::Orientation::Vertical, 2);
    text.set_valign(gtk::Align::Center);
    text.set_hexpand(true);

    let name = gtk::Label::new(Some(display_name));
    name.set_xalign(0.0);
    name.add_css_class("sidebar-profile-name");
    name.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let user = gtk::Label::new(Some(&format!("@{username}")));
    user.set_xalign(0.0);
    user.add_css_class("sidebar-profile-user");

    text.append(&name);
    text.append(&user);
    card.append(&text);

    let chevron = gtk::Label::new(Some("›"));
    chevron.add_css_class("sidebar-profile-chevron");
    card.append(&chevron);

    card
}

fn nav_row(label: &str, icon: &str) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.add_css_class("nav-row");
    let body = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    body.set_margin_top(10);
    body.set_margin_bottom(10);
    body.set_margin_start(12);
    body.set_margin_end(12);

    let icon_label = gtk::Label::new(Some(icon));
    icon_label.add_css_class("nav-icon");
    let text = gtk::Label::new(Some(label));
    text.set_xalign(0.0);
    text.add_css_class("nav-label");
    body.append(&icon_label);
    body.append(&text);
    row.set_child(Some(&body));
    row
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(concat!(
        include_str!("../../../assets/styles/nodalix-fonts.css"),
        include_str!("../data/nodalix-settings.css"),
    ));
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
