use crate::{pages, system};
use gtk::prelude::*;

pub fn build_users_page() -> gtk::Widget {
    let current = system::users::current_user();
    let page = pages::page(
        "Usuarios",
        "Gestiona cuentas locales. Esta versión solo muestra información y acciones desactivadas.",
    );

    let profile = pages::card("Usuario actual");
    profile.append(&avatar_row(&current));
    profile.append(&pages::row("Nombre", &current.display_name));
    profile.append(&pages::row("Usuario", &current.username));
    profile.append(&pages::row("UID", &current.uid.to_string()));
    profile.append(&pages::button_bar(&[
        "Cambiar contraseña",
        "Gestionar grupos",
        "Añadir usuario",
    ]));
    page.append(&profile);

    let list = pages::card("Usuarios locales");
    let users = system::users::local_users();
    if users.is_empty() {
        list.append(&pages::row(
            "Estado",
            "No se pudieron leer usuarios locales",
        ));
    } else {
        for user in users {
            list.append(&pages::row(
                &user.display_name,
                &format!("{} · UID {}", user.username, user.uid),
            ));
        }
    }
    page.append(&list);
    pages::scrolled_page(page)
}

pub fn build_user_image_page() -> gtk::Widget {
    let current = system::users::current_user();
    let page = pages::page(
        "Imagen de usuario",
        "Previsualiza las rutas usadas para el avatar. Los cambios están desactivados por ahora.",
    );

    let card = pages::card("Avatar");
    card.append(&avatar_row(&current));
    for path in system::users::avatar_candidates(&current) {
        let state = if path.is_file() {
            "detectado"
        } else {
            "no existe"
        };
        card.append(&pages::row(&path.display().to_string(), state));
    }
    card.append(&pages::button_bar(&["Elegir imagen", "Restablecer imagen"]));
    page.append(&card);
    pages::scrolled_page(page)
}

fn avatar_row(user: &system::users::LocalUser) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    row.add_css_class("profile-row");

    let avatar: gtk::Widget = if let Some(path) = &user.avatar {
        let picture = gtk::Picture::for_filename(path);
        picture.set_size_request(72, 72);
        picture.add_css_class("profile-avatar");
        picture.upcast()
    } else {
        let label = gtk::Label::new(Some(&initials(&user.display_name)));
        label.set_size_request(72, 72);
        label.add_css_class("profile-avatar");
        label.add_css_class("profile-avatar-fallback");
        label.upcast()
    };

    let text = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let name = gtk::Label::new(Some(&user.display_name));
    name.set_xalign(0.0);
    name.add_css_class("profile-name");
    let username = gtk::Label::new(Some(&user.username));
    username.set_xalign(0.0);
    username.add_css_class("profile-username");
    text.append(&name);
    text.append(&username);
    row.append(&avatar);
    row.append(&text);
    row
}

fn initials(name: &str) -> String {
    let result = name
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>();
    if result.is_empty() {
        "?".to_string()
    } else {
        result.to_uppercase()
    }
}
