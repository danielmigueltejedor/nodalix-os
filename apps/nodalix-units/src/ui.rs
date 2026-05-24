use adw::prelude::AdwApplicationWindowExt;
use gtk::{gdk, prelude::*};

pub fn build(app: &adw::Application) {
    load_css();
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Nodalix Units")
        .default_width(860)
        .default_height(560)
        .build();
    let root = gtk::Box::new(gtk::Orientation::Vertical, 16);
    root.add_css_class("units-root");
    window.set_content(Some(&root));
    let title = gtk::Label::new(Some("Nodalix Units"));
    title.add_css_class("app-title");
    title.set_xalign(0.0);
    root.append(&title);
    let input = gtk::Entry::new();
    input.set_placeholder_text(Some("10 m + 25 cm"));
    input.add_css_class("expression-entry");
    root.append(&input);
    let grid = gtk::Grid::new();
    grid.set_column_spacing(10);
    grid.set_row_spacing(10);
    for (index, category) in [
        "Length",
        "Area",
        "Volume",
        "Mass",
        "Force",
        "Pressure",
        "Energy",
        "Power",
        "Speed",
        "Temperature",
        "Density",
        "Aviation",
    ]
    .iter()
    .enumerate()
    {
        let label = gtk::Label::new(Some(category));
        label.add_css_class("unit-card");
        grid.attach(&label, (index % 3) as i32, (index / 3) as i32, 1, 1);
    }
    root.append(&grid);
    window.present();
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(include_str!("../data/nodalix-units.css"));
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
