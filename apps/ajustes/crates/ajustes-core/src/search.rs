//! What the Shell's search finds in Settings: the pages, by name and by what is in them, in
//! English and Spanish. No D-Bus and no GTK here, so it is unit tested; the provider that
//! answers the Shell is in the app.

use crate::pages::{self, Page};

/// Every result id starts with this, then the page id: `nodalix-storage`.
pub const RESULT_PREFIX: &str = "nodalix-";

/// Per page: its name in both languages, then the words to find it by (both languages, no
/// accents needed). A search term matches the start of a word.
const WORDS: &[(&str, &str, &str)] = &[
    (
        "appearance",
        "Appearance Apariencia",
        "style theme dark light mode colour color accent background wallpaper wallpapers animated desktop lock screen estilo tema oscuro claro modo acento fondo fondos animados escritorio pantalla bloqueo",
    ),
    (
        "updates",
        "Updates Actualizaciones",
        "update upgrade automatic software system actualizar automaticas sistema novedades",
    ),
    (
        "network",
        "Network Red",
        "internet ethernet wired vpn proxy dns ip address conexion cable direccion",
    ),
    (
        "wifi",
        "Wi-Fi",
        "wifi wireless network hotspot wlan inalambrica red",
    ),
    (
        "bluetooth",
        "Bluetooth",
        "devices pairing headphones dispositivos emparejar auriculares",
    ),
    (
        "sharing",
        "Sharing Compartir",
        "remote desktop file media device name escritorio remoto archivos nombre dispositivo",
    ),
    (
        "localsend",
        "LocalSend",
        "send files nearby share enviar archivos cercanos compartir",
    ),
    (
        "online-accounts",
        "Online accounts, Cuentas en linea",
        "google microsoft nextcloud cloud sign in cuentas nube",
    ),
    (
        "cloud",
        "Cloud storage Almacenamiento en la nube",
        "drive onedrive dropbox icloud rclone nube",
    ),
    (
        "display",
        "Displays Pantallas",
        "screen monitor resolution refresh rate night light scale orientation brightness pantalla resolucion frecuencia luz nocturna escala brillo",
    ),
    (
        "sound",
        "Sound Sonido",
        "audio volume output input speakers microphone alerts volumen salida entrada altavoces microfono alertas",
    ),
    (
        "power",
        "Power Energia",
        "battery saving suspend blank profile performance bateria ahorro suspension rendimiento perfil",
    ),
    (
        "keyboard",
        "Keyboard Teclado",
        "layout shortcuts compose key input distribucion atajos tecla",
    ),
    (
        "mouse",
        "Mouse and touchpad Raton y panel tactil",
        "pointer speed tap click scroll natural puntero velocidad",
    ),
    (
        "printers",
        "Printers Impresoras",
        "printing printer impresion impresora",
    ),
    (
        "color",
        "Color",
        "profiles calibration icc perfiles calibracion",
    ),
    (
        "notifications",
        "Notifications Notificaciones",
        "do not disturb lock screen no molestar",
    ),
    (
        "multitasking",
        "Multitasking Multitarea",
        "hot corner workspaces windows esquina activa areas trabajo ventanas",
    ),
    (
        "storage",
        "Storage Almacenamiento",
        "disk space free large files documents big trash cache purge clean disco espacio libre archivos grandes documentos papelera limpiar",
    ),
    (
        "applications",
        "Apps Applications Aplicaciones",
        "default installed search web browser mail predeterminadas instaladas navegador correo busqueda",
    ),
    (
        "privacy",
        "Privacy and security Privacidad y seguridad",
        "lock screen location camera microphone usb thunderbolt diagnostics history bloqueo ubicacion camara microfono diagnostico historial",
    ),
    (
        "accessibility",
        "Accessibility Accesibilidad",
        "universal access text size contrast zoom screen reader tamano texto lector pantalla",
    ),
    (
        "datetime",
        "Date and time Fecha y hora",
        "time zone clock automatic 24 hour zona horaria reloj formato",
    ),
    (
        "region",
        "Region and language Region e idioma",
        "formats locale lengua formatos",
    ),
    (
        "users",
        "Users Usuarios",
        "accounts password name picture photo administrator cuentas contrasena nombre foto administrador",
    ),
    (
        "about",
        "About Acerca de",
        "system version nodalix os device name memory processor kernel disk diagnostics info sistema nombre dispositivo memoria procesador nucleo informacion diagnostico",
    ),
];

/// Lower case, accents removed, anything that is not a letter or a digit a space.
pub fn fold(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            c if c.is_alphanumeric() => c,
            _ => ' ',
        })
        .collect()
}

/// Whether every term starts a word of `text`.
fn every_term_starts_a_word(terms: &[String], text: &str) -> bool {
    let words: Vec<String> = fold(text).split_whitespace().map(str::to_owned).collect();
    terms
        .iter()
        .all(|term| words.iter().any(|word| word.starts_with(term.as_str())))
}

/// The terms of a search, folded. A term can itself hold several words (`"fondos animados"`).
fn terms<S: AsRef<str>>(terms: &[S]) -> Vec<String> {
    terms
        .iter()
        .flat_map(|t| {
            fold(t.as_ref())
                .split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The pages a search finds, best first: those whose name matches every term, then those
/// whose words do. Nothing for no terms.
pub fn find<S: AsRef<str>>(search: &[S]) -> Vec<&'static Page> {
    let terms = terms(search);
    if terms.is_empty() {
        return Vec::new();
    }
    let mut found: Vec<(u8, usize, &'static Page)> = Vec::new();
    for (position, page) in pages::all().enumerate() {
        let Some((_, names, words)) = WORDS.iter().find(|(id, ..)| *id == page.id) else {
            continue;
        };
        let rank = if every_term_starts_a_word(&terms, names) {
            0
        } else if every_term_starts_a_word(&terms, &format!("{names} {words}")) {
            1
        } else {
            continue;
        };
        found.push((rank, position, page));
    }
    found.sort_by_key(|(rank, position, _)| (*rank, *position));
    found.into_iter().map(|(.., page)| page).collect()
}

/// The result id of a page.
pub fn result_id(page: &Page) -> String {
    format!("{RESULT_PREFIX}{}", page.id)
}

/// The page a result id names.
pub fn page_of(result: &str) -> Option<&'static Page> {
    pages::find(result.strip_prefix(RESULT_PREFIX)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(search: &[&str]) -> Vec<String> {
        find(search).iter().map(|p| result_id(p)).collect()
    }

    #[test]
    fn every_page_has_words_and_every_entry_is_a_page() {
        for page in pages::all() {
            assert!(
                WORDS.iter().any(|(id, ..)| *id == page.id),
                "{} has no search words",
                page.id
            );
        }
        for (id, ..) in WORDS {
            assert!(pages::find(id).is_some(), "{id} is not a page");
        }
    }

    #[test]
    fn finds_the_storage_page_for_the_question_the_installer_checks_asks() {
        assert!(ids(&["documentos", "grandes"]).contains(&"nodalix-storage".to_owned()));
        assert!(ids(&["large", "files"]).contains(&"nodalix-storage".to_owned()));
    }

    #[test]
    fn finds_by_what_is_inside_in_both_languages() {
        let has = |terms: &[&str], id: &str| ids(terms).contains(&id.to_owned());
        assert!(has(&["actualizaciones", "automáticas"], "nodalix-updates"));
        assert!(has(&["fondos", "animados"], "nodalix-appearance"));
        assert!(has(&["localsend"], "nodalix-localsend"));
        assert!(has(&["versión", "nodalix"], "nodalix-about"));
        assert!(has(&["luz", "nocturna"], "nodalix-display"));
        assert!(has(&["night", "light"], "nodalix-display"));
        assert!(has(&["contraseña"], "nodalix-users"));
        assert!(has(&["WIFI"], "nodalix-wifi"));
    }

    #[test]
    fn a_term_matches_the_start_of_a_word_not_the_middle() {
        assert!(ids(&["alt"]).contains(&"nodalix-sound".to_owned()));
        assert!(!ids(&["ltavoces"]).contains(&"nodalix-sound".to_owned()));
    }

    #[test]
    fn every_term_has_to_match() {
        assert!(ids(&["bluetooth", "impresora"]).is_empty());
    }

    #[test]
    fn a_page_found_by_its_name_comes_before_one_found_by_its_words() {
        // "sonido" names the Sound page; no other page may come before it.
        assert_eq!(
            ids(&["sonido"]).first().map(String::as_str),
            Some("nodalix-sound")
        );
        assert_eq!(
            ids(&["red"]).first().map(String::as_str),
            Some("nodalix-network")
        );
    }

    #[test]
    fn nothing_for_no_terms_or_for_punctuation() {
        assert!(ids(&[]).is_empty());
        assert!(ids(&["", "  ", "-"]).is_empty());
    }

    #[test]
    fn result_ids_name_pages_and_only_pages() {
        for page in pages::all() {
            assert_eq!(page_of(&result_id(page)).map(|p| p.id), Some(page.id));
        }
        assert!(page_of("nodalix-nope").is_none());
        assert!(page_of("storage").is_none());
    }

    #[test]
    fn folding_removes_accents_and_case() {
        assert_eq!(fold("Contraseña ÚLTIMA"), "contrasena ultima");
        assert_eq!(fold("Wi-Fi"), "wi fi");
    }
}
