//! One row of the process table as a GObject, so a `ColumnView` can show, sort and filter it.

use std::cell::{Cell, RefCell};

use nodalia_gtk::gtk::glib;
use nodalia_gtk::gtk::glib::prelude::*;
use nodalia_gtk::gtk::glib::subclass::prelude::*;

/// What the table shows for a process at one refresh.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row {
    pub pid: u32,
    pub name: String,
    pub user: String,
    pub cpu: f64,
    pub rss: u64,
    pub threads: u32,
    pub state: char,
    pub cmdline: String,
}

mod imp {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::ProcessObject)]
    pub struct ProcessObject {
        /// Bumped on every update: expressions that depend on it re-evaluate.
        #[property(get, set)]
        pub revision: Cell<u32>,
        pub row: RefCell<Row>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProcessObject {
        const NAME: &'static str = "NodalixMonitorProcess";
        type Type = super::ProcessObject;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ProcessObject {}
}

glib::wrapper! {
    pub struct ProcessObject(ObjectSubclass<imp::ProcessObject>);
}

impl ProcessObject {
    pub fn new(row: Row) -> Self {
        let obj: Self = glib::Object::new();
        *obj.imp().row.borrow_mut() = row;
        obj
    }

    pub fn row(&self) -> Row {
        self.imp().row.borrow().clone()
    }

    pub fn update(&self, row: Row) {
        if *self.imp().row.borrow() != row {
            *self.imp().row.borrow_mut() = row;
            self.set_revision(self.revision().wrapping_add(1));
        }
    }
}
