use gtk::{glib, prelude::*};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

pub fn switcher() -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    let active = active_workspace_id().unwrap_or(1);
    let ids = Rc::new(RefCell::new(workspace_ids(active)));
    let buttons = Rc::new(RefCell::new(Vec::new()));
    render(&row, &ids.borrow(), active, &buttons);

    let refresh_row = row.clone();
    glib::timeout_add_local(Duration::from_millis(250), move || {
        let active = active_workspace_id().unwrap_or(1);
        let next_ids = workspace_ids(active);
        if *ids.borrow() != next_ids {
            *ids.borrow_mut() = next_ids;
            render(&refresh_row, &ids.borrow(), active, &buttons);
        } else {
            mark_active(&buttons.borrow(), Some(active));
        }
        glib::ControlFlow::Continue
    });

    row
}

fn workspace_ids(active: i32) -> Vec<i32> {
    let max = active.max(5);
    (1..=max).collect()
}

fn render(
    row: &gtk::Box,
    ids: &[i32],
    active: i32,
    buttons: &Rc<RefCell<Vec<(i32, gtk::Button)>>>,
) {
    while let Some(child) = row.first_child() {
        row.remove(&child);
    }

    let mut next_buttons = Vec::new();
    for id in ids {
        let button = gtk::Button::with_label(&id.to_string());
        button.add_css_class("workspace-button");
        if *id == active {
            button.add_css_class("active-workspace");
        }
        button.set_tooltip_text(Some(&format!("Workspace {id}")));
        button.connect_clicked({
            let id = *id;
            move |_| {
                super::spawn_detached("hyprctl", &["dispatch", "workspace", &id.to_string()]);
            }
        });
        row.append(&button);
        next_buttons.push((*id, button));
    }

    *buttons.borrow_mut() = next_buttons;
}

fn active_workspace_id() -> Option<i32> {
    if !super::command_exists("hyprctl") {
        return None;
    }
    super::capture("hyprctl", &["activeworkspace", "-j"])
        .and_then(|text| extract_json_i32(&text, "id"))
}

fn mark_active(buttons: &[(i32, gtk::Button)], active: Option<i32>) {
    for (id, button) in buttons {
        if Some(*id) == active {
            button.add_css_class("active-workspace");
        } else {
            button.remove_css_class("active-workspace");
        }
    }
}

fn extract_json_i32(text: &str, key: &str) -> Option<i32> {
    let needle = format!("\"{}\":", key);
    let start = text.find(&needle)? + needle.len();
    let rest = text[start..].trim_start();
    let end = rest
        .find(|ch: char| !ch.is_ascii_digit() && ch != '-')
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}
