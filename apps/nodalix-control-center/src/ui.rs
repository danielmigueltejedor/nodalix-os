use adw::prelude::AdwApplicationWindowExt;
use gtk::{gdk, prelude::*};

pub fn build(app: &adw::Application) {
    load_css();

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Nodalix Control Center")
        .build();
    crate::shell::window::configure(&window);

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .overlay_scrolling(true)
        .build();

    let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
    root.add_css_class("control-center-root");
    scroll.set_child(Some(&root));
    window.set_content(Some(&scroll));

    let connector_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    connector_row.set_halign(gtk::Align::End);
    let connector = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    connector.add_css_class("cc-island-connector");
    connector.append(&gtk::Label::new(Some("󰖩")));
    connector.append(&gtk::Label::new(Some("󰂯")));
    connector.append(&gtk::Label::new(Some("󰕮")));
    connector_row.append(&connector);
    root.append(&connector_row);

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    header.add_css_class("panel-header");
    let title = gtk::Label::new(Some("Control Center"));
    title.add_css_class("panel-title");
    title.set_xalign(0.0);
    title.set_hexpand(true);
    let close = gtk::Button::with_label("󰅖");
    close.add_css_class("icon-button");
    close.set_tooltip_text(Some("Cerrar"));
    let close_window = window.clone();
    close.connect_clicked(move |_| close_window.close());
    header.append(&title);
    header.append(&close);
    root.append(&header);

    root.append(&crate::widgets::profile::build());

    let grid = gtk::Grid::new();
    grid.add_css_class("cc-grid");
    grid.set_column_spacing(8);
    grid.set_row_spacing(8);
    grid.attach(&crate::widgets::network::build(), 0, 0, 1, 1);
    grid.attach(&crate::widgets::toggles::build(), 1, 0, 1, 1);
    grid.attach(&crate::widgets::audio::build(), 0, 1, 2, 1);
    grid.attach(&crate::widgets::brightness::build(), 0, 2, 2, 1);
    grid.attach(&crate::widgets::media::build(), 0, 3, 2, 1);
    grid.attach(&crate::widgets::quick_actions::build(), 0, 4, 1, 1);
    grid.attach(&crate::widgets::power::build(&window), 1, 4, 1, 1);
    root.append(&grid);

    window.present();
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(concat!(
        include_str!("../../../assets/styles/nodalix-fonts.css"),
        include_str!("../data/nodalix-control-center.css"),
    ));
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
