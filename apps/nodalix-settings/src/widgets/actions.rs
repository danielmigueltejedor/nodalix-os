use gtk::glib;
use gtk::prelude::*;
use std::sync::mpsc;

pub struct ActionButton {
    pub button: gtk::Button,
}

pub fn icon_action_button(icon: &str, tooltip: &str, destructive: bool) -> ActionButton {
    let button = gtk::Button::new();
    button.add_css_class("quick-action-btn");
    if destructive {
        button.add_css_class("quick-action-destructive");
    }
    button.set_tooltip_text(Some(tooltip));
    button.set_has_frame(false);

    let label = gtk::Label::new(Some(icon));
    label.add_css_class("quick-action-icon");
    button.set_child(Some(&label));

    ActionButton { button }
}

/// Runs blocking work on a worker thread and delivers the result on the GTK main loop.
pub fn run_bg<F, T, C>(task: F, on_done: C)
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
    C: FnOnce(Result<T, String>) + 'static,
{
    let (tx, rx) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let _ = tx.send(task());
    });

    let rx = std::cell::RefCell::new(rx);
    let callback = std::cell::RefCell::new(Some(on_done));
    glib::idle_add_local(move || {
        match rx.borrow().try_recv() {
            Ok(result) => {
                if let Some(cb) = callback.borrow_mut().take() {
                    cb(result);
                }
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                if let Some(cb) = callback.borrow_mut().take() {
                    cb(Err("La tarea no devolvió resultado".to_string()));
                }
                glib::ControlFlow::Break
            }
        }
    });
}

#[allow(deprecated)]
pub fn confirm_destructive(
    parent: Option<&impl IsA<gtk::Window>>,
    heading: &str,
    body: &str,
    confirm_label: &str,
    on_result: impl FnOnce(bool) + 'static,
) {
    let dialog = gtk::Dialog::builder()
        .title(heading)
        .modal(true)
        .build();

    if let Some(window) = parent {
        dialog.set_transient_for(Some(window));
    }

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(18);
    content.set_margin_bottom(12);
    content.set_margin_start(20);
    content.set_margin_end(20);
    let body_label = gtk::Label::new(Some(body));
    body_label.set_wrap(true);
    body_label.set_xalign(0.0);
    body_label.add_css_class("dialog-label");
    content.append(&body_label);
    dialog.set_child(Some(&content));

    dialog.add_button("Cancelar", gtk::ResponseType::Cancel);
    dialog.add_button(confirm_label, gtk::ResponseType::Accept);
    dialog.set_default_response(gtk::ResponseType::Cancel);

    let callback = std::cell::RefCell::new(Some(on_result));
    dialog.connect_response(move |dialog, response| {
        if let Some(cb) = callback.borrow_mut().take() {
            cb(response == gtk::ResponseType::Accept);
        }
        dialog.close();
    });
    dialog.present();
}

pub fn window_ancestor(widget: &impl IsA<gtk::Widget>) -> Option<gtk::Window> {
    widget.root().and_downcast::<gtk::Window>()
}
