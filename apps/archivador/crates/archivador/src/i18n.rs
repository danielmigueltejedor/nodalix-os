//! Archivador's Spanish strings.

const ES: &[(&str, &str)] = &[
    ("Files", "Archivos"),
    ("Home", "Carpeta personal"),
    ("Desktop", "Escritorio"),
    ("Documents", "Documentos"),
    ("Downloads", "Descargas"),
    ("Music", "Música"),
    ("Pictures", "Imágenes"),
    ("Videos", "Vídeos"),
    ("Trash", "Papelera"),
    ("Devices", "Dispositivos"),
    ("Back", "Atrás"),
    ("Forward", "Adelante"),
    ("Parent folder", "Carpeta superior"),
    ("Search", "Buscar"),
    ("Search in this folder", "Buscar en esta carpeta"),
    ("Show as a grid", "Ver como cuadrícula"),
    ("Show as a list", "Ver como lista"),
    ("New folder", "Carpeta nueva"),
    ("Untitled folder", "Carpeta sin título"),
    ("Open", "Abrir"),
    ("Copy", "Copiar"),
    ("Cut", "Cortar"),
    ("Paste", "Pegar"),
    ("Rename…", "Cambiar nombre…"),
    ("Rename", "Cambiar nombre"),
    ("Move to the trash", "Mover a la papelera"),
    ("Delete permanently…", "Eliminar para siempre…"),
    ("Delete permanently", "Eliminar para siempre"),
    ("Restore", "Restaurar"),
    ("Empty the trash…", "Vaciar la papelera…"),
    ("Empty the trash", "Vaciar la papelera"),
    ("Select all", "Seleccionar todo"),
    ("Send with LocalSend…", "Enviar con LocalSend…"),
    ("Send with LocalSend", "Enviar con LocalSend"),
    ("Refresh", "Actualizar"),
    ("Sort by", "Ordenar por"),
    ("Name", "Nombre"),
    ("Modified", "Modificado"),
    ("Size", "Tamaño"),
    ("Type", "Tipo"),
    ("Reverse order", "Orden inverso"),
    ("Show hidden files", "Mostrar archivos ocultos"),
    ("About", "Acerca de"),
    ("Cancel", "Cancelar"),
    ("Create", "Crear"),
    ("Delete", "Eliminar"),
    ("This folder is empty", "Esta carpeta está vacía"),
    ("The trash is empty", "La papelera está vacía"),
    (
        "Nothing matches your search",
        "Nada coincide con tu búsqueda",
    ),
    ("Can't open this folder", "No se puede abrir esta carpeta"),
    ("Can't open this file", "No se puede abrir este archivo"),
    ("Move to the trash?", "¿Mover a la papelera?"),
    (
        "It can't be moved to the trash",
        "No se puede mover a la papelera",
    ),
    (
        "It will be deleted for good. This can't be undone.",
        "Se eliminará para siempre. No se puede deshacer.",
    ),
    (
        "Everything in the trash will be deleted for good.",
        "Todo lo de la papelera se eliminará para siempre.",
    ),
    ("Nearby devices", "Dispositivos cercanos"),
    ("No devices nearby", "No hay dispositivos cerca"),
    (
        "Open LocalSend on the other device, or switch it on in Settings.",
        "Abre LocalSend en el otro dispositivo, o actívalo en Ajustes.",
    ),
    (
        "Switch LocalSend on in Settings.",
        "Activa LocalSend en Ajustes.",
    ),
    ("LocalSend isn't running", "LocalSend no está en ejecución"),
    (
        "Archivador, the Nodalix file manager.",
        "Archivador, el gestor de archivos de Nodalix.",
    ),
    ("Close", "Cerrar"),
    ("{} found", "{} encontrados"),
    ("{} selected", "{} seleccionados"),
    ("Can't open {}", "No se puede abrir {}"),
    ("Moved {} to the trash", "{} movidos a la papelera"),
    ("Deleted {}", "{} eliminados"),
    ("Restored {}", "{} restaurados"),
    ("Sending to {}…", "Enviando a {}…"),
];

/// The text in the user's language.
pub fn tr(text: &'static str) -> &'static str {
    nodalix_ui::translate(text, ES)
}

/// `Moved 3 items to the trash` and the like: the count and its noun in the user's language.
pub fn items(n: usize) -> String {
    match (nodalix_ui::spanish(), n) {
        (true, 1) => "1 elemento".into(),
        (true, n) => format!("{n} elementos"),
        (false, 1) => "1 item".into(),
        (false, n) => format!("{n} items"),
    }
}

/// Message templates, filled by the caller. `{}` is replaced by the first argument.
pub fn fmt(text: &'static str, arg: &str) -> String {
    tr(text).replacen("{}", arg, 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spanish_strings_are_not_empty_or_repeated() {
        let mut seen = std::collections::HashSet::new();
        for (en, es) in ES {
            assert!(!en.is_empty() && !es.is_empty());
            assert!(seen.insert(*en), "{en} is listed twice");
        }
    }
}
