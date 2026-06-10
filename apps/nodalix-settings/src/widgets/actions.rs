use adw::prelude::*;
use gtk::glib;
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
    glib::idle_add_local(move || match rx.borrow().try_recv() {
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
    });
}

pub fn confirm_destructive(
    parent: Option<&impl IsA<gtk::Window>>,
    heading: &str,
    body: &str,
    confirm_label: &str,
    on_result: impl FnOnce(bool) + 'static,
) {
    let dialog = adw::MessageDialog::new(parent, Some(heading), Some(body));
    dialog.set_modal(true);

    dialog.add_response("cancel", "Cancelar");
    dialog.add_response("confirm", confirm_label);
    dialog.set_response_appearance("confirm", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");

    let callback = std::cell::RefCell::new(Some(on_result));
    dialog.connect_response(None, move |dialog, response| {
        if let Some(cb) = callback.borrow_mut().take() {
            cb(response == "confirm");
        }
        dialog.close();
    });

    dialog.present();
}

pub fn window_ancestor(widget: &impl IsA<gtk::Widget>) -> Option<gtk::Window> {
    if let Some(window) = widget.root().and_downcast::<gtk::Window>() {
        return Some(window);
    }

    let mut current = widget.parent();
    while let Some(node) = current {
        if let Some(window) = node.downcast_ref::<gtk::Window>() {
            return Some(window.clone());
        }
        current = node.parent();
    }

    None
}
