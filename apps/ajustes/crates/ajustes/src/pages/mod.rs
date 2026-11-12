pub mod about;
pub mod accessibility;
pub mod appearance;
pub mod bus;
pub mod datetime;
pub mod gs;
pub mod keyboard;
pub mod localsend;
pub mod multitasking;
pub mod network;
pub mod nm;
pub mod notifications;
pub mod pointing;
pub mod power;
pub mod privacy;
pub mod sharing;
pub mod sound;
pub mod updates;
pub mod wifi;

use nodalia_gtk::{self as ndl, gtk, gtk::prelude::*};

/// A scrolled page of preference groups, centred and no wider than reads well.
pub fn scroller(content: &impl gtk::prelude::IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    let clamp = ndl::Clamp::builder()
        .maximum_size(720)
        .child(content)
        .build();
    gtk::ScrolledWindow::builder()
        .child(&clamp)
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build()
}

/// A vertical box with the page's margins.
pub fn column() -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 18);
    b.set_margin_top(12);
    b.set_margin_bottom(24);
    b.set_margin_start(16);
    b.set_margin_end(16);
    b
}
