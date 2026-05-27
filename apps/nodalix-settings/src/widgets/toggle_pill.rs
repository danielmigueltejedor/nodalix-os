use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// Pill-style on/off control (replaces raw GtkSwitch on home cards).
pub struct TogglePill {
    pub root: gtk::Box,
    btn: gtk::Button,
    label_on: String,
    label_off: String,
    pub syncing: Rc<std::cell::Cell<bool>>,
}

impl TogglePill {
    pub fn new(label_on: &str, label_off: &str) -> Self {
        let root = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        root.add_css_class("toggle-pill");
        root.add_css_class("toggle-pill-off");

        let btn = gtk::Button::with_label(label_off);
        btn.add_css_class("toggle-pill-knob");
        btn.set_has_frame(false);
        root.append(&btn);

        Self {
            root,
            btn,
            label_on: label_on.to_string(),
            label_off: label_off.to_string(),
            syncing: Rc::new(std::cell::Cell::new(false)),
        }
    }

    pub fn set_active(&self, on: bool) {
        self.syncing.set(true);
        if on {
            self.root.add_css_class("toggle-pill-on");
            self.root.remove_css_class("toggle-pill-off");
            self.btn.set_label(&self.label_on);
        } else {
            self.root.add_css_class("toggle-pill-off");
            self.root.remove_css_class("toggle-pill-on");
            self.btn.set_label(&self.label_off);
        }
        self.syncing.set(false);
    }

    pub fn connect_toggled<F>(&self, f: F) -> gtk::glib::SignalHandlerId
    where
        F: FnMut(bool) + 'static,
    {
        let root = self.root.clone();
        let btn = self.btn.clone();
        let label_on = self.label_on.clone();
        let label_off = self.label_off.clone();
        let syncing = self.syncing.clone();
        let f = Rc::new(RefCell::new(f));
        btn.clone().connect_clicked(move |_| {
            if syncing.get() {
                return;
            }
            let next = !root.has_css_class("toggle-pill-on");
            if next {
                root.add_css_class("toggle-pill-on");
                root.remove_css_class("toggle-pill-off");
                btn.set_label(&label_on);
            } else {
                root.add_css_class("toggle-pill-off");
                root.remove_css_class("toggle-pill-on");
                btn.set_label(&label_off);
            }
            f.borrow_mut()(next);
        })
    }
}
