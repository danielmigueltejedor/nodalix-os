//! The small questions: a name, a yes or no.

use std::cell::RefCell;
use std::rc::Rc;

use archivador_core::names;
use ndl::gtk::{self, prelude::*};
use nodalia_gtk::{self as ndl, ResponseAppearance};

use crate::i18n::tr;

/// Asks for a name. `accept` runs with the name once it is valid and the user says so. With
/// `stem_only` the name's extension is left out of the selection, so typing replaces just the
/// part before it.
pub fn ask_name(
    parent: &impl IsA<gtk::Widget>,
    heading: &'static str,
    accept_label: &'static str,
    initial: &str,
    stem_only: bool,
    accept: impl Fn(String) + 'static,
) {
    let dialog = ndl::AlertDialog::new(Some(tr(heading)), None);
    dialog.add_response("cancel", tr("Cancel"));
    dialog.add_response("accept", tr(accept_label));
    dialog.set_response_appearance("accept", ResponseAppearance::Suggested);
    dialog.set_default_response(Some("accept"));
    dialog.set_close_response("cancel");

    let entry = gtk::Entry::new();
    entry.set_text(initial);
    entry.set_activates_default(false);
    dialog.set_extra_child(Some(&entry));
    dialog.set_response_enabled("accept", names::validate(initial).is_ok());

    // The entry's handlers reach the dialog through a slot that the response empties, so the
    // two do not keep each other alive.
    let slot: Rc<RefCell<Option<ndl::AlertDialog>>> = Rc::new(RefCell::new(Some(dialog.clone())));
    {
        let slot = slot.clone();
        entry.connect_changed(move |entry| {
            if let Some(dialog) = slot.borrow().as_ref() {
                let text = entry.text();
                dialog.set_response_enabled("accept", names::validate(&text).is_ok());
            }
        });
    }
    {
        let slot = slot.clone();
        entry.connect_activate(move |_| {
            let dialog = slot.borrow().clone();
            if let Some(dialog) = dialog {
                dialog.activate_default();
            }
        });
    }
    {
        let entry = entry.clone();
        let slot = slot.clone();
        dialog.connect_response(None, move |_, id| {
            slot.borrow_mut().take();
            if id == "accept" {
                let text = entry.text();
                if let Ok(name) = names::validate(&text) {
                    accept(name.to_owned());
                }
            }
        });
    }
    dialog.present(Some(parent));
    entry.grab_focus();
    let end = if stem_only {
        match initial.rfind('.') {
            Some(i) if i > 0 => initial[..i].chars().count(),
            _ => initial.chars().count(),
        }
    } else {
        initial.chars().count()
    };
    entry.select_region(0, end as i32);
}

/// A yes or no question; `accept` runs on yes. A destructive answer is drawn as one.
pub fn confirm(
    parent: &impl IsA<gtk::Widget>,
    heading: &str,
    body: &str,
    accept_label: &'static str,
    destructive: bool,
    accept: impl Fn() + 'static,
) {
    let dialog = ndl::AlertDialog::new(Some(heading), Some(body));
    dialog.add_response("cancel", tr("Cancel"));
    dialog.add_response("accept", tr(accept_label));
    dialog.set_response_appearance(
        "accept",
        if destructive {
            ResponseAppearance::Destructive
        } else {
            ResponseAppearance::Suggested
        },
    );
    dialog.set_close_response("cancel");
    dialog.set_default_response(Some("cancel"));
    // Nothing else owns the alert: its buttons reach it through weak references, so it would be
    // dropped as this returns and every one of them would do nothing. A slot keeps it until
    // it has been answered.
    let keep = Rc::new(RefCell::new(Some(dialog.clone())));
    dialog.connect_response(None, move |_, id| {
        keep.borrow_mut().take();
        if id == "accept" {
            accept();
        }
    });
    dialog.present(Some(parent));
}
