use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct SegmentedOption {
    pub id: &'static str,
    pub icon: &'static str,
    pub label: &'static str,
}

pub struct SegmentedControl {
    pub root: gtk::Box,
    buttons: Vec<gtk::ToggleButton>,
}

impl SegmentedControl {
    pub fn new(options: &[SegmentedOption]) -> Self {
        let root = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        root.add_css_class("segmented-control");

        let group = gtk::ToggleButton::new();
        let mut buttons = Vec::new();

        for (index, opt) in options.iter().enumerate() {
            let btn = gtk::ToggleButton::new();
            btn.set_has_frame(false);
            btn.add_css_class("segmented-btn");
            btn.set_group(Some(&group));
            btn.set_tooltip_text(Some(opt.label));
            btn.set_widget_name(opt.id);

            let content = gtk::Box::new(gtk::Orientation::Vertical, 4);
            content.set_halign(gtk::Align::Center);
            let icon = gtk::Label::new(Some(opt.icon));
            icon.add_css_class("segmented-icon");
            let label = gtk::Label::new(Some(opt.label));
            label.add_css_class("segmented-label");
            content.append(&icon);
            content.append(&label);
            btn.set_child(Some(&content));

            if index == 0 {
                btn.set_active(true);
            }
            root.append(&btn);
            buttons.push(btn);
        }

        Self { root, buttons }
    }

    pub fn set_active_id(&self, id: &str) {
        for btn in &self.buttons {
            btn.set_active(btn.widget_name().as_str() == id);
        }
    }

    pub fn connect_changed<F>(&self, f: F)
    where
        F: FnMut(&str) + 'static,
    {
        let f = Rc::new(RefCell::new(f));
        for btn in &self.buttons {
            let f = f.clone();
            btn.connect_toggled(move |b| {
                if b.is_active() {
                    f.borrow_mut()(b.widget_name().as_str());
                }
            });
        }
    }
}
