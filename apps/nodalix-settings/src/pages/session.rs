use crate::{
    pages,
    system::power,
    widgets::{
        confirm_destructive, icon_action_button, run_bg, window_ancestor, ActionButton, StatusStrip,
    },
};
use gtk::prelude::*;
use nodalix_system_actions::SystemAction;

pub fn build_session_page() -> gtk::Widget {
    let page = pages::page(
        "Sesión",
        "Acciones de energía y sesión. Las acciones destructivas piden confirmación.",
    );
    let status = StatusStrip::new();
    page.append(&status.root);

    let card = pages::card("Acciones rápidas");
    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    bar.add_css_class("quick-actions-bar");

    let specs: [(&str, &str, bool, SystemAction, fn() -> Result<(), String>); 5] = [
        (
            "󰌾",
            "Bloquear sesión",
            false,
            SystemAction::Lock,
            power::lock_session,
        ),
        (
            "󰍃",
            "Cerrar sesión",
            true,
            SystemAction::Logout,
            power::logout_session,
        ),
        (
            "󰒲",
            "Suspender",
            false,
            SystemAction::Suspend,
            power::suspend,
        ),
        ("󰑐", "Reiniciar", true, SystemAction::Reboot, power::reboot),
        ("󰐥", "Apagar", true, SystemAction::Shutdown, power::poweroff),
    ];

    for (icon, tooltip, destructive, system_action, action) in specs {
        let (available, reason) = power::action_available(system_action);
        let ActionButton { button } = icon_action_button(icon, tooltip, destructive);
        button.set_sensitive(available);
        if let Some(reason) = reason {
            button.set_tooltip_text(Some(&format!("{tooltip} — {reason}")));
        }

        let status = status.clone();
        let tooltip_owned = tooltip.to_string();
        button.connect_clicked(move |btn| {
            if !available {
                return;
            }
            let parent = window_ancestor(btn);
            let run_action = {
                let status = status.clone();
                let tooltip_owned = tooltip_owned.clone();
                move || {
                    status.set_loading(&format!("{tooltip_owned}…"));
                    let status_done = status.clone();
                    run_bg(action, move |result| match result {
                        Ok(()) => status_done.set_success(&format!("{tooltip_owned} enviado")),
                        Err(err) => status_done.set_error(&err),
                    });
                }
            };
            if destructive {
                let status_cancel = status.clone();
                let run_action = run_action;
                confirm_destructive(
                    parent.as_ref(),
                    &format!("¿{tooltip_owned}?"),
                    &format!("Confirma que quieres {}.", tooltip_owned.to_lowercase()),
                    tooltip,
                    move |ok| {
                        if ok {
                            run_action();
                        } else {
                            status_cancel.set_error("Acción cancelada");
                        }
                    },
                );
            } else {
                run_action();
            }
        });
        bar.append(&button);
    }

    card.append(&bar);
    page.append(&card);
    pages::scrolled_page(page)
}
