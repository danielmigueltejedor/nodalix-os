use adw::prelude::AdwApplicationWindowExt;
use gtk::{gdk, prelude::*};

pub fn build(app: &adw::Application) {
    load_css();

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Nodalix Bundle Creator")
        .default_width(1080)
        .default_height(700)
        .build();

    let root = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    root.add_css_class("bundle-root");
    window.set_content(Some(&root));

    root.append(&sidebar());
    root.append(&main_panel());

    window.present();
}

fn sidebar() -> gtk::Box {
    let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 10);
    sidebar.add_css_class("bundle-sidebar");
    sidebar.set_size_request(240, -1);

    let title = gtk::Label::new(Some("Bundle Creator"));
    title.add_css_class("app-title");
    title.set_xalign(0.0);
    sidebar.append(&title);

    for item in [
        "Overview",
        "Manifest",
        "Assets",
        "Validation",
        "Preview",
        "Export",
    ] {
        let button = gtk::Button::with_label(item);
        button.add_css_class("nav-button");
        sidebar.append(&button);
    }

    sidebar
}

fn main_panel() -> gtk::Box {
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 16);
    panel.add_css_class("bundle-content");
    panel.set_hexpand(true);

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    for label in ["New Bundle", "Open Bundle", "Validate", "Export"] {
        let button = gtk::Button::with_label(label);
        button.add_css_class("action-button");
        if label == "Validate" {
            button.connect_clicked(|button| {
                let message = match crate::manifest::example_manifest()
                    .and_then(|manifest| crate::validator::validate(&manifest).map(|_| manifest))
                {
                    Ok(manifest) => format!("Valid bundle: {}", manifest.display_name),
                    Err(error) => format!("Validation failed: {error}"),
                };
                button.set_tooltip_text(Some(&message));
                button.set_label(&message);
            });
        }
        header.append(&button);
    }
    panel.append(&header);

    let manifest = crate::manifest::example_manifest().expect("bundled example manifest is valid");
    let card = gtk::Box::new(gtk::Orientation::Vertical, 10);
    card.add_css_class("summary-card");

    let title = gtk::Label::new(Some("Project summary"));
    title.add_css_class("card-title");
    title.set_xalign(0.0);
    card.append(&title);

    for (label, value) in crate::bundle::summary(&manifest) {
        card.append(&info_row(label, &value));
    }
    card.append(&info_row("Validation", "Ready"));
    card.append(&info_row("Preview", crate::preview::preview_status()));
    card.append(&info_row("Export", crate::exporter::export_status()));
    panel.append(&card);
    panel
}

fn info_row(label: &str, value: &str) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.add_css_class("info-row");
    let left = gtk::Label::new(Some(label));
    left.add_css_class("info-label");
    left.set_xalign(0.0);
    left.set_size_request(150, -1);
    let right = gtk::Label::new(Some(value));
    right.add_css_class("info-value");
    right.set_xalign(0.0);
    right.set_wrap(true);
    right.set_hexpand(true);
    row.append(&left);
    row.append(&right);
    row
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(include_str!("../data/nodalix-bundle-creator.css"));
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
