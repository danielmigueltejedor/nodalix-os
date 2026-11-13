//! The process table.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::rc::Rc;

use monitor_core::format_bytes;
use monitor_core::procs::{self, Signal};
use nodalia_gtk as ndl;
use nodalia_gtk::gtk::{self, gio, glib, prelude::*};

use crate::i18n::tr;
use crate::process_object::{ProcessObject, Row};
use crate::sampler::Snapshot;

pub struct ProcessView {
    pub widget: gtk::Box,
    store: gio::ListStore,
    by_pid: RefCell<HashMap<u32, ProcessObject>>,
    query: Rc<RefCell<String>>,
    filter: gtk::CustomFilter,
    users: HashMap<u32, String>,
    view: gtk::ColumnView,
    selection: gtk::SingleSelection,
    menu: gtk::PopoverMenu,
    pub search: gtk::SearchEntry,
}

/// What the menu copies.
#[derive(Clone, Copy)]
pub enum Copy {
    Pid,
    Command,
}

type Cmp = fn(&Row, &Row) -> Ordering;

/// The column's order, with the pid as the last word: rows that tie (most of them on CPU) keep
/// the same order whatever order they were added or rebuilt in.
fn compare_rows(cmp: Cmp, a: &Row, b: &Row) -> Ordering {
    cmp(a, b).then_with(|| a.pid.cmp(&b.pid))
}

/// Called with the list item and the widget that was clicked, and the click's place in it.
type SecondaryClick = Rc<dyn Fn(&gtk::ListItem, &gtk::Widget, f64, f64)>;

fn column(
    title: &str,
    expand: bool,
    numeric: bool,
    with_icon: bool,
    cmp: Cmp,
    text: fn(&Row) -> String,
    on_secondary: &SecondaryClick,
) -> gtk::ColumnViewColumn {
    let on_secondary = on_secondary.clone();
    let factory = gtk::SignalListItemFactory::new();
    // The label follows the list item's `item` property and the row's `revision`, so one binding
    // made here serves every row the item is reused for; binding again on each rebind would
    // pile up watches on the same label.
    factory.connect_setup(move |_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().expect("a list item");
        let label = gtk::Label::new(None);
        label.set_xalign(if numeric { 1.0 } else { 0.0 });
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        if numeric {
            label.add_css_class("numeric");
        }
        let image = gtk::Image::new();
        if with_icon {
            image.set_pixel_size(20);
            let cell = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            cell.append(&image);
            label.set_hexpand(true);
            cell.append(&label);
            item.set_child(Some(&cell));
        } else {
            item.set_child(Some(&label));
        }
        // A right click anywhere on the row's cell, padding included, asks for the menu. The
        // handler reads the item's position when it is clicked, since the item is reused.
        let click = gtk::GestureClick::builder()
            .button(gtk::gdk::BUTTON_SECONDARY)
            .build();
        {
            let (item, on_secondary) = (item.downgrade(), on_secondary.clone());
            click.connect_pressed(move |gesture, _, x, y| {
                if let (Some(item), Some(widget)) = (item.upgrade(), gesture.widget()) {
                    on_secondary(&item, &widget, x, y);
                    gesture.set_state(gtk::EventSequenceState::Claimed);
                }
            });
        }
        // The whole cell takes the click: the widget that holds the item's own child.
        let cell = item.child();
        cell.as_ref()
            .and_then(|c| c.parent())
            .or(cell)
            .unwrap_or_else(|| label.clone().upcast())
            .add_controller(click);

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
        if with_icon {
            let params: [gtk::Expression; 1] = [item_expr.upcast()];
            let icon = gtk::ClosureExpression::new::<Option<gio::Icon>>(
                params,
                glib::closure!(
                    move |_this: Option<glib::Object>, obj: Option<glib::Object>| {
                        obj.and_downcast::<ProcessObject>()
                            .map(|o| crate::icons::for_row(&o.row()))
                    }
                ),
            );
            icon.bind(&image, "gicon", Some(item));
        }
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
        compare_rows(cmp, &a.row(), &b.row()).into()
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

        // The menu of a right click. The window gives it its entries (it owns the actions).
        let menu = gtk::PopoverMenu::from_model(None::<&gio::MenuModel>);
        menu.set_parent(&view);
        menu.set_has_arrow(false);
        menu.set_halign(gtk::Align::Start);
        let on_secondary: SecondaryClick = {
            let (selection, menu, view) = (selection.clone(), menu.clone(), view.clone());
            Rc::new(move |item, cell, x, y| {
                // The row under the pointer is the one the menu is about.
                selection.set_selected(item.position());
                let Some(point) =
                    cell.compute_point(&view, &gtk::graphene::Point::new(x as f32, y as f32))
                else {
                    return;
                };
                menu.set_pointing_to(Some(&gtk::gdk::Rectangle::new(
                    point.x() as i32,
                    point.y() as i32,
                    1,
                    1,
                )));
                menu.popup();
            })
        };

        let cols: [gtk::ColumnViewColumn; 7] = [
            column(
                tr("Name"),
                true,
                false,
                true,
                |a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                |r| r.name.clone(),
                &on_secondary,
            ),
            column(
                tr("User"),
                false,
                false,
                false,
                |a, b| a.user.cmp(&b.user),
                |r| r.user.clone(),
                &on_secondary,
            ),
            column(
                tr("% CPU"),
                false,
                true,
                false,
                |a, b| a.cpu.total_cmp(&b.cpu),
                |r| format!("{:.1}", r.cpu),
                &on_secondary,
            ),
            column(
                tr("Memory"),
                false,
                true,
                false,
                |a, b| a.rss.cmp(&b.rss),
                |r| format_bytes(r.rss),
                &on_secondary,
            ),
            column(
                tr("ID"),
                false,
                true,
                false,
                |a, b| a.pid.cmp(&b.pid),
                |r| r.pid.to_string(),
                &on_secondary,
            ),
            column(
                tr("Threads"),
                false,
                true,
                false,
                |a, b| a.threads.cmp(&b.threads),
                |r| r.threads.to_string(),
                &on_secondary,
            ),
            column(
                tr("State"),
                false,
                false,
                false,
                |a, b| a.state.cmp(&b.state),
                |r| tr(procs::state_label(r.state)).to_string(),
                &on_secondary,
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
            query,
            filter,
            users: procs::read_users(),
            view,
            selection,
            menu,
            search,
        })
    }

    /// Brings the table in line with a reading: update in place, add the new, drop the gone.
    /// A process is the pid together with its start time, so a pid handed to another process
    /// is a new row, not the old one under a new name.
    pub fn update(&self, snapshot: &Snapshot) {
        let mut by_pid = self.by_pid.borrow_mut();
        let seen: std::collections::HashSet<(u32, u64)> = snapshot
            .processes
            .iter()
            .map(|(p, _)| (p.pid, p.start_time))
            .collect();
        let gone: Vec<u32> = by_pid
            .iter()
            .filter(|(pid, obj)| !seen.contains(&(**pid, obj.row().start_time)))
            .map(|(pid, _)| *pid)
            .collect();
        let selected = self.selected_process();
        for pid in &gone {
            by_pid.remove(pid);
        }

        let mut added = Vec::new();
        for (p, cpu) in &snapshot.processes {
            let row = Row {
                pid: p.pid,
                start_time: p.start_time,
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

        if gone.is_empty() {
            self.store.extend_from_slice(&added);
        } else {
            // Rebuilding the store is one items-changed for any number of exits. It already
            // holds the new rows, since they are in `by_pid` by now.
            // In pid order, not hash order, so rows that tie on the sort column stay where they are.
            let mut all: Vec<ProcessObject> = by_pid.values().cloned().collect();
            all.sort_by_key(|o| o.row().pid);
            self.store.splice(0, self.store.n_items(), &all);
        }
        drop(by_pid);
        if !gone.is_empty() {
            if let Some((pid, start)) = selected {
                self.select_process(pid, start);
            }
        }
        // Names and command lines change under a running pid; a filter does not watch rows.
        if !self.query.borrow().is_empty() {
            self.filter.changed(gtk::FilterChange::Different);
        }
        if let Some(sorter) = self.view.sorter() {
            sorter.changed(gtk::SorterChange::Different);
        }
    }

    /// The selected process as (pid, start time).
    fn selected_process(&self) -> Option<(u32, u64)> {
        self.selection
            .selected_item()
            .and_downcast::<ProcessObject>()
            .map(|o| {
                let r = o.row();
                (r.pid, r.start_time)
            })
    }

    fn select_process(&self, pid: u32, start_time: u64) {
        for i in 0..self.selection.n_items() {
            if let Some(o) = self.selection.item(i).and_downcast::<ProcessObject>() {
                let r = o.row();
                if r.pid == pid && r.start_time == start_time {
                    self.selection.set_selected(i);
                    return;
                }
            }
        }
    }

    /// The entries of the right-click menu.
    pub fn set_context_menu(&self, model: &gio::MenuModel) {
        self.menu.set_menu_model(Some(model));
        // The rows are made from the model, so they are styled after it is set.
        ndl::menu_rows(&self.menu);
    }

    /// Puts the selected process's pid, or its command line, on the clipboard.
    pub fn copy_selected(&self, what: Copy) {
        let Some(row) = self
            .selection
            .selected_item()
            .and_downcast::<ProcessObject>()
            .map(|o| o.row())
        else {
            return;
        };
        let text = match what {
            Copy::Pid => row.pid.to_string(),
            // A kernel thread has no command line; its name is what there is to copy.
            Copy::Command if row.cmdline.is_empty() => row.name,
            Copy::Command => row.cmdline,
        };
        self.view.clipboard().set_text(&text);
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
        let (pid, start_time) = self.selected_process().ok_or_else(String::new)?;
        procs::signal(pid, start_time, signal).map_err(|e| {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_that_tie_are_ordered_by_pid() {
        let row = |pid| Row {
            pid,
            cpu: 0.0,
            ..Row::default()
        };
        let by_cpu: Cmp = |a, b| a.cpu.total_cmp(&b.cpu);
        // The same two rows, added in either order, compare the same way.
        assert_eq!(compare_rows(by_cpu, &row(100), &row(200)), Ordering::Less);
        assert_eq!(
            compare_rows(by_cpu, &row(200), &row(100)),
            Ordering::Greater
        );
        assert_eq!(compare_rows(by_cpu, &row(7), &row(7)), Ordering::Equal);
        // A real difference in the column still wins over the pid.
        let busy = Row {
            pid: 1,
            cpu: 5.0,
            ..Row::default()
        };
        assert_eq!(compare_rows(by_cpu, &busy, &row(2)), Ordering::Greater);
    }
}
