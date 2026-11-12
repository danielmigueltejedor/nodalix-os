//! A window: a sidebar of places beside the folder that is open.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::time::Duration;

use archivador_core::clipboard::{Clipboard, Mode};
use archivador_core::entry::{self, Entry, SortKey};
use archivador_core::history::History;
use archivador_core::settings::{Settings, View};
use archivador_core::trash::Trash;
use archivador_core::{names, ops, places};
use ndl::gtk::{self, gdk, gio, glib, prelude::*};
use nodalia_gtk as ndl;

use crate::dialogs;
use crate::i18n::{fmt, items, tr};
use crate::items::{self as look, ContextMenu};
use crate::object::FileObject;
use crate::share;

/// The place that stands for the trash in the history and in the sidebar.
const TRASH: &str = "trash:///";

thread_local! {
    /// The open windows. Their widgets only hold weak references to the browser, so this is what
    /// keeps one alive until its window closes.
    static OPEN: RefCell<Vec<Rc<Browser>>> = const { RefCell::new(Vec::new()) };

    /// What was copied or cut, shared by every window of the app.
    static CLIPBOARD: RefCell<Option<Clipboard>> = const { RefCell::new(None) };
}

struct State {
    history: History,
    entries: Vec<Entry>,
    settings: Settings,
    search: String,
    /// Files to select once the folder has been read again.
    select: Vec<PathBuf>,
    /// The folder could not be read.
    error: Option<String>,
}

pub struct Browser {
    window: ndl::ApplicationWindow,
    toasts: ndl::ToastOverlay,
    store: gio::ListStore,
    selection: gtk::MultiSelection,
    list_view: gtk::ListView,
    grid_view: gtk::GridView,
    pages: gtk::Stack,
    empty: ndl::StatusPage,
    path_bar: gtk::Box,
    path_scroll: gtk::ScrolledWindow,
    sidebar: gtk::ListBox,
    trash_icon: RefCell<Option<gtk::Image>>,
    search_bar: gtk::SearchBar,
    search_entry: gtk::SearchEntry,
    view_button: gtk::Button,
    state: RefCell<State>,
    monitor: RefCell<Option<gio::FileMonitor>>,
    reload_queued: Cell<bool>,
    syncing: Cell<bool>,
}

fn trash() -> Trash {
    Trash::new(&glib::user_data_dir())
}

fn is_trash(path: &Path) -> bool {
    path == Path::new(TRASH)
}

fn settings_file() -> PathBuf {
    glib::user_config_dir()
        .join("nodalix")
        .join("archivador.json")
}

fn load_settings() -> Settings {
    std::fs::read_to_string(settings_file())
        .map_or_else(|_| Settings::default(), |t| Settings::parse(&t))
}

fn save_settings(settings: &Settings) {
    let file = settings_file();
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(file, settings.to_text());
}

fn sort_name(key: SortKey) -> &'static str {
    match key {
        SortKey::Name => "name",
        SortKey::Modified => "modified",
        SortKey::Size => "size",
        SortKey::Kind => "kind",
    }
}

fn sort_from_name(name: &str) -> SortKey {
    match name {
        "modified" => SortKey::Modified,
        "size" => SortKey::Size,
        "kind" => SortKey::Kind,
        _ => SortKey::Name,
    }
}

impl Browser {
    pub fn new(app: &gtk::Application, start: PathBuf) -> Rc<Self> {
        let settings = load_settings();
        let window = ndl::ApplicationWindow::new(app);
        window.set_title(Some(tr("Files")));
        window.set_default_size(1100, 720);
        window.set_icon_name(Some("system-file-manager"));

        let store = gio::ListStore::new::<FileObject>();
        let selection = gtk::MultiSelection::new(Some(store.clone()));
        let list_view = gtk::ListView::new(Some(selection.clone()), None::<gtk::ListItemFactory>);
        list_view.add_css_class("archivador-files");
        list_view.set_tab_behavior(gtk::ListTabBehavior::Item);
        list_view.set_enable_rubberband(true);
        let grid_view = gtk::GridView::new(Some(selection.clone()), None::<gtk::ListItemFactory>);
        grid_view.add_css_class("archivador-files");
        grid_view.set_enable_rubberband(true);
        grid_view.set_max_columns(12);

        let scrolled = |child: &gtk::Widget| {
            gtk::ScrolledWindow::builder()
                .child(child)
                .hexpand(true)
                .vexpand(true)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .build()
        };
        let empty = ndl::StatusPage::builder()
            .icon_name("folder-symbolic")
            .build();
        let pages = gtk::Stack::new();
        pages.add_named(&scrolled(list_view.upcast_ref()), Some("list"));
        pages.add_named(&scrolled(grid_view.upcast_ref()), Some("grid"));
        pages.add_named(&empty, Some("empty"));

        let path_bar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        path_bar.add_css_class("archivador-pathbar");
        let path_scroll = gtk::ScrolledWindow::builder()
            .child(&path_bar)
            .hexpand(true)
            .hscrollbar_policy(gtk::PolicyType::External)
            .vscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(true)
            .build();

        let sidebar = gtk::ListBox::new();
        sidebar.add_css_class("navigation-sidebar");

        let search_entry = gtk::SearchEntry::new();
        search_entry.set_placeholder_text(Some(tr("Search in this folder")));
        search_entry.set_hexpand(true);
        let search_bar = gtk::SearchBar::new();
        search_bar.set_child(Some(&search_entry));
        search_bar.connect_entry(&search_entry);
        search_bar.set_key_capture_widget(Some(window.upcast_ref::<gtk::Widget>()));

        let view_button = gtk::Button::new();

        let this = Rc::new(Self {
            window,
            toasts: ndl::ToastOverlay::new(),
            store,
            selection,
            list_view,
            grid_view,
            pages,
            empty,
            path_bar,
            path_scroll,
            sidebar,
            trash_icon: RefCell::new(None),
            search_bar,
            search_entry,
            view_button,
            state: RefCell::new(State {
                history: History::new(start),
                entries: Vec::new(),
                settings,
                search: String::new(),
                select: Vec::new(),
                error: None,
            }),
            monitor: RefCell::new(None),
            reload_queued: Cell::new(false),
            syncing: Cell::new(false),
        });
        this.build();
        this.load();
        OPEN.with(|open| open.borrow_mut().push(this.clone()));
        let weak = this.weak();
        this.window.connect_close_request(move |_| {
            if let Some(this) = weak.upgrade() {
                OPEN.with(|open| open.borrow_mut().retain(|b| !Rc::ptr_eq(b, &this)));
            }
            glib::Propagation::Proceed
        });
        this
    }

    pub fn present(&self) {
        self.window.present();
    }

    /// Selects `paths` once the folder has been read.
    pub fn select_after_load(&self, paths: Vec<PathBuf>) {
        self.state.borrow_mut().select = paths;
        self.reload();
    }

    fn weak(self: &Rc<Self>) -> Weak<Self> {
        Rc::downgrade(self)
    }

    // ----- building the window ------------------------------------------------------------

    fn build(self: &Rc<Self>) {
        self.build_views();
        self.build_actions();

        // The sidebar.
        let sidebar_scroll = gtk::ScrolledWindow::builder()
            .child(&self.sidebar)
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        let sidebar_header = ndl::HeaderBar::new();
        sidebar_header.set_show_end_title_buttons(false);
        sidebar_header.set_title_widget(Some(&ndl::WindowTitle::new(tr("Files"), "")));
        let sidebar = ndl::ToolbarView::new();
        sidebar.add_top_bar(&sidebar_header);
        sidebar.set_content(Some(&sidebar_scroll));
        self.rebuild_sidebar();
        {
            let this = self.weak();
            self.sidebar.connect_row_activated(move |_, row| {
                let (Some(this), name) = (this.upgrade(), row.widget_name()) else {
                    return;
                };
                this.navigate(PathBuf::from(name.as_str()));
            });
        }
        {
            let this = self.weak();
            let monitor = gio::VolumeMonitor::get();
            let rebuild = move || {
                if let Some(this) = this.upgrade() {
                    this.rebuild_sidebar();
                    this.sync_sidebar();
                }
            };
            let again = rebuild.clone();
            monitor.connect_mount_added(move |_, _| again());
            let again = rebuild.clone();
            monitor.connect_mount_removed(move |_, _| again());
            monitor.connect_mount_changed(move |_, _| rebuild());
        }

        // The header.
        let header = ndl::HeaderBar::new();
        let nav = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        for (icon, action, tip) in [
            ("go-previous-symbolic", "win.back", "Back"),
            ("go-next-symbolic", "win.forward", "Forward"),
            ("go-up-symbolic", "win.up", "Parent folder"),
        ] {
            let button = gtk::Button::from_icon_name(icon);
            button.set_action_name(Some(action));
            button.set_tooltip_text(Some(tr(tip)));
            nav.append(&button);
        }
        header.pack_start(&nav);
        header.set_title_widget(Some(&self.path_scroll));

        let menu_button = gtk::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        menu_button.set_popover(Some(&self.main_menu()));
        header.pack_end(&menu_button);
        self.view_button.set_action_name(Some("win.toggle-view"));
        header.pack_end(&self.view_button);
        let new_folder = gtk::Button::from_icon_name("folder-new-symbolic");
        new_folder.set_action_name(Some("win.new-folder"));
        new_folder.set_tooltip_text(Some(tr("New folder")));
        header.pack_end(&new_folder);
        let search = gtk::ToggleButton::new();
        search.set_icon_name("edit-find-symbolic");
        search.set_tooltip_text(Some(tr("Search")));
        search
            .bind_property("active", &self.search_bar, "search-mode-enabled")
            .bidirectional()
            .sync_create()
            .build();
        header.pack_end(&search);
        {
            let this = self.weak();
            self.search_entry.connect_search_changed(move |entry| {
                if let Some(this) = this.upgrade() {
                    this.state.borrow_mut().search = entry.text().to_string();
                    this.repopulate();
                }
            });
        }

        // The content.
        let column = gtk::Box::new(gtk::Orientation::Vertical, 0);
        column.append(&self.search_bar);
        column.append(&self.pages);
        let content = ndl::ToolbarView::new();
        content.add_top_bar(&header);
        content.set_content(Some(&column));

        let split = ndl::OverlaySplitView::new();
        split.set_sidebar(Some(&sidebar));
        split.set_content(Some(&content));
        self.toasts.set_child(Some(&split));
        self.window.set_content(Some(&self.toasts));
        self.apply_view();
    }

    fn build_views(self: &Rc<Self>) {
        let menu: ContextMenu = {
            let this = self.weak();
            Rc::new(move |widget, x, y, position| {
                let Some(this) = this.upgrade() else { return };
                if !this.selection.is_selected(position) {
                    this.selection.select_item(position, true);
                }
                this.popup(widget, x, y, true);
            })
        };
        self.list_view
            .set_factory(Some(&look::list_factory(menu.clone())));
        self.grid_view.set_factory(Some(&look::grid_factory(menu)));

        let activate = {
            let this = self.weak();
            move |position: u32| {
                if let Some(this) = this.upgrade() {
                    this.activate(position);
                }
            }
        };
        let again = activate.clone();
        self.list_view.connect_activate(move |_, p| again(p));
        self.grid_view.connect_activate(move |_, p| activate(p));

        // A right click on the empty part of the folder.
        let click = gtk::GestureClick::builder().button(3).build();
        let this = self.weak();
        click.connect_pressed(move |gesture, _, x, y| {
            let Some(this) = this.upgrade() else { return };
            gesture.set_state(gtk::EventSequenceState::Claimed);
            this.selection.unselect_all();
            this.popup(this.pages.upcast_ref(), x, y, false);
        });
        self.pages.add_controller(click);

        let this = self.weak();
        self.selection.connect_selection_changed(move |_, _, _| {
            if let Some(this) = this.upgrade() {
                this.update_actions();
            }
        });
    }

    fn main_menu(&self) -> gtk::PopoverMenu {
        let menu = gio::Menu::new();
        let order = gio::Menu::new();
        for (label, key) in [
            ("Name", "name"),
            ("Modified", "modified"),
            ("Size", "size"),
            ("Type", "kind"),
        ] {
            order.append(Some(tr(label)), Some(&format!("win.sort::{key}")));
        }
        menu.append_submenu(Some(tr("Sort by")), &order);
        let toggles = gio::Menu::new();
        toggles.append(Some(tr("Reverse order")), Some("win.descending"));
        toggles.append(Some(tr("Show hidden files")), Some("win.show-hidden"));
        menu.append_section(None, &toggles);
        let more = gio::Menu::new();
        more.append_item(&ndl::menu_item(
            tr("Select all"),
            "win.select-all",
            "edit-select-all-symbolic",
        ));
        more.append_item(&ndl::menu_item(
            tr("Refresh"),
            "win.refresh",
            "view-refresh-symbolic",
        ));
        menu.append_section(None, &more);
        let about = gio::Menu::new();
        about.append_item(&ndl::menu_item(
            tr("About"),
            "win.about",
            "help-about-symbolic",
        ));
        menu.append_section(None, &about);
        let popover = gtk::PopoverMenu::from_model(Some(&menu));
        ndl::menu_rows(&popover);
        popover
    }

    /// The menu of a right click: on files, or on the empty part of the folder.
    fn context_model(&self, on_files: bool) -> gio::Menu {
        let menu = gio::Menu::new();
        let section = gio::Menu::new();
        let in_trash = is_trash(self.state.borrow().history.current());
        let item = |label: &str, action: &str, icon: &str| ndl::menu_item(label, action, icon);
        if in_trash {
            if on_files {
                section.append_item(&item(tr("Restore"), "win.restore", "edit-undo-symbolic"));
                menu.append_section(None, &section);
                let danger = gio::Menu::new();
                danger.append_item(&ndl::menu_item_danger(
                    tr("Delete permanently…"),
                    "win.delete",
                    "edit-delete-symbolic",
                ));
                menu.append_section(None, &danger);
            } else {
                section.append_item(&ndl::menu_item_danger(
                    tr("Empty the trash…"),
                    "win.empty-trash",
                    "edit-delete-symbolic",
                ));
                menu.append_section(None, &section);
            }
            return menu;
        }
        if on_files {
            section.append_item(&item(tr("Open"), "win.open", "document-open-symbolic"));
            menu.append_section(None, &section);
            let edit = gio::Menu::new();
            edit.append_item(&item(tr("Copy"), "win.copy", "edit-copy-symbolic"));
            edit.append_item(&item(tr("Cut"), "win.cut", "edit-cut-symbolic"));
            edit.append_item(&item(tr("Rename…"), "win.rename", "document-edit-symbolic"));
            menu.append_section(None, &edit);
            let share = gio::Menu::new();
            share.append_item(&item(
                tr("Send with LocalSend…"),
                "win.send",
                "send-to-symbolic",
            ));
            menu.append_section(None, &share);
            let danger = gio::Menu::new();
            danger.append_item(&ndl::menu_item_danger(
                tr("Move to the trash"),
                "win.trash",
                "user-trash-symbolic",
            ));
            menu.append_section(None, &danger);
        } else {
            section.append_item(&item(
                tr("New folder"),
                "win.new-folder",
                "folder-new-symbolic",
            ));
            section.append_item(&item(tr("Paste"), "win.paste", "edit-paste-symbolic"));
            section.append_item(&item(
                tr("Select all"),
                "win.select-all",
                "edit-select-all-symbolic",
            ));
            menu.append_section(None, &section);
        }
        menu
    }

    fn popup(&self, parent: &gtk::Widget, x: f64, y: f64, on_files: bool) {
        let popover = gtk::PopoverMenu::from_model(Some(&self.context_model(on_files)));
        ndl::menu_rows(&popover);
        popover.set_parent(parent);
        popover.set_has_arrow(false);
        popover.set_halign(gtk::Align::Start);
        popover.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
        popover.connect_closed(|popover| {
            let popover = popover.clone();
            glib::idle_add_local_once(move || popover.unparent());
        });
        popover.popup();
    }

    // ----- actions -------------------------------------------------------------------------

    fn action(self: &Rc<Self>, name: &str, run: impl Fn(&Rc<Self>) + 'static) {
        let action = gio::SimpleAction::new(name, None);
        let this = self.weak();
        action.connect_activate(move |_, _| {
            if let Some(this) = this.upgrade() {
                run(&this);
            }
        });
        self.window.add_action(&action);
    }

    fn build_actions(self: &Rc<Self>) {
        self.action("back", |t| t.go(|h| h.go_back().map(Path::to_path_buf)));
        self.action("forward", |t| {
            t.go(|h| h.go_forward().map(Path::to_path_buf))
        });
        self.action("up", |t| {
            let parent = t.state.borrow().history.parent();
            if let Some(parent) = parent {
                t.navigate(parent);
            }
        });
        self.action("home", |t| t.navigate(crate::home()));
        self.action("refresh", |t| t.reload());
        self.action("select-all", |t| {
            t.selection.select_all();
        });
        self.action("open", |t| {
            for entry in t.selected() {
                t.open(&entry);
            }
        });
        self.action("new-folder", Self::new_folder);
        self.action("rename", Self::rename);
        self.action("copy", |t| t.to_clipboard(Mode::Copy));
        self.action("cut", |t| t.to_clipboard(Mode::Cut));
        self.action("paste", Self::paste);
        self.action("trash", Self::trash_selected);
        self.action("delete", Self::delete_selected);
        self.action("restore", Self::restore_selected);
        self.action("empty-trash", Self::empty_trash);
        self.action("send", |t| {
            let paths: Vec<PathBuf> = t.selected().into_iter().map(|e| e.path).collect();
            if !paths.is_empty() {
                share::present(&t.window, paths, &t.toasts);
            }
        });
        self.action("search", |t| t.search_bar.set_search_mode(true));
        self.action("close", |t| t.window.close());
        self.action("toggle-view", |t| {
            {
                let mut state = t.state.borrow_mut();
                state.settings.view = match state.settings.view {
                    View::List => View::Grid,
                    View::Grid => View::List,
                };
                save_settings(&state.settings);
            }
            t.apply_view();
        });
        self.action("about", |t| {
            ndl::AboutDialog::builder()
                .application_icon("system-file-manager")
                .application_name(tr("Files"))
                .developer_name("Nodalix")
                .version(env!("CARGO_PKG_VERSION"))
                .comments(tr("Archivador, the Nodalix file manager."))
                .license_type(gtk::License::Gpl30)
                .build()
                .present(Some(&t.window));
        });

        // The ones that keep a state: the sort order and two switches.
        let settings = self.state.borrow().settings;
        let sort = gio::SimpleAction::new_stateful(
            "sort",
            Some(glib::VariantTy::STRING),
            &sort_name(settings.sort).to_variant(),
        );
        let this = self.weak();
        sort.connect_activate(move |action, parameter| {
            let (Some(this), Some(name)) =
                (this.upgrade(), parameter.and_then(|p| p.get::<String>()))
            else {
                return;
            };
            action.set_state(&name.to_variant());
            this.change(|s| s.sort = sort_from_name(&name));
        });
        self.window.add_action(&sort);
        for (name, value, apply) in [
            (
                "descending",
                settings.descending,
                (|s: &mut Settings, on| s.descending = on) as fn(&mut Settings, bool),
            ),
            (
                "show-hidden",
                settings.show_hidden,
                |s: &mut Settings, on| s.show_hidden = on,
            ),
        ] {
            let action = gio::SimpleAction::new_stateful(name, None, &value.to_variant());
            let this = self.weak();
            action.connect_activate(move |action, _| {
                let on = !action
                    .state()
                    .and_then(|s| s.get::<bool>())
                    .unwrap_or(false);
                action.set_state(&on.to_variant());
                if let Some(this) = this.upgrade() {
                    this.change(|s| apply(s, on));
                }
            });
            self.window.add_action(&action);
        }
    }

    /// Changes a setting, remembers it and shows the folder again.
    fn change(&self, apply: impl FnOnce(&mut Settings)) {
        {
            let mut state = self.state.borrow_mut();
            apply(&mut state.settings);
            save_settings(&state.settings);
        }
        self.repopulate();
    }

    fn set_enabled(&self, name: &str, enabled: bool) {
        if let Some(action) = self
            .window
            .lookup_action(name)
            .and_downcast::<gio::SimpleAction>()
        {
            action.set_enabled(enabled);
        }
    }

    /// Which actions make sense for what is selected and where we are.
    fn update_actions(&self) {
        let (in_trash, can_back, can_forward, can_up) = {
            let state = self.state.borrow();
            (
                is_trash(state.history.current()),
                state.history.can_go_back(),
                state.history.can_go_forward(),
                state.history.parent().is_some(),
            )
        };
        let selected = (0..self.store.n_items())
            .filter(|&i| self.selection.is_selected(i))
            .count();
        let some = selected > 0;
        let pasteable = CLIPBOARD.with(|c| c.borrow().is_some());
        self.set_enabled("back", can_back);
        self.set_enabled("forward", can_forward);
        self.set_enabled("up", can_up);
        self.set_enabled("open", some && !in_trash);
        self.set_enabled("copy", some && !in_trash);
        self.set_enabled("cut", some && !in_trash);
        self.set_enabled("rename", selected == 1 && !in_trash);
        self.set_enabled("trash", some && !in_trash);
        self.set_enabled("send", some && !in_trash);
        self.set_enabled("new-folder", !in_trash);
        self.set_enabled("paste", pasteable && !in_trash);
        self.set_enabled("restore", some && in_trash);
        self.set_enabled("delete", some);
        self.set_enabled("empty-trash", in_trash && self.store.n_items() > 0);
    }

    // ----- what is selected ----------------------------------------------------------------

    fn selected(&self) -> Vec<Entry> {
        (0..self.store.n_items())
            .filter(|&i| self.selection.is_selected(i))
            .filter_map(|i| self.store.item(i).and_downcast::<FileObject>())
            .map(|f| f.entry())
            .collect()
    }

    fn toast(&self, message: &str) {
        self.toasts.add_toast(ndl::Toast::new(message));
    }

    fn current_dir(&self) -> PathBuf {
        self.state.borrow().history.current().to_path_buf()
    }

    // ----- going places --------------------------------------------------------------------

    fn go(self: &Rc<Self>, step: impl FnOnce(&mut History) -> Option<PathBuf>) {
        let moved = step(&mut self.state.borrow_mut().history).is_some();
        if moved {
            self.load();
        }
    }

    pub fn navigate(self: &Rc<Self>, path: PathBuf) {
        if !is_trash(&path) && !path.is_dir() {
            self.toast(&fmt("Can't open {}", &path.display().to_string()));
            return;
        }
        if self.state.borrow().history.current() == path {
            return;
        }
        self.state.borrow_mut().history.visit(path);
        self.search_bar.set_search_mode(false);
        self.load();
    }

    fn activate(self: &Rc<Self>, position: u32) {
        let Some(file) = self.store.item(position).and_downcast::<FileObject>() else {
            return;
        };
        let entry = file.entry();
        if is_trash(&self.current_dir()) {
            return;
        }
        self.open(&entry);
    }

    fn open(self: &Rc<Self>, entry: &Entry) {
        if entry.is_dir {
            self.navigate(entry.path.clone());
            return;
        }
        let launcher = gtk::FileLauncher::new(Some(&gio::File::for_path(&entry.path)));
        let this = self.window.clone();
        let toasts = self.toasts.clone();
        launcher.launch(Some(&this), None::<&gio::Cancellable>, move |result| {
            if let Err(error) = result {
                if !error.matches(gtk::DialogError::Dismissed) {
                    toasts.add_toast(ndl::Toast::new(&format!(
                        "{}: {}",
                        tr("Can't open this file"),
                        error.message()
                    )));
                }
            }
        });
    }

    // ----- reading the folder --------------------------------------------------------------

    /// Shows the current place: reads it, watches it, and updates the chrome around it.
    fn load(self: &Rc<Self>) {
        let dir = self.current_dir();
        self.state.borrow_mut().search.clear();
        self.search_entry.set_text("");
        self.read(&dir);
        self.watch(&dir);
        self.show_title(&dir);
        self.rebuild_path_bar(&dir);
        self.sync_sidebar();
        self.update_actions();
    }

    /// Reads the folder again, keeping what is selected.
    fn reload(&self) {
        let dir = self.current_dir();
        if self.state.borrow().select.is_empty() {
            let kept: Vec<PathBuf> = self.selected().into_iter().map(|e| e.path).collect();
            self.state.borrow_mut().select = kept;
        }
        self.read(&dir);
        self.sync_sidebar();
    }

    fn read(&self, dir: &Path) {
        let result = if is_trash(dir) {
            Ok(trash().entries())
        } else {
            entry::read_dir(dir)
        };
        {
            let mut state = self.state.borrow_mut();
            match result {
                Ok(entries) => {
                    state.entries = entries;
                    state.error = None;
                }
                Err(error) => {
                    state.entries.clear();
                    state.error = Some(error.to_string());
                }
            }
        }
        self.repopulate();
    }

    fn watch(self: &Rc<Self>, dir: &Path) {
        self.monitor.replace(None);
        let target = if is_trash(dir) {
            trash().files_dir().to_path_buf()
        } else {
            dir.to_path_buf()
        };
        if is_trash(dir) {
            let _ = std::fs::create_dir_all(&target);
        }
        let file = gio::File::for_path(target);
        let Ok(monitor) = file.monitor_directory(
            gio::FileMonitorFlags::WATCH_MOVES,
            None::<&gio::Cancellable>,
        ) else {
            return;
        };
        let this = self.weak();
        monitor.connect_changed(move |_, _, _, _| {
            let Some(this) = this.upgrade() else { return };
            if this.reload_queued.replace(true) {
                return;
            }
            let later = this.weak();
            glib::timeout_add_local_once(Duration::from_millis(250), move || {
                if let Some(this) = later.upgrade() {
                    this.reload_queued.set(false);
                    this.reload();
                }
            });
        });
        self.monitor.replace(Some(monitor));
    }

    /// Fills the views from what was read, with the filter and the sort order applied.
    fn repopulate(&self) {
        let dir = self.current_dir();
        let in_trash = is_trash(&dir);
        let (shown, select, view, searching, error) = {
            let mut state = self.state.borrow_mut();
            let s = state.settings;
            let mut shown: Vec<Entry> = state
                .entries
                .iter()
                .filter(|e| entry::matches(e, s.show_hidden || in_trash, &state.search))
                .cloned()
                .collect();
            shown.sort_by(|a, b| entry::compare(a, b, s.sort, s.descending));
            (
                shown,
                std::mem::take(&mut state.select),
                s.view,
                !state.search.is_empty(),
                state.error.clone(),
            )
        };
        let objects: Vec<FileObject> = shown.iter().cloned().map(FileObject::new).collect();
        self.store.splice(0, self.store.n_items(), &objects);

        self.selection.unselect_all();
        let mut first = None;
        for (i, file) in shown.iter().enumerate() {
            if select.contains(&file.path) {
                self.selection.select_item(i as u32, false);
                first.get_or_insert(i as u32);
            }
        }
        if let Some(position) = first {
            match view {
                View::List => self
                    .list_view
                    .scroll_to(position, gtk::ListScrollFlags::FOCUS, None),
                View::Grid => self
                    .grid_view
                    .scroll_to(position, gtk::ListScrollFlags::FOCUS, None),
            }
        }

        if shown.is_empty() {
            let (icon, title, description) = if let Some(error) = error {
                ("dialog-error-symbolic", tr("Can't open this folder"), error)
            } else if searching {
                (
                    "edit-find-symbolic",
                    tr("Nothing matches your search"),
                    String::new(),
                )
            } else if in_trash {
                (
                    "user-trash-symbolic",
                    tr("The trash is empty"),
                    String::new(),
                )
            } else {
                ("folder-symbolic", tr("This folder is empty"), String::new())
            };
            self.empty.set_icon_name(Some(icon));
            self.empty.set_title(title);
            self.empty.set_description(
                Some(&description)
                    .filter(|d| !d.is_empty())
                    .map(String::as_str),
            );
            self.pages.set_visible_child_name("empty");
        } else {
            self.apply_view();
        }
        self.update_actions();
    }

    fn apply_view(&self) {
        let view = self.state.borrow().settings.view;
        let (name, icon, tip) = match view {
            View::List => ("list", "view-grid-symbolic", "Show as a grid"),
            View::Grid => ("grid", "view-list-symbolic", "Show as a list"),
        };
        self.view_button.set_icon_name(icon);
        self.view_button.set_tooltip_text(Some(tr(tip)));
        if self.store.n_items() > 0 {
            self.pages.set_visible_child_name(name);
        }
    }

    // ----- the chrome around the folder ----------------------------------------------------

    fn show_title(&self, dir: &Path) {
        let title = if is_trash(dir) {
            tr("Trash").to_owned()
        } else if dir == crate::home() {
            tr("Home").to_owned()
        } else {
            dir.file_name()
                .map_or_else(|| "/".into(), |n| n.to_string_lossy().into_owned())
        };
        self.window.set_title(Some(&title));
    }

    fn rebuild_path_bar(self: &Rc<Self>, dir: &Path) {
        while let Some(child) = self.path_bar.first_child() {
            self.path_bar.remove(&child);
        }
        let crumbs = if is_trash(dir) {
            vec![places::Crumb {
                label: Some(tr("Trash").to_owned()),
                path: dir.to_path_buf(),
            }]
        } else {
            places::breadcrumbs(dir, &crate::home())
        };
        let last = crumbs.len().saturating_sub(1);
        for (i, crumb) in crumbs.into_iter().enumerate() {
            if i > 0 {
                let arrow = gtk::Image::from_icon_name("go-next-symbolic");
                self.path_bar.append(&arrow);
            }
            let label = crumb.label.unwrap_or_else(|| tr("Home").to_owned());
            let button = gtk::Button::with_label(&label);
            if i == last {
                button.add_css_class("current");
            }
            let this = self.weak();
            let target = crumb.path;
            button.connect_clicked(move |_| {
                if let Some(this) = this.upgrade() {
                    this.navigate(target.clone());
                }
            });
            self.path_bar.append(&button);
        }
        // Keep the end of a long path in view.
        let scroll = self.path_scroll.clone();
        glib::idle_add_local_once(move || {
            let adjustment = scroll.hadjustment();
            adjustment.set_value(adjustment.upper());
        });
    }

    fn sidebar_row(&self, title: &str, icon: &str, path: &str) -> (gtk::ListBoxRow, gtk::Image) {
        let row = gtk::ListBoxRow::new();
        row.set_widget_name(path);
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let image = gtk::Image::from_icon_name(icon);
        content.append(&image);
        let label = gtk::Label::new(Some(title));
        label.set_xalign(0.0);
        label.set_hexpand(true);
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        content.append(&label);
        row.set_child(Some(&content));
        (row, image)
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

    /// Places, drives and the trash.
    fn rebuild_sidebar(&self) {
        while let Some(row) = self.sidebar.row_at_index(0) {
            self.sidebar.remove(&row);
        }
        let home = crate::home();
        let dirs = std::fs::read_to_string(glib::user_config_dir().join("user-dirs.dirs"))
            .unwrap_or_default();
        for place in places::standard(&home, &dirs, Path::is_dir) {
            let (row, _) =
                self.sidebar_row(tr(place.title), place.icon, &place.path.to_string_lossy());
            self.sidebar.append(&row);
        }
        let (row, icon) = self.sidebar_row(tr("Trash"), "user-trash-symbolic", TRASH);
        icon.set_icon_name(Some(if trash().is_empty() {
            "user-trash-symbolic"
        } else {
            "user-trash-full-symbolic"
        }));
        self.trash_icon.replace(Some(icon));
        self.sidebar.append(&row);

        // The drives that are mounted, and the file system itself.
        self.sidebar.append(&Self::heading(tr("Devices")));
        let (row, _) = self.sidebar_row("/", "drive-harddisk-symbolic", "/");
        self.sidebar.append(&row);
        for mount in gio::VolumeMonitor::get().mounts() {
            let Some(path) = mount.root().path() else {
                continue;
            };
            if mount.is_shadowed() || path == Path::new("/") {
                continue;
            }
            let (row, icon) = self.sidebar_row(
                &mount.name(),
                "drive-removable-media-symbolic",
                &path.to_string_lossy(),
            );
            icon.set_from_gicon(&mount.symbolic_icon());
            self.sidebar.append(&row);
        }
    }

    /// Highlights the sidebar row that is the folder we are in.
    fn sync_sidebar(&self) {
        if self.syncing.replace(true) {
            return;
        }
        let dir = self.current_dir();
        let wanted = dir.to_string_lossy().into_owned();
        self.sidebar.unselect_all();
        for i in 0.. {
            let Some(row) = self.sidebar.row_at_index(i) else {
                break;
            };
            if row.is_selectable() && row.widget_name() == wanted {
                self.sidebar.select_row(Some(&row));
                break;
            }
        }
        if let Some(icon) = self.trash_icon.borrow().as_ref() {
            icon.set_icon_name(Some(if trash().is_empty() {
                "user-trash-symbolic"
            } else {
                "user-trash-full-symbolic"
            }));
        }
        self.syncing.set(false);
    }

    // ----- changing things -----------------------------------------------------------------

    fn new_folder(self: &Rc<Self>) {
        let dir = self.current_dir();
        if is_trash(&dir) {
            return;
        }
        let proposal = names::unique_path(&dir, tr("Untitled folder"));
        let initial = proposal
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let this = self.weak();
        dialogs::ask_name(
            &self.window,
            "New folder",
            "Create",
            &initial,
            false,
            move |name| {
                let Some(this) = this.upgrade() else { return };
                match ops::create_folder(&dir, &name) {
                    Ok(path) => {
                        this.state.borrow_mut().select = vec![path];
                        this.reload();
                    }
                    Err(error) => this.toast(&format!("{name}: {error}")),
                }
            },
        );
    }

    fn rename(self: &Rc<Self>) {
        let mut selected = self.selected();
        if selected.len() != 1 {
            return;
        }
        let entry = selected.remove(0);
        let initial = entry.name.clone();
        let this = self.weak();
        dialogs::ask_name(
            &self.window,
            "Rename",
            "Rename",
            &initial,
            !entry.is_dir,
            move |name| {
                let Some(this) = this.upgrade() else { return };
                if name == entry.name {
                    return;
                }
                match ops::rename(&entry.path, &name) {
                    Ok(path) => {
                        this.state.borrow_mut().select = vec![path];
                        this.reload();
                    }
                    Err(error) => this.toast(&format!("{name}: {error}")),
                }
            },
        );
    }

    fn to_clipboard(&self, mode: Mode) {
        let paths: Vec<PathBuf> = self.selected().into_iter().map(|e| e.path).collect();
        if paths.is_empty() {
            return;
        }
        CLIPBOARD.with(|c| *c.borrow_mut() = Some(Clipboard::new(mode, paths)));
        self.update_actions();
    }

    fn paste(self: &Rc<Self>) {
        let dir = self.current_dir();
        let Some(clipboard) = CLIPBOARD.with(|c| c.borrow().clone()) else {
            return;
        };
        if is_trash(&dir) {
            return;
        }
        let this = self.weak();
        glib::spawn_future_local(async move {
            let cut = clipboard.mode == Mode::Cut;
            let target = dir.clone();
            let result = gio::spawn_blocking(move || clipboard.paste(&target)).await;
            let Some(this) = this.upgrade() else { return };
            match result {
                Ok(Ok(pasted)) => {
                    if cut {
                        CLIPBOARD.with(|c| *c.borrow_mut() = None);
                    }
                    if this.current_dir() == dir {
                        this.state.borrow_mut().select = pasted.clone();
                        this.reload();
                    }
                    this.update_actions();
                }
                Ok(Err(error)) => {
                    this.reload();
                    this.toast(&error.to_string());
                }
                Err(_) => this.toast("?"),
            }
        });
    }

    fn trash_selected(self: &Rc<Self>) {
        let selected = self.selected();
        if selected.is_empty() || is_trash(&self.current_dir()) {
            return;
        }
        let date = glib::DateTime::now_local()
            .ok()
            .and_then(|d| d.format("%Y-%m-%dT%H:%M:%S").ok())
            .map(|s| s.to_string())
            .unwrap_or_default();
        let bin = trash();
        let mut moved = 0;
        let mut refused: Vec<(Entry, String)> = Vec::new();
        for entry in selected {
            match bin.trash(&entry.path, &date) {
                Ok(()) => moved += 1,
                Err(error) => refused.push((entry, error.to_string())),
            }
        }
        self.reload();
        if moved > 0 {
            self.toast(&fmt("Moved {} to the trash", &items(moved)));
        }
        if let Some((entry, error)) = refused.first() {
            let paths: Vec<PathBuf> = refused.iter().map(|(e, _)| e.path.clone()).collect();
            let body = format!(
                "{}: {error}\n\n{}",
                entry.name,
                tr("It will be deleted for good. This can't be undone.")
            );
            let this = self.weak();
            dialogs::confirm(
                &self.window,
                tr("It can't be moved to the trash"),
                &body,
                "Delete",
                true,
                move || {
                    if let Some(this) = this.upgrade() {
                        this.remove_for_good(paths.clone());
                    }
                },
            );
        }
    }

    fn delete_selected(self: &Rc<Self>) {
        let selected = self.selected();
        if selected.is_empty() {
            return;
        }
        let name = match selected.as_slice() {
            [one] => format!("“{}”", one.name),
            many => items(many.len()),
        };
        let paths: Vec<PathBuf> = selected.into_iter().map(|e| e.path).collect();
        let this = self.weak();
        dialogs::confirm(
            &self.window,
            &format!("{} {name}?", tr("Delete permanently")),
            tr("It will be deleted for good. This can't be undone."),
            "Delete",
            true,
            move || {
                if let Some(this) = this.upgrade() {
                    this.remove_for_good(paths.clone());
                }
            },
        );
    }

    /// Deletes for good, off the main thread: a big folder takes a while.
    fn remove_for_good(self: &Rc<Self>, paths: Vec<PathBuf>) {
        let in_trash = is_trash(&self.current_dir());
        let this = self.weak();
        glib::spawn_future_local(async move {
            let count = paths.len();
            let result = gio::spawn_blocking(move || {
                let bin = trash();
                for path in &paths {
                    let done = if in_trash {
                        let name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        bin.delete(&name)
                    } else {
                        ops::delete(path)
                    };
                    done?;
                }
                Ok::<(), std::io::Error>(())
            })
            .await;
            let Some(this) = this.upgrade() else { return };
            this.reload();
            match result {
                Ok(Ok(())) => this.toast(&fmt("Deleted {}", &items(count))),
                Ok(Err(error)) => this.toast(&error.to_string()),
                Err(_) => this.toast("?"),
            }
        });
    }

    fn restore_selected(self: &Rc<Self>) {
        let bin = trash();
        let mut restored = 0;
        for entry in self.selected() {
            let name = entry
                .path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            match bin.restore(&name) {
                Ok(_) => restored += 1,
                Err(error) => self.toast(&format!("{}: {error}", entry.name)),
            }
        }
        self.reload();
        if restored > 0 {
            self.toast(&fmt("Restored {}", &items(restored)));
        }
    }

    fn empty_trash(self: &Rc<Self>) {
        let this = self.weak();
        dialogs::confirm(
            &self.window,
            tr("Empty the trash"),
            tr("Everything in the trash will be deleted for good."),
            "Empty the trash",
            true,
            move || {
                let Some(this) = this.upgrade() else { return };
                let again = this.weak();
                glib::spawn_future_local(async move {
                    let result = gio::spawn_blocking(|| trash().empty()).await;
                    let Some(this) = again.upgrade() else { return };
                    this.reload();
                    if let Ok(Err(error)) = result {
                        this.toast(&error.to_string());
                    }
                });
            },
        );
    }
}
