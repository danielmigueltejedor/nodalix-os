//! Settings' Spanish strings. Page titles live in `ajustes-core`, so they are listed here too.

const ES: &[(&str, &str)] = &[
    ("Settings", "Ajustes"),
    ("Search settings", "Buscar en los ajustes"),
    ("Appearance", "Apariencia"),
    ("Updates", "Actualizaciones"),
    ("Connections", "Conexiones"),
    ("Devices", "Dispositivos"),
    ("System", "Sistema"),
    ("Network", "Red"),
    ("Wi-Fi", "Wi-Fi"),
    ("Bluetooth", "Bluetooth"),
    ("Sharing", "Compartir"),
    ("Online accounts", "Cuentas en línea"),
    ("Displays", "Pantallas"),
    ("Sound", "Sonido"),
    ("Power", "Energía"),
    ("Keyboard", "Teclado"),
    ("Mouse and touchpad", "Ratón y panel táctil"),
    ("Printers", "Impresoras"),
    ("Color", "Color"),
    ("Notifications", "Notificaciones"),
    ("Multitasking", "Multitarea"),
    ("Apps", "Aplicaciones"),
    ("Privacy and security", "Privacidad y seguridad"),
    ("Accessibility", "Accesibilidad"),
    ("Date and time", "Fecha y hora"),
    ("Region and language", "Región e idioma"),
    ("Users", "Usuarios"),
    ("About", "Acerca de"),
    ("Open", "Abrir"),
    ("This page has not been ported to the new Settings yet. It opens in the previous Settings window.", "Esta página aún no está en los nuevos Ajustes. Se abre en la ventana de los Ajustes anteriores."),
    ("Could not open the previous Settings", "No se pudo abrir los Ajustes anteriores"),
    ("Style", "Estilo"),
    ("Color scheme", "Esquema de color"),
    ("Follow the system", "Según el sistema"),
    ("Light", "Claro"),
    ("Dark", "Oscuro"),
    ("Accent color", "Color de acento"),
    ("Blue", "Azul"),
    ("Teal", "Turquesa"),
    ("Green", "Verde"),
    ("Yellow", "Amarillo"),
    ("Orange", "Naranja"),
    ("Red", "Rojo"),
    ("Pink", "Rosa"),
    ("Purple", "Morado"),
    ("Slate", "Pizarra"),
    ("Text size", "Tamaño del texto"),
    ("Play animations in the interface", "Reproducir animaciones en la interfaz"),
    ("Backgrounds", "Fondos"),
    ("Pictures and animated backgrounds", "Imágenes y fondos animados"),
    ("Device", "Dispositivo"),
    ("Device name", "Nombre del dispositivo"),
    ("Version", "Versión"),
    ("Kernel", "Kernel"),
    ("Architecture", "Arquitectura"),
    ("Processor", "Procesador"),
    ("Memory", "Memoria"),
    ("Disk", "Disco"),
    ("Unknown", "Desconocido"),
    ("Status", "Estado"),
    ("Check for updates", "Buscar actualizaciones"),
    ("Update everything", "Actualizar todo"),
    ("Update", "Actualizar"),
    ("Restart", "Reiniciar"),
    ("Sources", "Fuentes"),
    ("Details", "Detalles"),
    ("Last checked", "Última comprobación"),
    ("Restart to finish the update", "Reinicia para terminar la actualización"),
    ("The update service is not running. It starts with your session; sign out and in again if this stays.", "El servicio de actualizaciones no está en marcha. Arranca con tu sesión; cierra sesión y vuelve a entrar si sigue así."),
    ("Not installed", "No instalado"),
    ("Show the update log", "Mostrar el registro"),
];

/// The text in the user's language.
pub fn tr(text: &'static str) -> &'static str {
    nodalix_ui::translate(text, ES)
}

/// Whether the catalog has a Spanish text for `text`.
#[cfg(test)]
pub fn has_translation(text: &str) -> bool {
    ES.iter().any(|(en, _)| *en == text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_page_and_section_title_is_translated() {
        for (section, pages) in ajustes_core::pages::SECTIONS {
            if !section.is_empty() {
                assert!(has_translation(section), "{section}");
            }
            for page in pages.iter() {
                assert!(has_translation(page.title), "{}", page.title);
            }
        }
    }

    #[test]
    fn the_catalog_has_no_duplicates() {
        let mut seen = std::collections::HashSet::new();
        for (en, _) in ES {
            assert!(seen.insert(*en), "{en} appears twice");
        }
    }
}
