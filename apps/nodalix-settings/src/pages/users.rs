use crate::{
    pages,
    system::{profile, users},
    widgets::{avatar_widget, initials_from_name, run_bg, StatusStrip},
};
use gtk::prelude::*;

pub fn build_users_page() -> gtk::Widget {
    let current = users::current_user();
    let page = pages::page(
        "Perfil y usuarios",
        "Imagen de cuenta, almacenamiento Nodalix y usuarios locales del sistema.",
    );
    let status = StatusStrip::new();
    page.append(&status.root);

    let profile_card = pages::card("Tu perfil");
    profile_card.append(&avatar_section(&current, &status));
    profile_card.append(&pages::row("Nombre", &current.display_name));
    profile_card.append(&pages::row("Usuario", &current.username));
    profile_card.append(&pages::row("UID", &current.uid.to_string()));
    page.append(&profile_card);

    let paths = pages::card("Almacenamiento del perfil Nodalix");
    for (label, path) in profile::profile_store_paths() {
        let state = if std::path::Path::new(&path).exists() {
            "presente"
        } else {
            "pendiente"
        };
        paths.append(&pages::row(&label, &format!("{path} · {state}")));
    }
    page.append(&paths);

    let list = pages::card("Usuarios locales");
    let all = users::local_users();
    if all.is_empty() {
        list.append(&pages::row(
            "Estado",
            "No se pudieron leer usuarios locales",
        ));
    } else {
        for user in all {
            list.append(&pages::row(
                &user.display_name,
                &format!("{} · UID {}", user.username, user.uid),
            ));
        }
    }
    page.append(&list);
    pages::scrolled_page(page)
}

fn avatar_section(user: &users::LocalUser, status: &StatusStrip) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    row.add_css_class("profile-row");

    let avatar_host = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    avatar_host.append(&avatar_widget(
        96,
        user.avatar.as_deref(),
        &initials_from_name(&user.display_name),
    ));

    let side = gtk::Box::new(gtk::Orientation::Vertical, 10);
    side.set_valign(gtk::Align::Center);

    let name = gtk::Label::new(Some(&user.display_name));
    name.set_xalign(0.0);
    name.add_css_class("profile-name");
    let username = gtk::Label::new(Some(&format!("@{}", user.username)));
    username.set_xalign(0.0);
    username.add_css_class("profile-username");

    let hint = gtk::Label::new(Some(
        "La imagen se guarda en ~/.config/nodalix/user/avatar.png y se sincroniza con el greeter.",
    ));
    hint.set_xalign(0.0);
    hint.set_wrap(true);
    hint.add_css_class("home-card-detail");

    let pick = gtk::Button::with_label("Cambiar imagen…");
    pick.add_css_class("pill-button");
    pick.add_css_class("pill-button-accent");
    let status = status.clone();
    let avatar_host_cb = avatar_host.clone();
    pick.connect_clicked(move |_| {
        let dialog = gtk::FileDialog::new();
        dialog.set_title("Elegir imagen de usuario");
        let filter = gtk::FileFilter::new();
        filter.add_mime_type("image/png");
        filter.add_mime_type("image/jpeg");
        filter.add_mime_type("image/webp");
        filter.set_name(Some("Imágenes"));
        let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        dialog.set_filters(Some(&filters));
        let status = status.clone();
        let avatar_host = avatar_host_cb.clone();
        dialog.open(
            None::<&gtk::Window>,
            None::<&gtk::gio::Cancellable>,
            move |result| {
                let Ok(file) = result else { return };
                let Some(path) = file.path() else { return };
                status.set_loading("Guardando imagen…");
                let path_buf = path.clone();
                run_bg(
                    move || users::set_avatar_from_file(&path_buf),
                    move |result| match result {
                        Ok(dest) => {
                            status.set_success(&format!("Avatar guardado en {}", dest.display()));
                            while let Some(child) = avatar_host.first_child() {
                                avatar_host.remove(&child);
                            }
                            let fresh = users::current_user();
                            avatar_host.append(&avatar_widget(
                                96,
                                fresh.avatar.as_deref(),
                                &initials_from_name(&fresh.display_name),
                            ));
                        }
                        Err(err) => status.set_error(&err),
                    },
                );
            },
        );
    });

    side.append(&name);
    side.append(&username);
    side.append(&hint);
    side.append(&pick);
    row.append(&avatar_host);
    row.append(&side);
    row
}
