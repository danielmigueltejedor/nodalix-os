//! The window: a sidebar of pages beside the page that is open.

use ajustes_core::pages::{self, Kind, Page};
use nodalia_gtk::{self as ndl, gtk, gtk::prelude::*};

use crate::i18n::tr;
use crate::legacy;
use crate::pages::{
    about, accessibility, accounts, appearance, applications, bluetooth::BluetoothPage, color,
    datetime, displays, keyboard, localsend::LocalSendPage, multitasking, network::NetworkPage,
    notifications, pointing, power, printers, privacy, region, sharing, sound,
    updates::UpdatesPage, users, wifi::WifiPage,
};

fn sidebar_row(page: &Page) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_widget_name(page.id);
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    content.append(&gtk::Image::from_icon_name(page.icon));
    let label = gtk::Label::new(Some(tr(page.title)));
    label.set_hexpand(true);
    label.set_xalign(0.0);
    content.append(&label);
    if matches!(page.kind, Kind::Legacy(_)) {
        let arrow = gtk::Image::from_icon_name("go-next-symbolic");
        arrow.add_css_class("dim-label");
        content.append(&arrow);
    }
    row.set_child(Some(&content));
    row
}

fn heading(text: &str) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_selectable(false);
    row.set_activatable(false);
    let label = gtk::Label::new(Some(&text.to_uppercase()));
    label.add_css_class("ndl-micro");
    label.set_xalign(0.0);
    label.set_margin_top(12);
    label.set_margin_start(6);
    row.set_child(Some(&label));
    row
}

pub fn present(app: &gtk::Application) {
    if let Some(window) = app.active_window() {
        window.present();
        return;
    }
    let window = ndl::ApplicationWindow::new(app);
    window.set_title(Some(tr("Settings")));
    // `NODALIX_SETTINGS_SIZE=1000x1400` opens a taller window, to see a whole page in one picture.
    let (width, height) = std::env::var("NODALIX_SETTINGS_SIZE")
        .ok()
        .and_then(|v| {
            let (w, h) = v.split_once('x')?;
            Some((w.parse().ok()?, h.parse().ok()?))
        })
        .unwrap_or((1000, 700));
    window.set_default_size(width, height);
    window.set_icon_name(Some("os.nodalix.Settings"));

    let toasts = ndl::ToastOverlay::new();
    let updates = UpdatesPage::new();
    updates.set_toasts(&toasts);
    let localsend = LocalSendPage::new();
    localsend.set_toasts(&toasts);
    let wifi = WifiPage::new();
    wifi.set_toasts(&toasts);
    let network = NetworkPage::new();
    network.set_toasts(&toasts);
    let bluetooth = BluetoothPage::new();
    bluetooth.set_toasts(&toasts);

    // The content area: one stack child per page.
    let stack = gtk::Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    stack.add_named(&appearance::build(&toasts), Some("appearance"));
    stack.add_named(&updates.widget, Some("updates"));
    stack.add_named(&localsend.widget, Some("localsend"));
    stack.add_named(&about::build(), Some("about"));
    stack.add_named(&wifi.widget, Some("wifi"));
    stack.add_named(&network.widget, Some("network"));
    stack.add_named(&bluetooth.widget, Some("bluetooth"));
    stack.add_named(&sharing::build(&toasts), Some("sharing"));
    stack.add_named(&sound::build(), Some("sound"));
    stack.add_named(&power::build(), Some("power"));
    stack.add_named(&keyboard::build(&toasts), Some("keyboard"));
    stack.add_named(&pointing::build(), Some("mouse"));
    stack.add_named(&notifications::build(), Some("notifications"));
    stack.add_named(&multitasking::build(), Some("multitasking"));
    stack.add_named(&privacy::build(), Some("privacy"));
    stack.add_named(&accessibility::build(), Some("accessibility"));
    stack.add_named(&applications::build(), Some("applications"));
    stack.add_named(&region::build(&toasts), Some("region"));
    stack.add_named(&users::build(&toasts), Some("users"));
    stack.add_named(&displays::build(&toasts), Some("display"));
    stack.add_named(&printers::build(&toasts), Some("printers"));
    stack.add_named(&accounts::build(&toasts), Some("online-accounts"));
    stack.add_named(&color::build(&toasts), Some("color"));
    stack.add_named(&datetime::build(&toasts), Some("datetime"));
    for page in pages::all() {
        if let Kind::Legacy(panel) = page.kind {
            stack.add_named(
                &legacy::page(tr(page.title), page.icon, panel),
                Some(page.id),
            );
        }
    }

    let title = ndl::WindowTitle::new(tr("Appearance"), "");
    let list = gtk::ListBox::new();
    list.add_css_class("navigation-sidebar");
    for (section, pages) in pages::SECTIONS {
        if !section.is_empty() {
            list.append(&heading(tr(section)));
        }
        for page in pages.iter() {
            list.append(&sidebar_row(page));
        }
    }
    {
        let (stack, title) = (stack.clone(), title.clone());
        list.connect_row_selected(move |_, row| {
            let Some(row) = row else { return };
            let id = row.widget_name();
            if let Some(page) = pages::find(&id) {
                stack.set_visible_child_name(page.id);
                title.set_title(tr(page.title));
            }
        });
    }
    if let Some(first) = list.row_at_index(0) {
        list.select_row(Some(&first));
    }

    let sidebar_scroll = gtk::ScrolledWindow::builder()
        .child(&list)
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    let sidebar_header = ndl::HeaderBar::new();
    sidebar_header.set_show_end_title_buttons(false);
    let sidebar = ndl::ToolbarView::new();
    sidebar.add_top_bar(&sidebar_header);
    sidebar.set_content(Some(&sidebar_scroll));

    let header = ndl::HeaderBar::new();
    header.set_title_widget(Some(&title));
    let content = ndl::ToolbarView::new();
    content.add_top_bar(&header);
    content.set_content(Some(&stack));

    let split = ndl::OverlaySplitView::new();
    split.set_sidebar(Some(&sidebar));
    split.set_content(Some(&content));
    toasts.set_child(Some(&split));
    window.set_content(Some(&toasts));

    // For screenshots and tests: open on another page.
    if let Some(id) = std::env::var("NODALIX_SETTINGS_PAGE")
        .ok()
        .and_then(|id| pages::find(&id))
    {
        for i in 0.. {
            let Some(row) = list.row_at_index(i) else {
                break;
            };
            if row.widget_name() == id.id {
                list.select_row(Some(&row));
            }
        }
    }

    window.present();
}
