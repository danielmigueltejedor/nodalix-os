//! The window: a sidebar of pages beside the page that is open.

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use ajustes_core::pages::{self, Page};
use nodalia_gtk::{self as ndl, gtk, gtk::prelude::*};

use crate::i18n::tr;
use crate::pages::{
    about, accessibility, accounts, appearance, applications, bluetooth::BluetoothPage, cloud,
    color, datetime, displays, keyboard, localsend::LocalSendPage, multitasking,
    network::NetworkPage, notifications, pointing, power, printers, privacy, region, sharing,
    sound, storage, updates::UpdatesPage, users, wifi::WifiPage,
};

/// Builds a page when it is first shown.
type Factory = Box<dyn FnOnce() -> gtk::Widget>;

fn sidebar_row(page: &Page) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_widget_name(page.id);
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    content.append(&gtk::Image::from_icon_name(page.icon));
    let label = gtk::Label::new(Some(tr(page.title)));
    label.set_hexpand(true);
    label.set_xalign(0.0);
    content.append(&label);
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

/// Takes the window to a page by its id.
type Select = Rc<dyn Fn(&str)>;

thread_local! {
    /// Takes the open window to a page. Set while a window exists, so a second launch
    /// (`nodalix-settings wifi`, the `open-page` action) can ask the one that is open.
    static SELECT: RefCell<Option<Select>> = const { RefCell::new(None) };
}

/// Opens Settings, on `page` when one is asked for. If a window is open already it is the one
/// that goes to the page and comes forward: there is one Settings window per session.
pub fn present(app: &gtk::Application, page: Option<&Page>) {
    if let Some(window) = app.active_window() {
        if let Some(page) = page {
            // Take the handler out of the cell before calling it: it selects a row, and a
            // row's handler must not find the cell borrowed.
            let select = SELECT.with(|cell| cell.borrow().clone());
            if let Some(select) = select {
                select(page.id);
            }
        }
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

    // The content area: one stack child per page. A page is built the first time it is shown, so
    // opening Settings costs one page, and the pages nobody opened never poll a service, never
    // hold widgets that GTK restyles on a light/dark switch and never take memory.
    let stack = gtk::Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    let factories: Rc<RefCell<HashMap<&'static str, Factory>>> = Rc::default();
    {
        let add = |id: &'static str, factory: Factory| {
            factories.borrow_mut().insert(id, factory);
        };
        macro_rules! lazy {
            ($id:expr, $toasts:ident => $page:expr) => {{
                let $toasts = toasts.clone();
                add($id, Box::new(move || $page.upcast::<gtk::Widget>()));
            }};
        }
        lazy!("appearance", t => appearance::build(&t));
        lazy!("updates", t => {
            let page = UpdatesPage::new();
            page.set_toasts(&t);
            page.widget.clone()
        });
        lazy!("localsend", t => {
            let page = LocalSendPage::new();
            page.set_toasts(&t);
            page.widget.clone()
        });
        lazy!("about", _t => about::build());
        lazy!("wifi", t => {
            let page = WifiPage::new();
            page.set_toasts(&t);
            page.widget.clone()
        });
        lazy!("network", t => {
            let page = NetworkPage::new();
            page.set_toasts(&t);
            page.widget.clone()
        });
        lazy!("bluetooth", t => {
            let page = BluetoothPage::new();
            page.set_toasts(&t);
            page.widget.clone()
        });
        lazy!("sharing", t => sharing::build(&t));
        lazy!("sound", _t => sound::build());
        lazy!("power", _t => power::build());
        lazy!("keyboard", t => keyboard::build(&t));
        lazy!("mouse", _t => pointing::build());
        lazy!("notifications", _t => notifications::build());
        lazy!("multitasking", _t => multitasking::build());
        lazy!("privacy", _t => privacy::build());
        lazy!("accessibility", _t => accessibility::build());
        lazy!("applications", _t => applications::build());
        lazy!("region", t => region::build(&t));
        lazy!("users", t => users::build(&t));
        lazy!("display", t => displays::build(&t));
        lazy!("printers", t => printers::build(&t));
        lazy!("online-accounts", t => accounts::build(&t));
        lazy!("cloud", t => cloud::build(&t));
        lazy!("storage", t => storage::build(&t));
        lazy!("color", t => color::build(&t));
        lazy!("datetime", t => datetime::build(&t));
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
        let (stack, title, factories) = (stack.clone(), title.clone(), factories.clone());
        list.connect_row_selected(move |_, row| {
            let Some(row) = row else { return };
            let id = row.widget_name();
            if let Some(page) = pages::find(&id) {
                let factory = factories.borrow_mut().remove(page.id);
                if let Some(factory) = factory {
                    stack.add_named(&factory(), Some(page.id));
                }
                stack.set_visible_child_name(page.id);
                title.set_title(tr(page.title));
            }
        });
    }
    let select: Select = {
        let list = list.clone();
        Rc::new(move |id: &str| {
            for i in 0.. {
                let Some(row) = list.row_at_index(i) else {
                    break;
                };
                if row.widget_name() == id {
                    list.select_row(Some(&row));
                    break;
                }
            }
        })
    };
    SELECT.with(|cell| *cell.borrow_mut() = Some(select.clone()));
    window.connect_destroy(|_| SELECT.with(|cell| *cell.borrow_mut() = None));
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

    // The page asked for, or for screenshots and tests `NODALIX_SETTINGS_PAGE=updates`.
    let env_page = std::env::var("NODALIX_SETTINGS_PAGE")
        .ok()
        .and_then(|id| pages::resolve(&id));
    if let Some(page) = page.or(env_page) {
        select(page.id);
    }

    window.present();
}
