//! A window: a sidebar of places beside the folder that is open.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::time::Duration;

use archivador_core::bookmarks::{self, Bookmark};
use archivador_core::clipboard::{Clipboard, Mode};
use archivador_core::cloud;
use archivador_core::entry::{self, Entry, SortKey};
use archivador_core::history::History;
use archivador_core::settings::{Appearance, Settings, View};
use archivador_core::trash::Trash;
use archivador_core::{names, ops, places, recent, servers};
use ndl::gtk::{self, gdk, gio, glib, prelude::*};
use nodalia_gtk as ndl;

use crate::columns::{Columns, Options};
use crate::customize;
use crate::dialogs;
use crate::i18n::{fmt, items, tr};
use crate::items::{self as look, ContextMenu, Hooks, SPRING};
use crate::object::FileObject;
use crate::properties;
use crate::share;
use crate::transfer;

/// The place that stands for the trash in the history and in the sidebar.
/// The sidebar row a drag is over, and the timer that opens it.
type Hover = Option<(gtk::ListBoxRow, Option<glib::SourceId>)>;

pub const TRASH: &str = "trash:///";
pub const RECENT: &str = "recent:///";
/// What a sidebar row's name holds when it is not a folder: the question to ask for a server.
const CONNECT: &str = "action:connect-server";
/// What a row's name starts with when it is a server that is not mounted: its address follows.
const SERVER: &str = "server:";

thread_local! {
    /// The open windows. Their widgets only hold weak references to the browser, so this is what
    /// keeps one alive until its window closes.
    static OPEN: RefCell<Vec<Rc<Browser>>> = const { RefCell::new(Vec::new()) };

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

/// One segment of the path bar: the arrow before it, the button, and where it leads.
struct PathItem {
    arrow: Option<gtk::Image>,
    button: gtk::Button,
    path: PathBuf,
}

pub struct Browser {
    window: ndl::ApplicationWindow,
    toasts: ndl::ToastOverlay,
    store: gio::ListStore,
    selection: gtk::MultiSelection,
    list_view: gtk::ListView,
    grid_view: gtk::GridView,
    /// The folder's column in the columns view.
    columns_view: gtk::ListView,
    miller: Rc<Columns>,
    pages: gtk::Stack,
    empty: ndl::StatusPage,
    path_bar: gtk::Box,
    path_scroll: gtk::ScrolledWindow,
    /// The path bar, or the field a path can be typed in (Ctrl+L).
    path_stack: gtk::Stack,
    location: gtk::Entry,
    /// The column header of the list, and the arrow that shows the sort order in it.
    columns: gtk::Box,
    arrows: Vec<(SortKey, gtk::Image, gtk::Button)>,
    status: gtk::Label,
    spinner: gtk::Spinner,
    busy: Cell<u32>,
    sidebar: gtk::ListBox,
    trash_icon: RefCell<Option<gtk::Image>>,
    search_bar: gtk::SearchBar,
    search_entry: gtk::SearchEntry,
    view_button: gtk::MenuButton,
    /// The segments of the path bar, and the button that stands for those that do not fit.
    path_items: RefCell<Vec<PathItem>>,
    path_more: gtk::Button,
    path_more_target: RefCell<PathBuf>,
    path_width: Cell<i32>,
    state: RefCell<State>,
    monitor: RefCell<Option<gio::FileMonitor>>,
    reload_queued: Cell<bool>,
    syncing: Cell<bool>,
    /// The cloud accounts in the sidebar, as Settings listed them.
    clouds: RefCell<Vec<cloud::Account>>,
    /// What was selected when the button went down on a file, before the list could narrow it to
    /// that file. A drag that starts on one of these carries all of them.
    pressed: RefCell<Vec<PathBuf>>,
}

fn trash() -> Trash {
    Trash::new(&glib::user_data_dir())
}

/// A press on a file is the start of a drag, and the list would draw a selection box for as long
/// as it takes the drag to begin, and leave it there afterwards. Only a press on the empty part
/// is meant to draw one, so the box is hidden for the others.
fn hide_rubberband_on_files(view: &impl IsA<gtk::Widget>) {
    let press = gtk::GestureClick::new();
    press.set_propagation_phase(gtk::PropagationPhase::Capture);
    press.connect_pressed(|gesture, _, x, y| {
        let Some(view) = gesture.widget() else { return };
        let mut node = view.pick(x, y, gtk::PickFlags::DEFAULT);
        let mut on_file = false;
        while let Some(widget) = node {
            if widget == view {
                break;
            }
            if matches!(widget.css_name().as_str(), "row" | "child") {
                on_file = true;
                break;
            }
            node = widget.parent();
        }
        if on_file {
            view.add_css_class("archivador-on-file");
        } else {
            view.remove_css_class("archivador-on-file");
        }
    });
    view.add_controller(press);
}

fn is_trash(path: &Path) -> bool {
    path == Path::new(TRASH)
}

fn is_recent(path: &Path) -> bool {
    path == Path::new(RECENT)
}

/// A place that is not a folder on disk, so nothing is made, pasted or dropped in it.
fn is_virtual(path: &Path) -> bool {
    is_trash(path) || is_recent(path)
}

fn bookmarks_file() -> PathBuf {
    glib::user_config_dir().join("gtk-3.0").join("bookmarks")
}

fn load_bookmarks() -> Vec<Bookmark> {
    bookmarks::parse(&std::fs::read_to_string(bookmarks_file()).unwrap_or_default())
}

fn save_bookmarks(list: &[Bookmark]) {
    let file = bookmarks_file();
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(file, bookmarks::serialize(list));
}

fn servers_file() -> PathBuf {
    glib::user_config_dir()
        .join("nodalix")
        .join("archivador-servers")
}

fn load_servers() -> Vec<String> {
    servers::parse(&std::fs::read_to_string(servers_file()).unwrap_or_default())
}

fn save_servers(list: &[String]) {
    let file = servers_file();
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(file, servers::serialize(list));
}

/// The files the desktop says were opened lately, newest visit first by their time.
fn recent_entries() -> Vec<Entry> {
    let items = gtk::RecentManager::default()
        .items()
        .into_iter()
        .filter_map(|info| {
            let path = gio::File::for_uri(&info.uri()).path()?;
            // A share or a phone reached through GVfs: looking at it can take as long as the
            // network does, with the window waiting, so it is not listed.
            if path.components().any(|c| c.as_os_str() == "gvfs") {
                return None;
            }
            let visited = u64::try_from(info.visited().to_unix())
                .ok()
                .map(|secs| std::time::UNIX_EPOCH + Duration::from_secs(secs));
            Some(recent::Item { path, visited })
        });
    recent::entries(items)
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

/// `$XDG_STATE_HOME`, which glib has no function for.
fn state_dir() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| crate::home().join(".local/state"))
}

/// Whether something is mounted at `path`.
fn is_mounted(path: &Path) -> bool {
    std::fs::read_to_string("/proc/self/mountinfo").is_ok_and(|info| cloud::is_mounted(&info, path))
}

/// Runs `systemctl --user <action> <unit>`; the error is what it said.
fn systemctl(action: &str, unit: &str) -> Result<(), String> {
    let output = std::process::Command::new("systemctl")
        .args(["--user", action, unit])
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        let said = String::from_utf8_lossy(&output.stderr);
        Err(said.lines().next().unwrap_or("").trim().to_owned())
    }
}

/// Makes the window light, dark, or what the system is.
fn apply_appearance(appearance: Appearance) {
    ndl::StyleManager::default().set_color_scheme(match appearance {
        Appearance::System => ndl::ColorScheme::Default,
        Appearance::Light => ndl::ColorScheme::ForceLight,
        Appearance::Dark => ndl::ColorScheme::ForceDark,
    });
}

fn view_name(view: View) -> &'static str {
    match view {
        View::List => "list",
        View::Grid => "grid",
        View::Columns => "columns",
    }
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
        apply_appearance(settings.appearance);
        let window = ndl::ApplicationWindow::new(app);
        window.set_title(Some(tr("Files")));
        window.set_default_size(1100, 720);
        window.set_size_request(560, 360);
        window.set_icon_name(Some("os.nodalix.Archivador"));

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

        let columns_view =
            gtk::ListView::new(Some(selection.clone()), None::<gtk::ListItemFactory>);
        columns_view.add_css_class("archivador-files");
        columns_view.set_enable_rubberband(true);
        let miller = Columns::new(&columns_view);

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
        pages.add_named(&miller.widget, Some("columns"));
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

        let view_button = gtk::MenuButton::new();
        let path_more = gtk::Button::with_label("…");

        let location = gtk::Entry::new();
        location.set_hexpand(true);
        let path_stack = gtk::Stack::new();
        path_stack.set_hexpand(true);
        path_stack.add_named(&path_scroll, Some("bar"));
        path_stack.add_named(&location, Some("entry"));

        let (columns, arrows) = Self::column_header();
        let status = gtk::Label::new(None);
        status.add_css_class("archivador-status");
        status.set_xalign(0.0);
        let spinner = gtk::Spinner::new();
        spinner.set_visible(false);

        let this = Rc::new(Self {
            window,
            toasts: ndl::ToastOverlay::new(),
            store,
            selection,
            list_view,
            grid_view,
            columns_view,
            miller,
            pages,
            empty,
            path_bar,
            path_scroll,
            path_stack,
            location,
            columns,
            arrows,
            status,
            spinner,
            busy: Cell::new(0),
            sidebar,
            trash_icon: RefCell::new(None),
            search_bar,
            search_entry,
            view_button,
            path_items: RefCell::new(Vec::new()),
            path_more,
            path_more_target: RefCell::new(PathBuf::new()),
            path_width: Cell::new(0),
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
            clouds: RefCell::new(Vec::new()),
            pressed: RefCell::new(Vec::new()),
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

    /// The token of the launch that asked for this window, so the desktop gives it the focus.
    pub fn set_startup_id(&self, id: &str) {
        if !id.is_empty() {
            self.window.set_startup_id(id);
        }
    }

    /// Opens the properties of these files or folders over the window.
    pub fn show_properties(&self, paths: &[PathBuf]) {
        let entries: Vec<Entry> = paths.iter().filter_map(|p| Entry::at(p.clone())).collect();
        if !entries.is_empty() {
            properties::present(&self.window, entries);
        }
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
                if name == CONNECT {
                    this.sync_sidebar();
                    this.ask_server();
                    return;
                }
                if let Some(uri) = name.strip_prefix(SERVER) {
                    this.sync_sidebar();
                    this.connect_server(uri.to_owned());
                    return;
                }
                let path = PathBuf::from(name.as_str());
                let account = this
                    .clouds
                    .borrow()
                    .iter()
                    .find(|a| a.mount_point(&crate::home()) == path)
                    .cloned();
                match account {
                    Some(account) if !is_mounted(&path) => this.connect_cloud(account),
                    _ => this.navigate(path),
                }
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
        header.set_title_widget(Some(&self.path_stack));
        self.build_location();
        self.connect_columns();
        {
            let this = self.weak();
            self.path_more.connect_clicked(move |_| {
                if let Some(this) = this.upgrade() {
                    let target = this.path_more_target.borrow().clone();
                    this.navigate(target);
                }
            });
            // The bar fits itself again whenever the window is resized.
            let this = self.weak();
            self.path_scroll.add_tick_callback(move |scroll, _| {
                if let Some(this) = this.upgrade() {
                    if scroll.width() != this.path_width.get() {
                        this.fit_path_bar();
                    }
                }
                glib::ControlFlow::Continue
            });
        }

        let menu_button = gtk::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        menu_button.set_popover(Some(&self.main_menu()));
        header.pack_end(&menu_button);
        header.pack_end(&self.spinner);
        self.view_button.set_popover(Some(&Self::view_menu()));
        self.view_button.set_tooltip_text(Some(tr("View")));
        header.pack_end(&self.view_button);
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
        column.append(&self.columns);
        column.append(&self.pages);
        column.append(&self.status);
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

    /// The strip above the list: Name, Modified and Size, each one sorting by itself.
    fn column_header() -> (gtk::Box, Vec<(SortKey, gtk::Image, gtk::Button)>) {
        let strip = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        strip.add_css_class("archivador-columns");
        // The icon's width, so Name starts where the names do.
        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_size_request(24, -1);
        strip.append(&spacer);
        let mut arrows = Vec::new();
        for (key, title) in [
            (SortKey::Name, "Name"),
            (SortKey::Modified, "Modified"),
            (SortKey::Size, "Size"),
        ] {
            let button = gtk::Button::new();
            button.add_css_class("flat");
            let content = gtk::Box::new(gtk::Orientation::Horizontal, 4);
            let label = gtk::Label::new(Some(tr(title)));
            let arrow = gtk::Image::new();
            arrow.set_pixel_size(12);
            if key == SortKey::Name {
                button.set_hexpand(true);
                content.set_halign(gtk::Align::Start);
                content.append(&label);
                content.append(&arrow);
            } else {
                content.set_halign(gtk::Align::End);
                content.append(&arrow);
                content.append(&label);
                if key == SortKey::Modified {
                    look::align_modified(&button);
                } else {
                    look::align_size(&button);
                }
            }
            button.set_child(Some(&content));
            strip.append(&button);
            arrows.push((key, arrow, button));
        }
        (strip, arrows)
    }

    /// Clicking a column sorts by it; clicking the one that sorts already reverses the order.
    fn connect_columns(self: &Rc<Self>) {
        for (key, _, button) in &self.arrows {
            let (this, key) = (self.weak(), *key);
            button.connect_clicked(move |_| {
                let Some(this) = this.upgrade() else { return };
                let current = this.state.borrow().settings;
                if current.sort == key {
                    let descending = !current.descending;
                    this.change(|s| s.descending = descending);
                    this.window
                        .change_action_state("descending", &descending.to_variant());
                } else {
                    this.change(|s| s.sort = key);
                    this.window
                        .change_action_state("sort", &sort_name(key).to_variant());
                }
            });
        }
    }

    /// Ctrl+L: the path bar turns into a field to type a path in.
    fn build_location(self: &Rc<Self>) {
        let this = self.weak();
        self.location.connect_activate(move |entry| {
            let Some(this) = this.upgrade() else { return };
            let typed = entry.text().to_string();
            let path = match typed.strip_prefix('~') {
                Some(rest) => PathBuf::from(format!("{}{rest}", crate::home().display())),
                None => PathBuf::from(&typed),
            };
            this.path_stack.set_visible_child_name("bar");
            if path.is_dir() {
                this.navigate(path);
                this.focus_files();
            } else if path.exists() {
                let entry = Entry {
                    name: typed,
                    path,
                    ..Entry::default()
                };
                this.open(&entry);
            } else {
                this.toast(&fmt("Can't open {}", &typed));
            }
        });
        let keys = gtk::EventControllerKey::new();
        let this = self.weak();
        keys.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                if let Some(this) = this.upgrade() {
                    this.path_stack.set_visible_child_name("bar");
                    this.focus_files();
                }
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        self.location.add_controller(keys);
        let focus = gtk::EventControllerFocus::new();
        let this = self.weak();
        focus.connect_leave(move |_| {
            if let Some(this) = this.upgrade() {
                this.path_stack.set_visible_child_name("bar");
            }
        });
        self.location.add_controller(focus);
    }

    fn show_location(&self) {
        let dir = self.current_dir();
        if is_trash(&dir) {
            return;
        }
        self.location.set_text(&dir.to_string_lossy());
        self.path_stack.set_visible_child_name("entry");
        self.location.grab_focus();
        self.location.select_region(0, -1);
    }

    fn focus_files(&self) {
        match self.state.borrow().settings.view {
            View::List => self.list_view.grab_focus(),
            View::Grid => self.grid_view.grab_focus(),
            View::Columns => self.columns_view.grab_focus(),
        };
    }

    /// The little spinner in the header while something runs in the background.
    fn working(&self, on: bool) {
        let count = if on {
            self.busy.get() + 1
        } else {
            self.busy.get().saturating_sub(1)
        };
        self.busy.set(count);
        self.spinner.set_visible(count > 0);
        self.spinner.set_spinning(count > 0);
    }

    /// The line under the files: how many there are, or what is selected.
    fn update_status(&self) {
        let selected = self.selected();
        let text = if selected.is_empty() {
            let total = self.store.n_items() as usize;
            if self.state.borrow().search.is_empty() {
                items(total)
            } else {
                fmt("{} found", &items(total))
            }
        } else {
            let bytes: u64 = selected.iter().filter(|e| !e.is_dir).map(|e| e.size).sum();
            let mut text = fmt("{} selected", &items(selected.len()));
            if bytes > 0 {
                text = format!("{text} · {}", archivador_core::format::size(bytes));
            }
            text
        };
        self.status.set_label(&text);
        self.status.set_visible(self.store.n_items() > 0);
    }

    /// The arrow in the column header that shows how the list is sorted.
    fn update_columns(&self) {
        let (key, descending, view) = {
            let s = self.state.borrow().settings;
            (s.sort, s.descending, s.view)
        };
        for (column, arrow, _) in &self.arrows {
            if *column == key {
                arrow.set_icon_name(Some(if descending {
                    "pan-up-symbolic"
                } else {
                    "pan-down-symbolic"
                }));
                arrow.set_visible(true);
            } else {
                arrow.set_icon_name(None);
                arrow.set_visible(false);
            }
        }
        self.columns
            .set_visible(view == View::List && self.store.n_items() > 0);
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
        let drag_paths: Rc<dyn Fn(u32) -> Vec<PathBuf>> = {
            let this = self.weak();
            Rc::new(move |position| {
                let Some(this) = this.upgrade() else {
                    return Vec::new();
                };
                // The list narrows the selection to the file pressed when the button goes down. A
                // drag that starts on a file that was one of several selected carries them all,
                // and puts the selection back as it was.
                let before = this.pressed.borrow().clone();
                let grabbed = this
                    .store
                    .item(position)
                    .and_downcast::<FileObject>()
                    .map(|f| f.entry().path);
                if before.len() > 1 && grabbed.is_some_and(|p| before.contains(&p)) {
                    if this.selected().len() != before.len() {
                        this.selection.unselect_all();
                        for i in 0..this.store.n_items() {
                            let there = this
                                .store
                                .item(i)
                                .and_downcast::<FileObject>()
                                .is_some_and(|f| before.contains(&f.entry().path));
                            if there {
                                this.selection.select_item(i, false);
                            }
                        }
                    }
                    return before;
                }
                if !this.selection.is_selected(position) {
                    this.selection.select_item(position, true);
                }
                this.selected().into_iter().map(|e| e.path).collect()
            })
        };
        let drop_into: Rc<dyn Fn(u32, Vec<PathBuf>, bool) -> bool> = {
            let this = self.weak();
            Rc::new(move |position, paths, copy| {
                let Some(this) = this.upgrade() else {
                    return false;
                };
                let Some(folder) = this.store.item(position).and_downcast::<FileObject>() else {
                    return false;
                };
                this.drop_files(paths, folder.entry().path, copy)
            })
        };
        let open_folder: Rc<dyn Fn(u32)> = {
            let this = self.weak();
            Rc::new(move |position| {
                let Some(this) = this.upgrade() else { return };
                let Some(folder) = this.store.item(position).and_downcast::<FileObject>() else {
                    return;
                };
                let entry = folder.entry();
                if entry.is_dir {
                    this.navigate(entry.path);
                }
            })
        };
        let press: Rc<dyn Fn(u32, bool)> = {
            let this = self.weak();
            Rc::new(move |position, extend| {
                let Some(this) = this.upgrade() else { return };
                if extend {
                    // Ctrl and Shift change the selection themselves; nothing to keep.
                    this.pressed.borrow_mut().clear();
                    return;
                }
                if !this.selection.is_selected(position) {
                    this.selection.select_item(position, true);
                }
                let kept = this.selected().into_iter().map(|e| e.path).collect();
                this.pressed.replace(kept);
            })
        };
        let hooks = Hooks {
            press,
            menu,
            drag_paths,
            drop_into,
            open_folder,
        };
        hide_rubberband_on_files(&self.list_view);
        hide_rubberband_on_files(&self.grid_view);
        self.list_view
            .set_factory(Some(&look::list_factory(hooks.clone())));
        self.grid_view
            .set_factory(Some(&look::grid_factory(hooks.clone())));
        self.columns_view
            .set_factory(Some(&look::column_factory(Some(hooks))));
        {
            let (go, open) = (self.weak(), self.weak());
            self.miller.connect(
                Rc::new(move |folder, select| {
                    if let Some(this) = go.upgrade() {
                        this.navigate(folder);
                        if !select.is_empty() {
                            this.select_after_load(select);
                        }
                    }
                }),
                Rc::new(move |entry| {
                    if let Some(this) = open.upgrade() {
                        this.open(&entry);
                    }
                }),
            );
        }

        // Dropping on the empty part of the folder puts the files in it.
        let target = gtk::DropTarget::new(
            gdk::FileList::static_type(),
            gdk::DragAction::COPY | gdk::DragAction::MOVE,
        );
        let this = self.weak();
        target.connect_drop(move |target, value, _, _| {
            let Some(this) = this.upgrade() else {
                return false;
            };
            let dir = this.current_dir();
            !is_virtual(&dir)
                && this.drop_files(
                    transfer::paths_of(value),
                    dir,
                    transfer::drop_copies(target),
                )
        });
        self.pages.add_controller(target);

        // Dropping on a place in the sidebar puts them there, and on the trash trashes them. A drag
        // that rests on a place lights it up and, after a moment, goes into it.
        let target = gtk::DropTarget::new(
            gdk::FileList::static_type(),
            gdk::DragAction::COPY | gdk::DragAction::MOVE,
        );
        let over: Rc<RefCell<Hover>> = Rc::default();
        let clear = {
            let over = over.clone();
            move || {
                if let Some((row, timer)) = over.borrow_mut().take() {
                    row.remove_css_class("archivador-drop-target");
                    if let Some(timer) = timer {
                        timer.remove();
                    }
                }
            }
        };
        {
            let (this, over, clear) = (self.weak(), over.clone(), clear.clone());
            target.connect_motion(move |_, _, y| {
                let Some(this) = this.upgrade() else {
                    return gdk::DragAction::empty();
                };
                let row = this
                    .sidebar
                    .row_at_y(y as i32)
                    .filter(|r| r.is_selectable());
                let Some(row) = row else {
                    clear();
                    return gdk::DragAction::empty();
                };
                if over.borrow().as_ref().is_some_and(|(r, _)| *r == row) {
                    return gdk::DragAction::MOVE;
                }
                clear();
                row.add_css_class("archivador-drop-target");
                let path = PathBuf::from(row.widget_name().as_str());
                // The trash is not a place to look into, and a cloud that is not connected has
                // nothing to show yet.
                let opens = !is_virtual(&path)
                    && !path.to_string_lossy().starts_with("action:")
                    && !path.to_string_lossy().starts_with(SERVER)
                    && !this
                        .clouds
                        .borrow()
                        .iter()
                        .any(|a| a.mount_point(&crate::home()) == path && !is_mounted(&path));
                let timer = opens.then(|| {
                    let (again, over) = (this.weak(), over.clone());
                    glib::timeout_add_local_once(SPRING, move || {
                        if let Some((_, timer)) = over.borrow_mut().as_mut() {
                            timer.take();
                        }
                        if let Some(this) = again.upgrade() {
                            this.navigate(path);
                        }
                    })
                });
                over.borrow_mut().replace((row, timer));
                gdk::DragAction::MOVE
            });
        }
        target.connect_leave({
            let clear = clear.clone();
            move |_| clear()
        });
        let this = self.weak();
        target.connect_drop(move |target, value, _, y| {
            clear();
            let Some(this) = this.upgrade() else {
                return false;
            };
            let Some(row) = this
                .sidebar
                .row_at_y(y as i32)
                .filter(|r| r.is_selectable())
            else {
                return false;
            };
            this.drop_files(
                transfer::paths_of(value),
                PathBuf::from(row.widget_name().as_str()),
                transfer::drop_copies(target),
            )
        });
        self.sidebar.add_controller(target);

        let this = self.weak();
        self.window.clipboard().connect_changed(move |_| {
            if let Some(this) = this.upgrade() {
                this.update_actions();
            }
        });

        let activate = {
            let this = self.weak();
            move |position: u32| {
                if let Some(this) = this.upgrade() {
                    this.activate(position);
                }
            }
        };
        let again = activate.clone();
        let columns = activate.clone();
        self.columns_view.connect_activate(move |_, p| columns(p));
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
                this.update_status();
                this.update_miller();
            }
        });
    }

    /// The choice between the list, the grid and the columns.
    fn view_menu() -> gtk::PopoverMenu {
        let menu = gio::Menu::new();
        for (label, view, icon) in [
            ("List", "list", "view-list-symbolic"),
            ("Grid", "grid", "view-grid-symbolic"),
            ("Columns", "columns", "view-columns-symbolic"),
        ] {
            menu.append_item(&ndl::menu_item(
                tr(label),
                &format!("win.view::{view}"),
                icon,
            ));
        }
        let popover = gtk::PopoverMenu::from_model(Some(&menu));
        ndl::menu_rows(&popover);
        popover
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
            tr("New folder"),
            "win.new-folder",
            "folder-new-symbolic",
        ));
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
        more.append_item(&ndl::menu_item(
            tr("Connect to a server…"),
            "win.connect-server",
            "network-workgroup-symbolic",
        ));
        menu.append_section(None, &more);
        let about = gio::Menu::new();
        about.append_item(&ndl::menu_item(
            tr("Preferences"),
            "win.preferences",
            "emblem-system-symbolic",
        ));
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
            section.append_item(&item(
                tr("Open With…"),
                "win.open-with",
                "system-run-symbolic",
            ));
            menu.append_section(None, &section);
            let edit = gio::Menu::new();
            edit.append_item(&item(tr("Copy"), "win.copy", "edit-copy-symbolic"));
            edit.append_item(&item(tr("Cut"), "win.cut", "edit-cut-symbolic"));
            edit.append_item(&item(tr("Rename…"), "win.rename", "document-edit-symbolic"));
            if let Some(folder) = self.selected_folder() {
                let pinned = load_bookmarks().iter().any(|b| b.path == folder);
                edit.append_item(&item(
                    tr(if pinned {
                        "Remove from Favorites"
                    } else {
                        "Add to Favorites"
                    }),
                    "win.favorite",
                    if pinned {
                        "starred-symbolic"
                    } else {
                        "non-starred-symbolic"
                    },
                ));
            }
            menu.append_section(None, &edit);
            let share = gio::Menu::new();
            if self.selected_folder().is_some() {
                share.append_item(&item(
                    tr("Customize folder…"),
                    "win.customize-folder",
                    "preferences-color-symbolic",
                ));
            }
            share.append_item(&item(
                tr("Send with LocalSend…"),
                "win.send",
                "send-to-symbolic",
            ));
            share.append_item(&item(
                tr("Properties"),
                "win.properties",
                "document-properties-symbolic",
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

    /// The folder that is the only thing selected, if that is what is.
    fn selected_folder(&self) -> Option<PathBuf> {
        let mut selected = self.selected();
        (selected.len() == 1 && selected[0].is_dir && !is_virtual(&self.current_dir()))
            .then(|| selected.remove(0).path)
    }

    /// Pins the selected folder to the sidebar, or takes it off again.
    fn toggle_favorite(self: &Rc<Self>) {
        let Some(folder) = self.selected_folder() else {
            return;
        };
        let list = load_bookmarks();
        let list = if list.iter().any(|b| b.path == folder) {
            self.toast(tr("Removed from Favorites"));
            bookmarks::remove(list, &folder)
        } else {
            self.toast(tr("Added to Favorites"));
            bookmarks::add(list, &folder)
        };
        save_bookmarks(&list);
        self.rebuild_sidebar();
        self.sync_sidebar();
    }

    fn popup(&self, parent: &gtk::Widget, x: f64, y: f64, on_files: bool) {
        // The recent files have no empty part to ask a menu of.
        if !on_files && is_recent(&self.current_dir()) {
            return;
        }
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
        self.action("open-with", |t| {
            let mut selected = t.selected();
            if selected.len() == 1 {
                let entry = selected.remove(0);
                let launcher = gtk::FileLauncher::new(Some(&gio::File::for_path(&entry.path)));
                launcher.set_always_ask(true);
                launcher.launch(Some(&t.window), None::<&gio::Cancellable>, |_| {});
            }
        });
        self.action("properties", |t| {
            let selected = t.selected();
            if !selected.is_empty() {
                properties::present(&t.window, selected);
            }
        });
        self.action("open", |t| {
            for entry in t.selected() {
                t.open(&entry);
            }
        });
        self.action("new-folder", Self::new_folder);
        self.action("favorite", Self::toggle_favorite);
        self.action("customize-folder", |t| {
            if let Some(folder) = t.selected_folder() {
                let this = t.weak();
                customize::present(&t.window, folder, move || {
                    if let Some(this) = this.upgrade() {
                        this.reload();
                    }
                });
            }
        });
        self.action("connect-server", Self::ask_server);
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
        self.action("location", |t| t.show_location());
        self.action("close", |t| t.window.close());
        let view = gio::SimpleAction::new_stateful(
            "view",
            Some(glib::VariantTy::STRING),
            &view_name(self.state.borrow().settings.view).to_variant(),
        );
        let this = self.weak();
        view.connect_activate(move |_, parameter| {
            let (Some(this), Some(name)) =
                (this.upgrade(), parameter.and_then(|p| p.get::<String>()))
            else {
                return;
            };
            {
                let mut state = this.state.borrow_mut();
                state.settings.view = match name.as_str() {
                    "grid" => View::Grid,
                    "columns" => View::Columns,
                    _ => View::List,
                };
                save_settings(&state.settings);
            }
            // The folder may be empty, which the views show differently.
            this.repopulate();
        });
        self.window.add_action(&view);
        self.action("preferences", |t| t.preferences());
        self.action("about", |t| {
            ndl::AboutDialog::builder()
                .application_icon("os.nodalix.Archivador")
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
        let (in_trash, in_virtual, can_back, can_forward, can_up) = {
            let state = self.state.borrow();
            (
                is_trash(state.history.current()),
                is_virtual(state.history.current()),
                state.history.can_go_back(),
                state.history.can_go_forward(),
                state.history.parent().is_some() && !is_virtual(state.history.current()),
            )
        };
        let selected = (0..self.store.n_items())
            .filter(|&i| self.selection.is_selected(i))
            .count();
        let some = selected > 0;
        let pasteable = transfer::has_files(&self.window.clipboard());
        self.set_enabled("back", can_back);
        self.set_enabled("forward", can_forward);
        self.set_enabled("up", can_up);
        self.set_enabled("open", some && !in_trash);
        self.set_enabled("open-with", selected == 1 && !in_trash);
        self.set_enabled("properties", some && !in_trash);
        self.set_enabled("copy", some && !in_trash);
        self.set_enabled("cut", some && !in_trash);
        self.set_enabled("rename", selected == 1 && !in_trash);
        self.set_enabled("trash", some && !in_trash);
        self.set_enabled("send", some && !in_trash);
        self.set_enabled("new-folder", !in_virtual);
        self.set_enabled("paste", pasteable && !in_virtual);
        self.set_enabled("restore", some && in_trash);
        self.set_enabled("delete", some);
        self.set_enabled("empty-trash", in_trash && self.store.n_items() > 0);
    }

    // ----- preferences ---------------------------------------------------------------------

    /// The preferences: how Archivador looks, what the sidebar lists, and the cloud accounts.
    fn preferences(self: &Rc<Self>) {
        let settings = self.state.borrow().settings;
        let combo = |title: &'static str, labels: &[&'static str], selected: usize| {
            let labels: Vec<&'static str> = labels.iter().map(|l| tr(l)).collect();
            let row = ndl::ComboRow::builder()
                .title(tr(title))
                .model(&gtk::StringList::new(&labels))
                .build();
            row.set_selected(selected as u32);
            row
        };

        let style = ndl::PreferencesGroup::builder().title(tr("Style")).build();
        let appearance = combo(
            "Appearance",
            &["Follow the system", "Light", "Dark"],
            match settings.appearance {
                Appearance::System => 0,
                Appearance::Light => 1,
                Appearance::Dark => 2,
            },
        );
        let this = self.weak();
        appearance.connect_selected_notify(move |row| {
            let Some(this) = this.upgrade() else { return };
            let value = match row.selected() {
                1 => Appearance::Light,
                2 => Appearance::Dark,
                _ => Appearance::System,
            };
            this.state.borrow_mut().settings.appearance = value;
            this.save();
            apply_appearance(value);
        });
        style.add(&appearance);
        let view = combo(
            "Show files as",
            &["List", "Grid", "Columns"],
            match settings.view {
                View::List => 0,
                View::Grid => 1,
                View::Columns => 2,
            },
        );
        let this = self.weak();
        view.connect_selected_notify(move |row| {
            let Some(this) = this.upgrade() else { return };
            let name = ["list", "grid", "columns"][(row.selected() as usize).min(2)];
            if let Some(action) = this.window.lookup_action("view") {
                action.activate(Some(&name.to_variant()));
            }
        });
        style.add(&view);

        let sidebar = ndl::PreferencesGroup::builder()
            .title(tr("Sidebar"))
            .build();
        let root = ndl::SwitchRow::builder()
            .title(tr("Show the file system"))
            .subtitle("/")
            .active(settings.show_root)
            .build();
        let this = self.weak();
        root.connect_active_notify(move |row| {
            let Some(this) = this.upgrade() else { return };
            this.state.borrow_mut().settings.show_root = row.is_active();
            this.save();
            this.rebuild_sidebar();
            this.sync_sidebar();
        });
        sidebar.add(&root);

        let cloud = ndl::PreferencesGroup::builder()
            .title(tr("Cloud"))
            .description(tr(
                "The accounts you connect show up in the sidebar with their own icon.",
            ))
            .build();
        let accounts = self.clouds.borrow().clone();
        for account in &accounts {
            let connected = is_mounted(&account.mount_point(&crate::home()));
            let row = ndl::ActionRow::builder()
                .title(&account.name)
                .subtitle(format!(
                    "{} · {}",
                    account.provider.title(),
                    if connected {
                        tr("Connected")
                    } else {
                        tr("Not connected")
                    }
                ))
                .build();
            row.add_prefix(&gtk::Image::from_icon_name(account.provider.icon()));
            cloud.add(&row);
        }
        let manage = ndl::ButtonRow::builder()
            .title(tr(if accounts.is_empty() {
                "Set up a cloud account…"
            } else {
                "Add or manage accounts…"
            }))
            .build();
        let this = self.weak();
        manage.connect_activated(move |_| {
            if let Some(this) = this.upgrade() {
                this.open_cloud_settings();
            }
        });
        cloud.add(&manage);

        let page = ndl::PreferencesPage::new();
        page.set_title(tr("Preferences"));
        page.add(&style);
        page.add(&sidebar);
        page.add(&cloud);
        let dialog = ndl::PreferencesDialog::new();
        dialog.set_title(tr("Preferences"));
        dialog.add(&page);
        dialog.present(Some(&self.window));
    }

    /// Opens Settings on its cloud page, where accounts are added and removed.
    fn open_cloud_settings(&self) {
        let started = std::process::Command::new("nodalix-settings")
            .env("NODALIX_SETTINGS_PAGE", "cloud")
            .spawn();
        if started.is_err() {
            self.toast(tr("Can't open Settings"));
        }
    }

    fn save(&self) {
        save_settings(&self.state.borrow().settings);
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
        if !is_virtual(&path) && !path.is_dir() {
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
        } else if is_recent(dir) {
            Ok(recent_entries())
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
        if is_recent(dir) {
            return;
        }
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
            if is_recent(&dir) {
                // The newest visit first, whatever order the folders are kept in.
                shown.sort_by(|a, b| entry::compare(a, b, SortKey::Modified, true));
            } else {
                shown.sort_by(|a, b| entry::compare(a, b, s.sort, s.descending));
            }
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
            let target: gtk::Widget = match view {
                View::List => self.list_view.clone().upcast(),
                View::Grid => self.grid_view.clone().upcast(),
                View::Columns => self.columns_view.clone().upcast(),
            };
            let scroll = move |widget: &gtk::Widget| {
                if let Some(list) = widget.downcast_ref::<gtk::ListView>() {
                    list.scroll_to(position, gtk::ListScrollFlags::FOCUS, None);
                } else if let Some(grid) = widget.downcast_ref::<gtk::GridView>() {
                    grid.scroll_to(position, gtk::ListScrollFlags::FOCUS, None);
                }
            };
            if target.is_mapped() {
                scroll(&target);
            } else {
                // A window that is not on screen yet has no size to scroll in: scroll once it
                // is shown, as when another program opens a folder with a file selected.
                let handler: Rc<Cell<Option<glib::SignalHandlerId>>> = Rc::new(Cell::new(None));
                let id = {
                    let handler = handler.clone();
                    target.connect_map(move |widget| {
                        let widget = widget.clone();
                        if let Some(id) = handler.take() {
                            widget.disconnect(id);
                        }
                        glib::idle_add_local_once(move || scroll(&widget));
                    })
                };
                handler.set(Some(id));
            }
        }

        // The columns view shows an empty folder as an empty column; the rest wants words.
        let plain_empty =
            view == View::Columns && error.is_none() && !searching && !is_virtual(&dir);
        if shown.is_empty() && !plain_empty {
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
            } else if is_recent(&dir) {
                (
                    "document-open-recent-symbolic",
                    tr("No recent files"),
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
        self.update_columns();
        self.update_status();
        self.update_actions();
        self.update_miller();
    }

    /// The options the columns view lists other folders with.
    fn options(&self) -> Options {
        let s = self.state.borrow().settings;
        Options {
            sort: s.sort,
            descending: s.descending,
            show_hidden: s.show_hidden,
        }
    }

    /// Brings the columns to the left and right of the folder up to date.
    fn update_miller(&self) {
        if self.state.borrow().settings.view != View::Columns {
            return;
        }
        let dir = self.current_dir();
        let options = self.options();
        self.miller.show_path(&dir, options, is_virtual(&dir));
        self.miller.show_selection(&self.selected(), options);
    }

    fn apply_view(&self) {
        let view = self.state.borrow().settings.view;
        let name = match view {
            View::List => "list",
            View::Grid => "grid",
            View::Columns => "columns",
        };
        self.view_button.set_icon_name(match view {
            View::List => "view-list-symbolic",
            View::Grid => "view-grid-symbolic",
            View::Columns => "view-columns-symbolic",
        });
        if let Some(action) = self
            .window
            .lookup_action("view")
            .and_downcast::<gio::SimpleAction>()
        {
            action.set_state(&name.to_variant());
        }
        if self.store.n_items() > 0 || view == View::Columns {
            self.pages.set_visible_child_name(name);
        }
        self.update_miller();
        self.update_columns();
        self.update_status();
    }

    // ----- the chrome around the folder ----------------------------------------------------

    fn show_title(&self, dir: &Path) {
        let title = if is_trash(dir) {
            tr("Trash").to_owned()
        } else if is_recent(dir) {
            tr("Recent").to_owned()
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
        let crumbs = if is_virtual(dir) {
            vec![places::Crumb {
                label: Some(tr(if is_trash(dir) { "Trash" } else { "Recent" }).to_owned()),
                path: dir.to_path_buf(),
            }]
        } else {
            places::breadcrumbs(dir, &crate::home())
        };
        // What stands for the segments that do not fit.
        self.path_more.add_css_class("archivador-more");
        self.path_more
            .set_tooltip_text(Some(tr("Show the folders above")));
        self.path_more.set_visible(false);
        self.path_bar.append(&self.path_more);
        let mut items = Vec::new();
        let last = crumbs.len().saturating_sub(1);
        for (i, crumb) in crumbs.into_iter().enumerate() {
            let arrow = (i > 0).then(|| {
                let arrow = gtk::Image::from_icon_name("go-next-symbolic");
                self.path_bar.append(&arrow);
                arrow
            });
            let label = crumb.label.unwrap_or_else(|| tr("Home").to_owned());
            let button = gtk::Button::with_label(&label);
            if i == last {
                button.add_css_class("current");
            }
            let this = self.weak();
            let target = crumb.path.clone();
            button.connect_clicked(move |_| {
                if let Some(this) = this.upgrade() {
                    this.navigate(target.clone());
                }
            });
            self.path_bar.append(&button);
            items.push(PathItem {
                arrow,
                button,
                path: crumb.path,
            });
        }
        self.path_items.replace(items);
        self.path_width.set(0);
        self.fit_path_bar();
    }

    /// Hides the segments from the start that do not fit, and shows `…` in their place, so the
    /// bar never cuts a name in half: the folder you are in is always whole.
    fn fit_path_bar(&self) {
        let available = self.path_scroll.width();
        if available <= 0 {
            return;
        }
        self.path_width.set(available);
        let items = self.path_items.borrow();
        for item in items.iter() {
            item.button.set_visible(true);
            if let Some(arrow) = &item.arrow {
                arrow.set_visible(true);
            }
        }
        self.path_more.set_visible(false);
        let natural = || self.path_bar.measure(gtk::Orientation::Horizontal, -1).1;
        let mut hidden = 0;
        while natural() > available && hidden + 1 < items.len() {
            items[hidden].button.set_visible(false);
            if let Some(arrow) = &items[hidden].arrow {
                arrow.set_visible(false);
            }
            hidden += 1;
            self.path_more.set_visible(true);
        }
        if hidden > 0 {
            self.path_more_target
                .replace(items[hidden - 1].path.clone());
        }
    }

    fn sidebar_row(&self, title: &str, icon: &str, path: &str) -> (gtk::ListBoxRow, gtk::Image) {
        let row = gtk::ListBoxRow::new();
        row.set_widget_name(path);
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let image = gtk::Image::from_icon_name(icon);
        image.set_pixel_size(22);
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
        row.add_css_class("archivador-heading");
        let label = gtk::Label::new(Some(&text.to_uppercase()));
        label.add_css_class("ndl-micro");
        label.set_xalign(0.0);
        label.set_margin_top(12);
        label.set_margin_start(6);
        row.set_child(Some(&label));
        row
    }

    /// Places, drives and the trash.
    fn rebuild_sidebar(self: &Rc<Self>) {
        while let Some(row) = self.sidebar.row_at_index(0) {
            self.sidebar.remove(&row);
        }
        let home = crate::home();
        let dirs = std::fs::read_to_string(glib::user_config_dir().join("user-dirs.dirs"))
            .unwrap_or_default();
        let layout = std::fs::read_to_string(state_dir().join("nodalix/layout/paths.json"))
            .unwrap_or_default();
        let applications = places::applications(&home, &layout, Path::is_dir);
        let (row, _) = self.sidebar_row(tr("Recent"), "document-open-recent-symbolic", RECENT);
        self.sidebar.append(&row);
        for place in places::standard(&home, &dirs, Path::is_dir)
            .into_iter()
            .chain(applications)
        {
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

        // The cloud accounts Settings has set up. They are read first: a folder pinned from one of
        // them is shown with that service's icon, not with a star.
        let accounts = std::fs::read_to_string(glib::user_config_dir().join("nodalix/cloud.json"))
            .map(|text| cloud::parse(&text))
            .unwrap_or_default();

        // The folders the user pinned.
        let pinned: Vec<Bookmark> = load_bookmarks()
            .into_iter()
            .filter(|b| b.path.is_dir())
            .collect();
        if !pinned.is_empty() {
            self.sidebar.append(&Self::heading(tr("Favorites")));
        }
        for bookmark in &pinned {
            let icon = cloud::provider_of(&accounts, &crate::home(), &bookmark.path)
                .map_or("starred-symbolic", cloud::Provider::icon);
            let (row, _) =
                self.sidebar_row(&bookmark.label, icon, &bookmark.path.to_string_lossy());
            self.add_unpin(&row, &bookmark.path);
            self.sidebar.append(&row);
        }

        // The drives that are mounted, and the file system itself.
        let devices = Self::heading(tr("Devices"));
        self.sidebar.append(&devices);
        let before = self.sidebar.observe_children().n_items();
        if self.state.borrow().settings.show_root {
            let (row, _) = self.sidebar_row("/", "drive-harddisk-symbolic", "/");
            self.sidebar.append(&row);
        }
        let mut network = Vec::new();
        for mount in gio::VolumeMonitor::get().mounts() {
            let Some(path) = mount.root().path() else {
                continue;
            };
            if mount.is_shadowed() || path == Path::new("/") {
                continue;
            }
            if mount.root().uri_scheme().is_some_and(|s| s != "file") {
                network.push((mount, path));
                continue;
            }
            let (row, icon) = self.sidebar_row(
                &mount.name(),
                "drive-removable-media-symbolic",
                &path.to_string_lossy(),
            );
            icon.set_from_gicon(&mount.symbolic_icon());
            if mount.can_unmount() || mount.can_eject() {
                self.add_eject(&row, &mount);
            }
            self.sidebar.append(&row);
        }

        // A heading with nothing under it is left out.
        if self.sidebar.observe_children().n_items() == before {
            self.sidebar.remove(&devices);
        }

        // The servers: the ones that are mounted, the ones used before, and a way to add one.
        self.sidebar.append(&Self::heading(tr("Network")));
        let mut shown = Vec::new();
        for (mount, path) in &network {
            let uri = mount.root().uri().to_string();
            shown.push(uri.trim_end_matches('/').to_owned());
            let (row, icon) = self.sidebar_row(
                &mount.name(),
                "network-server-symbolic",
                &path.to_string_lossy(),
            );
            icon.set_from_gicon(&mount.symbolic_icon());
            if mount.can_unmount() || mount.can_eject() {
                self.add_eject(&row, mount);
            }
            self.sidebar.append(&row);
        }
        for uri in load_servers() {
            if shown.contains(&uri.trim_end_matches('/').to_owned()) {
                continue;
            }
            let (row, _) = self.sidebar_row(
                &servers::label(&uri),
                "network-server-symbolic",
                &format!("{SERVER}{uri}"),
            );
            row.set_tooltip_text(Some(tr("Not connected. Click to connect")));
            row.add_css_class("archivador-offline");
            self.sidebar.append(&row);
        }
        let (row, _) = self.sidebar_row(
            tr("Connect to a server…"),
            "network-workgroup-symbolic",
            CONNECT,
        );
        self.sidebar.append(&row);

        if !accounts.is_empty() {
            self.sidebar.append(&Self::heading(tr("Cloud")));
        }
        for account in &accounts {
            let point = account.mount_point(&crate::home());
            let (row, _) = self.sidebar_row(
                &account.name,
                account.provider.icon(),
                &point.to_string_lossy(),
            );
            if is_mounted(&point) {
                self.add_disconnect(&row, account);
            } else {
                row.set_tooltip_text(Some(tr("Not connected. Click to connect")));
                row.add_css_class("archivador-offline");
            }
            self.sidebar.append(&row);
        }
        self.clouds.replace(accounts);
    }

    fn ask_server(self: &Rc<Self>) {
        let this = self.weak();
        dialogs::ask_server(&self.window, move |uri| {
            if let Some(this) = this.upgrade() {
                this.connect_server(uri);
            }
        });
    }

    /// Mounts the server, which may ask for a password, remembers it, and goes into it.
    fn connect_server(self: &Rc<Self>, uri: String) {
        self.working(true);
        let (this, window) = (self.weak(), self.window.clone());
        glib::spawn_future_local(async move {
            let file = gio::File::for_uri(&uri);
            let operation = gtk::MountOperation::new(Some(&window));
            let result = file
                .mount_enclosing_volume_future(gio::MountMountFlags::NONE, Some(&operation))
                .await;
            let Some(this) = this.upgrade() else { return };
            this.working(false);
            let name = servers::label(&uri);
            let mounted = match &result {
                Ok(()) => true,
                Err(error) => error.matches(gio::IOErrorEnum::AlreadyMounted),
            };
            match (mounted, file.path(), result) {
                (true, Some(path), _) => {
                    save_servers(&servers::remember(&load_servers(), &uri));
                    this.rebuild_sidebar();
                    this.navigate(path);
                }
                (true, None, _) => this.toast(&fmt("Can't browse {}", &name)),
                // The user closed the password box: nothing went wrong.
                (false, _, Err(error)) if error.matches(gio::IOErrorEnum::FailedHandled) => {
                    this.sync_sidebar();
                }
                (false, _, Err(error)) => this.toast(&format!(
                    "{}: {}",
                    fmt("Can't connect {}", &name),
                    error.message()
                )),
                (false, _, Ok(())) => {}
            }
        });
    }

    /// The button at the end of a favourite's row that takes it off the sidebar.
    fn add_unpin(self: &Rc<Self>, row: &gtk::ListBoxRow, path: &Path) {
        let Some(content) = row.child().and_downcast::<gtk::Box>() else {
            return;
        };
        let button = gtk::Button::from_icon_name("window-close-symbolic");
        button.add_css_class("flat");
        button.add_css_class("archivador-eject");
        button.set_tooltip_text(Some(tr("Remove from Favorites")));
        button.set_valign(gtk::Align::Center);
        let (this, path) = (self.weak(), path.to_path_buf());
        button.connect_clicked(move |_| {
            save_bookmarks(&bookmarks::remove(load_bookmarks(), &path));
            if let Some(this) = this.upgrade() {
                this.rebuild_sidebar();
                this.sync_sidebar();
            }
        });
        content.append(&button);
    }

    /// Starts the service that mounts the account, then shows it.
    fn connect_cloud(self: &Rc<Self>, account: cloud::Account) {
        self.working(true);
        let this = self.weak();
        glib::spawn_future_local(async move {
            let unit = account.unit();
            let result = gio::spawn_blocking(move || systemctl("start", &unit)).await;
            let Some(this) = this.upgrade() else { return };
            this.working(false);
            // The row was selected by the click; the folder shown has not changed.
            this.sync_sidebar();
            let point = account.mount_point(&crate::home());
            match result {
                Ok(Ok(())) if is_mounted(&point) => {
                    this.rebuild_sidebar();
                    this.navigate(point);
                }
                Ok(Ok(())) => {
                    this.toast(&fmt("Can't connect {}", &account.name));
                }
                Ok(Err(message)) => {
                    this.toast(&format!(
                        "{}: {message}",
                        fmt("Can't connect {}", &account.name)
                    ));
                }
                Err(_) => this.toast(&fmt("Can't connect {}", &account.name)),
            }
        });
    }

    /// The button at the end of a connected account's row that stops its mount.
    fn add_disconnect(self: &Rc<Self>, row: &gtk::ListBoxRow, account: &cloud::Account) {
        let Some(content) = row.child().and_downcast::<gtk::Box>() else {
            return;
        };
        let button = gtk::Button::from_icon_name("media-eject-symbolic");
        button.add_css_class("flat");
        button.add_css_class("archivador-eject");
        button.set_tooltip_text(Some(tr("Disconnect")));
        button.set_valign(gtk::Align::Center);
        let (this, account) = (self.weak(), account.clone());
        button.connect_clicked(move |_| {
            let (this, account) = (this.clone(), account.clone());
            let Some(strong) = this.upgrade() else { return };
            // Leave the mount before it goes, or the window is left showing a folder that is gone.
            let point = account.mount_point(&crate::home());
            if strong.current_dir().starts_with(&point) {
                strong.navigate(crate::home());
            }
            strong.working(true);
            glib::spawn_future_local(async move {
                let unit = account.unit();
                let result = gio::spawn_blocking(move || systemctl("stop", &unit)).await;
                let Some(this) = this.upgrade() else { return };
                this.working(false);
                if let Ok(Err(message)) = result {
                    this.toast(&format!(
                        "{}: {message}",
                        fmt("Can't disconnect {}", &account.name)
                    ));
                }
                this.rebuild_sidebar();
                this.sync_sidebar();
            });
        });
        content.append(&button);
    }

    /// The button at the end of a drive's row that unmounts it, and ejects it when it can.
    fn add_eject(self: &Rc<Self>, row: &gtk::ListBoxRow, mount: &gio::Mount) {
        let Some(content) = row.child().and_downcast::<gtk::Box>() else {
            return;
        };
        let button = gtk::Button::from_icon_name("media-eject-symbolic");
        button.add_css_class("flat");
        button.add_css_class("archivador-eject");
        button.set_tooltip_text(Some(tr("Eject")));
        button.set_valign(gtk::Align::Center);
        let (this, mount) = (self.weak(), mount.clone());
        button.connect_clicked(move |_| {
            let (this, name) = (this.clone(), mount.name().to_string());
            let done = move |result: Result<(), glib::Error>| {
                if let (Err(error), Some(this)) = (result, this.upgrade()) {
                    this.toast(&format!(
                        "{}: {}",
                        fmt("Can't unmount {}", &name),
                        error.message()
                    ));
                }
            };
            let none = None::<&gio::MountOperation>;
            let cancel = None::<&gio::Cancellable>;
            if mount.can_eject() {
                mount.eject_with_operation(gio::MountUnmountFlags::NONE, none, cancel, done);
            } else {
                mount.unmount_with_operation(gio::MountUnmountFlags::NONE, none, cancel, done);
            }
        });
        content.append(&button);
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
        if is_virtual(&dir) {
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
        // The system clipboard, so that other apps can paste what is copied here.
        let _ = self
            .window
            .clipboard()
            .set_content(Some(&transfer::clipboard_content(mode, &paths)));
        self.update_actions();
    }

    fn paste(self: &Rc<Self>) {
        let dir = self.current_dir();
        if is_virtual(&dir) {
            return;
        }
        let this = self.weak();
        let system = self.window.clipboard();
        glib::spawn_future_local(async move {
            let Some(clipboard) = transfer::read(&system).await else {
                return;
            };
            let Some(this) = this.upgrade() else { return };
            let cut = clipboard.mode == Mode::Cut;
            let paths = this.transfer(clipboard, dir).await;
            if cut && paths.is_some() {
                // What was moved is no longer where the clipboard says it is.
                let _ = system.set_content(None::<&gdk::ContentProvider>);
            }
        });
    }

    /// Puts the files of `clipboard` in `dir`, off the main thread, and says how it went. Returns
    /// where they ended up, or nothing if it failed.
    async fn transfer(self: &Rc<Self>, clipboard: Clipboard, dir: PathBuf) -> Option<Vec<PathBuf>> {
        self.working(true);
        let target = dir.clone();
        let result = gio::spawn_blocking(move || clipboard.paste(&target)).await;
        self.working(false);
        match result {
            Ok(Ok(pasted)) => {
                if self.current_dir() == dir {
                    self.state.borrow_mut().select = pasted.clone();
                    self.reload();
                }
                self.update_actions();
                Some(pasted)
            }
            Ok(Err(error)) => {
                self.reload();
                self.toast(&error.to_string());
                None
            }
            Err(_) => {
                self.toast("?");
                None
            }
        }
    }

    /// Files dropped on a folder, a place or the trash. Moves them, or copies when asked to.
    fn drop_files(self: &Rc<Self>, paths: Vec<PathBuf>, target: PathBuf, copy: bool) -> bool {
        if is_trash(&target) {
            self.trash_paths(paths);
            return true;
        }
        if is_recent(&target) {
            return false;
        }
        let paths: Vec<PathBuf> = paths
            .into_iter()
            // Not onto itself or into itself, and a move into the folder it is in does nothing.
            .filter(|p| !target.starts_with(p) && (copy || p.parent() != Some(target.as_path())))
            .collect();
        if paths.is_empty() || !target.is_dir() {
            return false;
        }
        let mode = if copy { Mode::Copy } else { Mode::Cut };
        let this = self.clone();
        glib::spawn_future_local(async move {
            this.transfer(Clipboard::new(mode, paths), target).await;
        });
        true
    }

    fn trash_selected(self: &Rc<Self>) {
        if is_trash(&self.current_dir()) {
            return;
        }
        let paths: Vec<PathBuf> = self.selected().into_iter().map(|e| e.path).collect();
        self.trash_paths(paths);
    }

    fn trash_paths(self: &Rc<Self>, paths: Vec<PathBuf>) {
        if paths.is_empty() {
            return;
        }
        let date = glib::DateTime::now_local()
            .ok()
            .and_then(|d| d.format("%Y-%m-%dT%H:%M:%S").ok())
            .map(|s| s.to_string())
            .unwrap_or_default();
        let bin = trash();
        let mut moved = 0;
        let mut refused: Vec<(PathBuf, String)> = Vec::new();
        for path in paths {
            match bin.trash(&path, &date) {
                Ok(()) => moved += 1,
                Err(error) => refused.push((path, error.to_string())),
            }
        }
        self.reload();
        if moved > 0 {
            self.toast(&fmt("Moved {} to the trash", &items(moved)));
        }
        if let Some((path, error)) = refused.first() {
            let paths: Vec<PathBuf> = refused.iter().map(|(p, _)| p.clone()).collect();
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let body = format!(
                "{name}: {error}\n\n{}",
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
        self.working(true);
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
            this.working(false);
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
                this.working(true);
                glib::spawn_future_local(async move {
                    let result = gio::spawn_blocking(|| trash().empty()).await;
                    let Some(this) = again.upgrade() else { return };
                    this.working(false);
                    this.reload();
                    if let Ok(Err(error)) = result {
                        this.toast(&error.to_string());
                    }
                });
            },
        );
    }
}
