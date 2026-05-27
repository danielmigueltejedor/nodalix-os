use crate::pages::{self, PageId};
use adw::prelude::AdwApplicationWindowExt;
use gtk::{gdk, prelude::*};

pub fn build(app: &adw::Application) {
    load_css();

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("LixSettings")
        .default_width(1120)
        .default_height(760)
        .build();

    let root = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    root.add_css_class("settings-root");
    window.set_content(Some(&root));

    let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 14);
    sidebar.add_css_class("sidebar");
    sidebar.set_size_request(292, -1);
    root.append(&sidebar);

    let title = gtk::Label::new(Some("LixSettings"));
    title.set_xalign(0.0);
    title.add_css_class("app-title");
    sidebar.append(&title);

    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("Buscar"));
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

    window.present();
}

fn nav_row(label: &str, icon: &str) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.add_css_class("nav-row");
    let body = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    body.set_margin_top(9);
    body.set_margin_bottom(9);
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
