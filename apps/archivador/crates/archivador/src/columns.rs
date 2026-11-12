//! The columns view, the way Finder has it: a column for each folder on the way down to the one
//! being shown, the folder itself, and, to the right, what is selected in it: a folder's own
//! content, or a card with a file's details.
//!
//! The folder being shown is the browser's, with all it can do; the columns to its left are only
//! for going back to a folder on the way, and the one to its right only for looking.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use archivador_core::entry::{self, Entry, SortKey};
use archivador_core::format;
use ndl::gtk::{self, gio, glib, prelude::*};
use nodalia_gtk as ndl;

use crate::i18n::{items, tr};
use crate::items;
use crate::object::FileObject;

/// A folder to show, and the files to select in it.
pub type Go = Rc<dyn Fn(PathBuf, Vec<PathBuf>)>;

/// Opens a file.
pub type Open = Rc<dyn Fn(Entry)>;

/// How the entries of a folder are filtered and ordered.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Options {
    pub sort: SortKey,
    pub descending: bool,
    pub show_hidden: bool,
}

const COLUMN_WIDTH: i32 = 250;

pub struct Columns {
    pub widget: gtk::ScrolledWindow,
    ancestors: gtk::Box,
    preview: gtk::Box,
    go: RefCell<Option<Go>>,
    open: RefCell<Option<Open>>,
    /// What the columns on the left were built for, so a reload of the same folder leaves them be.
    built_for: RefCell<Option<(PathBuf, Options)>>,
    /// What the preview was built for.
    previewed: RefCell<Option<(Vec<Entry>, Options)>>,
    generation: Cell<u32>,
}

impl Columns {
    /// `active` is the column of the folder being shown.
    pub fn new(active: &impl IsA<gtk::Widget>) -> Rc<Self> {
        let ancestors = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let preview = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        preview.set_visible(false);

        let current = gtk::ScrolledWindow::builder()
            .child(active)
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .width_request(COLUMN_WIDTH)
            .build();
        current.add_css_class("archivador-column");

        let strip = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        strip.append(&ancestors);
        strip.append(&current);
        strip.append(&preview);
        let widget = gtk::ScrolledWindow::builder()
            .child(&strip)
            .hexpand(true)
            .vexpand(true)
            .vscrollbar_policy(gtk::PolicyType::Never)
            .build();
        Rc::new(Self {
            widget,
            ancestors,
            preview,
            go: RefCell::new(None),
            open: RefCell::new(None),
            built_for: RefCell::new(None),
            previewed: RefCell::new(None),
            generation: Cell::new(0),
        })
    }

    /// What clicking a folder or file in another column does, and what the Open button does.
    pub fn connect(&self, go: Go, open: Open) {
        self.go.replace(Some(go));
        self.open.replace(Some(open));
    }

    fn go(&self, folder: PathBuf, select: Vec<PathBuf>) {
        let go = self.go.borrow().clone();
        if let Some(go) = go {
            // Not now: the column that was clicked is about to be replaced.
            glib::idle_add_local_once(move || go(folder, select));
        }
    }

    /// Shows the columns for `dir`: every folder from the home folder (or the root) down to it.
    pub fn show_path(self: &Rc<Self>, dir: &Path, options: Options, in_trash: bool) {
        let key = (dir.to_path_buf(), options);
        if self.built_for.borrow().as_ref() == Some(&key) {
            return;
        }
        self.built_for.replace(Some(key));
        self.generation.set(self.generation.get() + 1);
        while let Some(child) = self.ancestors.first_child() {
            self.ancestors.remove(&child);
        }
        let home = crate::home();
        let root = if dir.starts_with(&home) {
            home
        } else {
            PathBuf::from("/")
        };
        if !in_trash && dir.starts_with(&root) {
            let mut chain: Vec<&Path> = dir
                .ancestors()
                .skip(1)
                .take_while(|p| p.starts_with(&root))
                .collect();
            chain.reverse();
            for folder in chain {
                // The folder this one leads to.
                let child = dir
                    .ancestors()
                    .find(|p| p.parent() == Some(folder))
                    .map(Path::to_path_buf);
                self.ancestors
                    .append(&self.folder_column(folder, child, options, false));
            }
        }
        self.scroll_to_end();
    }

    /// Shows what is selected in the folder, in a column to the right of it.
    pub fn show_selection(self: &Rc<Self>, selected: &[Entry], options: Options) {
        let key = (selected.to_vec(), options);
        if self.previewed.borrow().as_ref() == Some(&key) {
            return;
        }
        self.previewed.replace(Some(key));
        while let Some(child) = self.preview.first_child() {
            self.preview.remove(&child);
        }
        let shown: Option<gtk::Widget> = match selected {
            [] => None,
            [one] if one.is_dir => {
                Some(self.folder_column(&one.path, None, options, true).upcast())
            }
            [one] => Some(self.file_card(one).upcast()),
            many => Some(Self::summary(many).upcast()),
        };
        self.preview.set_visible(shown.is_some());
        if let Some(widget) = shown {
            self.preview.append(&widget);
            self.scroll_to_end();
        }
    }

    /// Keeps the end of the strip, where the folder being shown is, in view.
    fn scroll_to_end(&self) {
        let adjustment = self.widget.hadjustment();
        glib::idle_add_local_once(move || {
            adjustment.set_value(adjustment.upper() - adjustment.page_size());
        });
    }

    /// A column that lists `dir`, with `child` (the folder the next column shows) selected.
    fn folder_column(
        self: &Rc<Self>,
        dir: &Path,
        child: Option<PathBuf>,
        options: Options,
        wide: bool,
    ) -> gtk::ScrolledWindow {
        let store = gio::ListStore::new::<FileObject>();
        let selection = gtk::SingleSelection::new(Some(store.clone()));
        selection.set_autoselect(false);
        selection.set_can_unselect(true);
        selection.set_selected(gtk::INVALID_LIST_POSITION);
        let view = gtk::ListView::new(Some(selection.clone()), Some(items::column_factory(None)));
        view.add_css_class("archivador-files");
        if !wide {
            // The way back, not the place you are in: its choice is quieter.
            view.add_css_class("archivador-trail");
        }
        view.set_single_click_activate(true);
        let this = Rc::downgrade(self);
        let store_for_click = store.clone();
        view.connect_activate(move |_, position| {
            let Some(this) = this.upgrade() else { return };
            let Some(file) = store_for_click.item(position).and_downcast::<FileObject>() else {
                return;
            };
            let entry = file.entry();
            if entry.is_dir {
                this.go(entry.path, Vec::new());
            } else if let Some(parent) = entry.path.parent() {
                this.go(parent.to_path_buf(), vec![entry.path]);
            }
        });
        let column = gtk::ScrolledWindow::builder()
            .child(&view)
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .width_request(if wide {
                COLUMN_WIDTH + 30
            } else {
                COLUMN_WIDTH
            })
            .build();
        column.add_css_class("archivador-column");

        let dir = dir.to_path_buf();
        let generation = self.generation.get();
        let this = Rc::downgrade(self);
        glib::spawn_future_local(async move {
            let read = {
                let dir = dir.clone();
                gio::spawn_blocking(move || entry::read_dir(&dir)).await
            };
            let Some(this) = this.upgrade() else { return };
            let Ok(Ok(mut entries)) = read else { return };
            // A newer set of columns replaced this one while it was read.
            if !wide && this.generation.get() != generation {
                return;
            }
            entries.retain(|e| entry::matches(e, options.show_hidden, ""));
            entries.sort_by(|a, b| entry::compare(a, b, options.sort, options.descending));
            let at = child
                .as_ref()
                .and_then(|child| entries.iter().position(|e| &e.path == child));
            let objects: Vec<FileObject> = entries.into_iter().map(FileObject::new).collect();
            store.splice(0, store.n_items(), &objects);
            if let Some(at) = at {
                selection.set_selected(at as u32);
                view.scroll_to(at as u32, gtk::ListScrollFlags::NONE, None);
            }
        });
        column
    }

    /// The details of a file: a large icon or the picture itself, its name, what it is, how big.
    fn file_card(self: &Rc<Self>, entry: &Entry) -> gtk::Box {
        let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
        card.add_css_class("archivador-card");
        card.set_width_request(COLUMN_WIDTH + 50);
        let icon = gtk::Image::new();
        icon.set_pixel_size(128);
        items::show_icon(&icon, entry, 256);
        icon.set_margin_bottom(8);
        let name = gtk::Label::new(Some(&entry.name));
        name.add_css_class("title-4");
        name.set_wrap(true);
        name.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        name.set_justify(gtk::Justification::Center);
        name.set_max_width_chars(28);
        name.set_selectable(true);
        let (content_type, _) = gio::content_type_guess(Some(&entry.name), None::<&[u8]>);
        let kind = gtk::Label::new(Some(&gio::content_type_get_description(&content_type)));
        kind.add_css_class("archivador-meta");
        let details = [
            Some(format::size(entry.size)),
            Some(items::modified_text(entry.modified)).filter(|m| !m.is_empty()),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
        let details = gtk::Label::new(Some(&details));
        details.add_css_class("archivador-meta");
        let open = gtk::Button::with_label(tr("Open"));
        open.add_css_class("pill");
        open.set_halign(gtk::Align::Center);
        open.set_margin_top(12);
        let this = Rc::downgrade(self);
        let file = entry.clone();
        open.connect_clicked(move |_| {
            let Some(this) = this.upgrade() else { return };
            let open = this.open.borrow().clone();
            if let Some(open) = open {
                open(file.clone());
            }
        });
        for widget in [
            icon.upcast_ref::<gtk::Widget>(),
            name.upcast_ref(),
            kind.upcast_ref(),
            details.upcast_ref(),
            open.upcast_ref(),
        ] {
            card.append(widget);
        }
        card
    }

    /// For several things selected: how many, and how big.
    fn summary(selected: &[Entry]) -> gtk::Box {
        let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
        card.add_css_class("archivador-card");
        card.set_width_request(COLUMN_WIDTH + 50);
        let icon = gtk::Image::from_icon_name("edit-copy-symbolic");
        icon.set_pixel_size(48);
        let title = gtk::Label::new(Some(&items(selected.len())));
        title.add_css_class("title-4");
        let bytes: u64 = selected.iter().filter(|e| !e.is_dir).map(|e| e.size).sum();
        card.append(&icon);
        card.append(&title);
        if bytes > 0 {
            let size = gtk::Label::new(Some(&format::size(bytes)));
            size.add_css_class("archivador-meta");
            card.append(&size);
        }
        card
    }
}
