//! One file or folder as a GObject, so `ListView` and `GridView` can show it.

use std::cell::RefCell;

use archivador_core::entry::Entry;
use nodalia_gtk::gtk::glib;
use nodalia_gtk::gtk::glib::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct FileObject {
        pub entry: RefCell<Entry>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FileObject {
        const NAME: &'static str = "NodalixArchivadorFile";
        type Type = super::FileObject;
    }

    impl ObjectImpl for FileObject {}
}

glib::wrapper! {
    pub struct FileObject(ObjectSubclass<imp::FileObject>);
}

impl FileObject {
    pub fn new(entry: Entry) -> Self {
        let object: Self = glib::Object::new();
        *object.imp().entry.borrow_mut() = entry;
        object
    }

    pub fn entry(&self) -> Entry {
        self.imp().entry.borrow().clone()
    }
}
