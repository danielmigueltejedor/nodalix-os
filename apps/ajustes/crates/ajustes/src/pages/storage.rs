//! Storage: how full the disk is, what fills it (applications, documents, music, pictures and
//! videos, downloads, development files) and the biggest items of each, which can be shown in the
//! file manager or removed. Measuring is the storage helper's job (it needs a walk of the home
//! folder), so it runs in the background and the page shows the figures when they arrive.

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;

use nodalia_gtk::{
    self as ndl, gtk,
    gtk::{gio, glib, prelude::*},
};

use ajustes_core::storage::{Item, Report, CATEGORIES};
use monitor_core::format_bytes;

use super::cmd::capture;
use super::{column, scroller};
use crate::i18n::tr;

const HELPER: &str = "/usr/lib/nodalix/storage-manager";
/// How many items of a kind are listed before "Show all".
const SHORT: usize = 6;

/// The helper's path; a test can point it at a stand-in.
fn helper() -> String {
    std::env::var("NODALIX_STORAGE_HELPER").unwrap_or_else(|_| HELPER.to_owned())
}

fn title_of(key: &str) -> &'static str {
    match key {
        "apps" => tr("Applications"),
        "documents" => tr("Documents"),
        "music" => tr("Music"),
        "media" => tr("Pictures and videos"),
        "downloads" => tr("Downloads"),
        _ => tr("Development"),
    }
}

fn icon_of(key: &str) -> &'static str {
    CATEGORIES
        .iter()
        .find(|(k, _)| *k == key)
        .map_or("drive-harddisk-symbolic", |(_, icon)| icon)
}

struct Page {
    toasts: RefCell<Option<ndl::ToastOverlay>>,
    widget: gtk::ScrolledWindow,
    content: gtk::Box,
    report: RefCell<Option<Report>>,
    status: RefCell<Option<String>>,
    expanded: RefCell<HashSet<&'static str>>,
    scanning: Cell<bool>,
}

fn bar(fraction: f64) -> gtk::ProgressBar {
    let bar = gtk::ProgressBar::new();
    bar.set_fraction(fraction);
    bar.set_valign(gtk::Align::Center);
    bar.set_size_request(180, -1);
    bar
}

fn size_label(size: Option<u64>) -> gtk::Label {
    let label = gtk::Label::new(Some(
        &size.map_or_else(|| tr("Unknown").to_owned(), format_bytes),
    ));
    label.add_css_class("dim-label");
    label.add_css_class("numeric");
    label.set_valign(gtk::Align::Center);
    label
}

impl Page {
    fn toast(&self, text: &str) {
        if let Some(toasts) = &*self.toasts.borrow() {
            toasts.add_toast(ndl::Toast::new(text));
        }
    }

    fn scan(self: &Rc<Self>) {
        if self.scanning.replace(true) {
            return;
        }
        *self.status.borrow_mut() = Some(tr("Measuring…").to_owned());
        self.render();
        let page = self.clone();
        glib::spawn_future_local(async move {
            let helper = helper();
            let output = capture(&[&helper, "scan"], None).await;
            *page.status.borrow_mut() = None;
            if output.ok {
                match Report::parse(&output.out) {
                    Ok(report) => *page.report.borrow_mut() = Some(report),
                    Err(_) => {
                        *page.status.borrow_mut() =
                            Some(tr("The storage tool gave no answer").to_owned());
                    }
                }
            } else {
                *page.status.borrow_mut() =
                    Some(tr("The storage tool is not available").to_owned());
            }
            page.scanning.set(false);
            page.render();
        });
    }

    fn render(self: &Rc<Self>) {
        while let Some(child) = self.content.first_child() {
            self.content.remove(&child);
        }
        let top = ndl::PreferencesGroup::new();
        let report = self.report.borrow().clone();
        let disk = ndl::ActionRow::builder().title(tr("Disk")).build();
        disk.add_prefix(&gtk::Image::from_icon_name("drive-harddisk-symbolic"));
        if let Some(report) = &report {
            disk.set_title(&format!(
                "{} {} {}",
                format_bytes(report.volume.used),
                tr("used of"),
                format_bytes(report.volume.total)
            ));
            disk.set_subtitle(&format!(
                "{} {}",
                format_bytes(report.volume.free),
                tr("free")
            ));
            disk.add_suffix(&bar(report.volume.fraction()));
        }
        if let Some(status) = &*self.status.borrow() {
            disk.set_subtitle(status);
        }
        let refresh = gtk::Button::from_icon_name("view-refresh-symbolic");
        refresh.set_valign(gtk::Align::Center);
        refresh.add_css_class("flat");
        refresh.set_tooltip_text(Some(tr("Measure again")));
        refresh.set_sensitive(!self.scanning.get());
        {
            let page = self.clone();
            refresh.connect_clicked(move |_| page.scan());
        }
        disk.add_suffix(&refresh);
        top.add(&disk);
        self.content.append(&top);
        let Some(report) = report else {
            return;
        };

        let used = report.volume.used.max(1);
        let split = ndl::PreferencesGroup::builder()
            .title(tr("What is using the space"))
            .build();
        for (key, icon) in CATEGORIES {
            let row = ndl::ActionRow::builder().title(title_of(key)).build();
            row.add_prefix(&gtk::Image::from_icon_name(icon));
            row.add_suffix(&size_label(Some(report.total(key))));
            row.add_suffix(&bar(report.total(key) as f64 / used as f64));
            split.add(&row);
        }
        let other = ndl::ActionRow::builder()
            .title(tr("System and other"))
            .build();
        other.add_prefix(&gtk::Image::from_icon_name("emblem-system-symbolic"));
        other.add_suffix(&size_label(Some(report.other())));
        other.add_suffix(&bar(report.other() as f64 / used as f64));
        split.add(&other);
        self.content.append(&split);
        if report.partial {
            let note = gtk::Label::new(Some(tr(
                "There are so many files that the figures are a minimum.",
            )));
            note.add_css_class("dim-label");
            note.set_xalign(0.0);
            note.set_wrap(true);
            self.content.append(&note);
        }

        for (key, _) in CATEGORIES {
            let items = report.items(key);
            if items.is_empty() {
                continue;
            }
            let group = ndl::PreferencesGroup::builder()
                .title(format!(
                    "{} · {}",
                    title_of(key),
                    format_bytes(report.total(key))
                ))
                .build();
            let all = self.expanded.borrow().contains(key);
            for item in items.iter().take(if all { items.len() } else { SHORT }) {
                group.add(&self.item_row(key, item));
            }
            if items.len() > SHORT {
                let more = ndl::ActionRow::builder()
                    .title(tr(if all { "Show fewer" } else { "Show all" }))
                    .build();
                more.set_activatable(true);
                more.add_suffix(&gtk::Image::from_icon_name(if all {
                    "go-up-symbolic"
                } else {
                    "go-down-symbolic"
                }));
                let page = self.clone();
                more.connect_activated(move |_| {
                    {
                        let mut open = page.expanded.borrow_mut();
                        if !open.remove(key) {
                            open.insert(key);
                        }
                    }
                    page.render();
                });
                group.add(&more);
            }
            self.content.append(&group);
        }
    }

    fn item_row(self: &Rc<Self>, key: &'static str, item: &Item) -> ndl::ActionRow {
        let row = ndl::ActionRow::builder().title(item.name.as_str()).build();
        row.set_subtitle(&item.detail);
        let icon = match gio::Icon::for_string(&item.icon) {
            Ok(icon) if !item.icon.is_empty() => gtk::Image::from_gicon(&icon),
            _ => gtk::Image::from_icon_name(icon_of(key)),
        };
        icon.set_pixel_size(24);
        row.add_prefix(&icon);
        row.add_suffix(&size_label(item.size));
        if !item.path.is_empty() {
            let show = gtk::Button::from_icon_name("folder-open-symbolic");
            show.set_valign(gtk::Align::Center);
            show.add_css_class("flat");
            show.set_tooltip_text(Some(tr("Show in the file manager")));
            let path = item.path.clone();
            show.connect_clicked(move |button| {
                let window = button.root().and_downcast::<gtk::Window>();
                let launcher = gtk::FileLauncher::new(Some(&gio::File::for_path(&path)));
                glib::spawn_future_local(async move {
                    let _ = launcher
                        .open_containing_folder_future(window.as_ref())
                        .await;
                });
            });
            row.add_suffix(&show);
        }
        let remove = gtk::Button::from_icon_name("user-trash-symbolic");
        remove.set_valign(gtk::Align::Center);
        remove.add_css_class("flat");
        let is_app = key == "apps";
        remove.set_tooltip_text(Some(tr(if is_app {
            "Remove"
        } else {
            "Move to the trash"
        })));
        let (page, item) = (self.clone(), item.clone());
        remove.connect_clicked(move |button| {
            let (page, item) = (page.clone(), item.clone());
            let window = button.root().and_downcast::<gtk::Window>();
            glib::spawn_future_local(async move {
                let dialog = gtk::AlertDialog::builder()
                    .message(tr(if is_app {
                        "Remove this application?"
                    } else {
                        "Move this to the trash?"
                    }))
                    .detail(item.name.as_str())
                    .buttons([
                        tr("Cancel"),
                        tr(if is_app {
                            "Remove"
                        } else {
                            "Move to the trash"
                        }),
                    ])
                    .cancel_button(0)
                    .default_button(0)
                    .modal(true)
                    .build();
                if dialog.choose_future(window.as_ref()).await == Ok(1) {
                    page.remove(&item, is_app).await;
                }
            });
        });
        row.add_suffix(&remove);
        row
    }

    async fn remove(self: &Rc<Self>, item: &Item, is_app: bool) {
        let helper = helper();
        let (action, target) = if is_app {
            ("trash-app", item.id.as_str())
        } else {
            ("trash-file", item.path.as_str())
        };
        let output = capture(&[&helper, action, target], None).await;
        if output.ok {
            self.toast(tr(if is_app {
                "Application removed"
            } else {
                "Moved to the trash"
            }));
        } else {
            let first = output
                .err
                .lines()
                .next()
                .unwrap_or_default()
                .trim()
                .to_owned();
            self.toast(&if first.is_empty() {
                tr("It could not be removed").to_owned()
            } else {
                first
            });
        }
        self.scan();
    }
}

pub fn build(toasts: &ndl::ToastOverlay) -> gtk::ScrolledWindow {
    let content = column();
    let widget = scroller(&content);
    let page = Rc::new(Page {
        toasts: RefCell::new(Some(toasts.clone())),
        widget: widget.clone(),
        content,
        report: RefCell::new(None),
        status: RefCell::new(None),
        expanded: RefCell::new(HashSet::new()),
        scanning: Cell::new(false),
    });
    page.render();
    // Measuring walks the home folder, so it starts when the page is first shown, not with the
    // window.
    let started = Cell::new(false);
    {
        let page = page.clone();
        widget.connect_map(move |_| {
            if !started.replace(true) {
                page.scan();
            }
        });
    }
    // The page keeps itself alive through its handlers; the scroller owns them.
    let _ = &page.widget;
    widget
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_text_is_translated() {
        for text in [
            "Storage",
            "Disk",
            "used of",
            "free",
            "Measuring…",
            "Measure again",
            "What is using the space",
            "System and other",
            "Applications",
            "Documents",
            "Music",
            "Pictures and videos",
            "Downloads",
            "Development",
            "Show all",
            "Show fewer",
            "Unknown",
            "Show in the file manager",
            "Move to the trash",
            "Remove",
            "Remove this application?",
            "Move this to the trash?",
            "Application removed",
            "Moved to the trash",
            "It could not be removed",
            "The storage tool is not available",
            "The storage tool gave no answer",
            "There are so many files that the figures are a minimum.",
        ] {
            assert!(crate::i18n::has_translation(text), "{text}");
        }
    }
}
