//! The process table.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::rc::Rc;

use monitor_core::format_bytes;
use monitor_core::procs::{self, Signal};
use nodalia_gtk::gtk::{self, gio, glib, prelude::*};

use crate::i18n::tr;
use crate::process_object::{ProcessObject, Row};
use crate::sampler::Snapshot;

pub struct ProcessView {
    pub widget: gtk::Box,
    store: gio::ListStore,
    by_pid: RefCell<HashMap<u32, ProcessObject>>,
    users: HashMap<u32, String>,
    view: gtk::ColumnView,
    selection: gtk::SingleSelection,
    pub search: gtk::SearchEntry,
}

type Cmp = fn(&Row, &Row) -> Ordering;

fn column(
    title: &str,
    expand: bool,
    numeric: bool,
    cmp: Cmp,
    text: fn(&Row) -> String,
) -> gtk::ColumnViewColumn {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(move |_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().expect("a list item");
        let label = gtk::Label::new(None);
        label.set_xalign(if numeric { 1.0 } else { 0.0 });
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        if numeric {
            label.add_css_class("numeric");
        }
        item.set_child(Some(&label));
    });
    factory.connect_bind(move |_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().expect("a list item");
        let label = item
            .child()
            .and_downcast::<gtk::Label>()
            .expect("the label made in setup");
        let item_expr = item.property_expression("item");
        let revision = gtk::PropertyExpression::new(
            ProcessObject::static_type(),
            Some(&item_expr),
            "revision",
        );
        let params: [gtk::Expression; 2] = [item_expr.clone().upcast(), revision.upcast()];
        let expr = gtk::ClosureExpression::new::<String>(
            params,
            glib::closure!(move |_this: Option<glib::Object>,
                                 obj: Option<glib::Object>,
                                 _revision: u32| {
                obj.and_downcast::<ProcessObject>()
                    .map(|o| text(&o.row()))
                    .unwrap_or_default()
            }),
        );
        expr.bind(&label, "label", Some(item));
    });
    let column = gtk::ColumnViewColumn::new(Some(title), Some(factory));
    column.set_expand(expand);
    column.set_resizable(true);
    column.set_sorter(Some(&gtk::CustomSorter::new(move |a, b| {
        let (Some(a), Some(b)) = (
            a.downcast_ref::<ProcessObject>(),
            b.downcast_ref::<ProcessObject>(),
        ) else {
            return gtk::Ordering::Equal;
        };
        cmp(&a.row(), &b.row()).into()
    })));
    column
}

impl ProcessView {
    pub fn new() -> Rc<Self> {
        let store = gio::ListStore::new::<ProcessObject>();
        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some(tr("Search processes")));
        search.set_hexpand(true);

        let query = Rc::new(RefCell::new(String::new()));
        let filter = {
            let query = query.clone();
            gtk::CustomFilter::new(move |obj| {
                let q = query.borrow();
                if q.is_empty() {
                    return true;
                }
                obj.downcast_ref::<ProcessObject>().is_some_and(|o| {
                    let r = o.row();
                    r.name.to_lowercase().contains(q.as_str())
                        || r.user.to_lowercase().contains(q.as_str())
                        || r.cmdline.to_lowercase().contains(q.as_str())
                        || r.pid.to_string() == *q
                })
            })
        };
        let filtered = gtk::FilterListModel::new(Some(store.clone()), Some(filter.clone()));

        let view = gtk::ColumnView::new(None::<gtk::SingleSelection>);
        let sorted = gtk::SortListModel::new(Some(filtered), view.sorter());
        let selection = gtk::SingleSelection::new(Some(sorted));
        selection.set_autoselect(false);
        selection.set_can_unselect(true);
        view.set_model(Some(&selection));
        view.set_show_row_separators(false);
        view.set_vexpand(true);

        let cols: [gtk::ColumnViewColumn; 7] = [
            column(
                tr("Name"),
                true,
                false,
                |a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                |r| r.name.clone(),
            ),
            column(
                tr("User"),
                false,
                false,
                |a, b| a.user.cmp(&b.user),
                |r| r.user.clone(),
            ),
            column(
                tr("% CPU"),
                false,
                true,
                |a, b| a.cpu.total_cmp(&b.cpu),
                |r| format!("{:.1}", r.cpu),
            ),
            column(
                tr("Memory"),
                false,
                true,
                |a, b| a.rss.cmp(&b.rss),
                |r| format_bytes(r.rss),
            ),
            column(
                tr("ID"),
                false,
                true,
                |a, b| a.pid.cmp(&b.pid),
                |r| r.pid.to_string(),
            ),
            column(
                tr("Threads"),
                false,
                true,
                |a, b| a.threads.cmp(&b.threads),
                |r| r.threads.to_string(),
            ),
            column(
                tr("State"),
                false,
                false,
                |a, b| a.state.cmp(&b.state),
                |r| tr(procs::state_label(r.state)).to_string(),
            ),
        ];
        for c in &cols {
            view.append_column(c);
        }
        // Heaviest first, like every system monitor.
        view.sort_by_column(Some(&cols[2]), gtk::SortType::Descending);

        {
            let (query, filter) = (query.clone(), filter.clone());
            search.connect_search_changed(move |entry| {
                *query.borrow_mut() = entry.text().to_lowercase();
                filter.changed(gtk::FilterChange::Different);
            });
        }

        let scroller = gtk::ScrolledWindow::builder()
            .child(&view)
            .vexpand(true)
            .build();
        scroller.add_css_class("monitor-table");
        let widget = gtk::Box::new(gtk::Orientation::Vertical, 8);
        widget.set_margin_start(8);
        widget.set_margin_end(8);
        widget.set_margin_bottom(8);
        widget.append(&search);
        widget.append(&scroller);

        Rc::new(Self {
            widget,
            store,
            by_pid: RefCell::new(HashMap::new()),
            users: procs::read_users(),
            view,
            selection,
            search,
        })
    }

    /// Brings the table in line with a reading: update in place, add the new, drop the gone.
    pub fn update(&self, snapshot: &Snapshot) {
        let mut by_pid = self.by_pid.borrow_mut();
        let mut seen = std::collections::HashSet::with_capacity(snapshot.processes.len());
        let mut added = Vec::new();
        for (p, cpu) in &snapshot.processes {
            seen.insert(p.pid);
            let row = Row {
                pid: p.pid,
                name: p.name.clone(),
                user: self
                    .users
                    .get(&p.uid)
                    .cloned()
                    .unwrap_or_else(|| p.uid.to_string()),
                cpu: *cpu,
                rss: p.rss,
                threads: p.threads,
                state: p.state,
                cmdline: p.cmdline.clone(),
            };
            match by_pid.get(&p.pid) {
                Some(obj) => obj.update(row),
                None => {
                    let obj = ProcessObject::new(row);
                    by_pid.insert(p.pid, obj.clone());
                    added.push(obj);
                }
            }
        }
        let gone: Vec<u32> = by_pid
            .keys()
            .copied()
            .filter(|pid| !seen.contains(pid))
            .collect();
        if !gone.is_empty() {
            for pid in &gone {
                by_pid.remove(pid);
            }
            // Rebuilding the store is one items-changed for any number of exits.
            let keep: Vec<ProcessObject> = by_pid.values().cloned().collect();
            let selected = self.selected_pid();
            self.store.splice(0, self.store.n_items(), &keep);
            drop(by_pid);
            if let Some(pid) = selected {
                self.select_pid(pid);
            }
            by_pid = self.by_pid.borrow_mut();
        }
        if !added.is_empty() {
            self.store.extend_from_slice(&added);
        }
        drop(by_pid);
        if let Some(sorter) = self.view.sorter() {
            sorter.changed(gtk::SorterChange::Different);
        }
    }

    pub fn selected_pid(&self) -> Option<u32> {
        self.selection
            .selected_item()
            .and_downcast::<ProcessObject>()
            .map(|o| o.row().pid)
    }

    fn select_pid(&self, pid: u32) {
        for i in 0..self.selection.n_items() {
            if let Some(o) = self.selection.item(i).and_downcast::<ProcessObject>() {
                if o.row().pid == pid {
                    self.selection.set_selected(i);
                    return;
                }
            }
        }
    }

    pub fn connect_selection_changed(&self, f: impl Fn(Option<u32>) + 'static) {
        let selection = self.selection.clone();
        selection.connect_selection_changed(move |sel, _, _| {
            f(sel
                .selected_item()
                .and_downcast::<ProcessObject>()
                .map(|o| o.row().pid));
        });
    }

    /// Signals the selected process; the error text is for a toast.
    pub fn signal_selected(&self, signal: Signal) -> Result<(), String> {
        let pid = self.selected_pid().ok_or_else(String::new)?;
        procs::signal(pid, signal).map_err(|e| {
            let name = self
                .by_pid
                .borrow()
                .get(&pid)
                .map(|o| o.row().name)
                .unwrap_or_default();
            format!("{name} ({pid}): {e}")
        })
    }
}
