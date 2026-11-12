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
    ("Recent", "Recientes"),
    ("No recent files", "No hay archivos recientes"),
    ("Favorites", "Favoritos"),
    ("Add to Favorites", "Añadir a Favoritos"),
    ("Remove from Favorites", "Quitar de Favoritos"),
    ("Added to Favorites", "Añadida a Favoritos"),
    ("Removed from Favorites", "Quitada de Favoritos"),
    ("Network", "Red"),
    ("Connect to a server…", "Conectar con un servidor…"),
    ("Connect to a server", "Conectar con un servidor"),
    (
        "Type the address, like smb://server/share, sftp://host or ftp://host.",
        "Escribe la dirección, como smb://servidor/recurso, sftp://equipo o ftp://equipo.",
    ),
    ("Connect", "Conectar"),
    ("Can't browse {}", "No se puede explorar {}"),
    ("Devices", "Dispositivos"),
    ("Back", "Atrás"),
    ("Forward", "Adelante"),
    ("Parent folder", "Carpeta superior"),
    ("Search", "Buscar"),
    ("Search in this folder", "Buscar en esta carpeta"),
    ("Show as a grid", "Ver como cuadrícula"),
    ("Show as a list", "Ver como lista"),
    ("Show in columns", "Ver en columnas"),
    ("Cloud", "Nube"),
    ("Preferences", "Preferencias"),
    ("Style", "Estilo"),
    ("Appearance", "Apariencia"),
    ("Follow the system", "Seguir el sistema"),
    ("Light", "Claro"),
    ("Dark", "Oscuro"),
    ("Show files as", "Ver los archivos como"),
    ("List", "Lista"),
    ("Grid", "Cuadrícula"),
    ("Columns", "Columnas"),
    ("View", "Vista"),
    ("Sidebar", "Barra lateral"),
    ("Show the file system", "Mostrar el sistema de archivos"),
    (
        "The accounts you connect show up in the sidebar with their own icon.",
        "Las cuentas que conectes aparecen en la barra lateral con su propio icono.",
    ),
    ("Connected", "Conectada"),
    ("Not connected", "Sin conexión"),
    (
        "Set up a cloud account…",
        "Configurar una cuenta en la nube…",
    ),
    ("Add or manage accounts…", "Añadir o gestionar cuentas…"),
    ("Can't open Settings", "No se puede abrir Ajustes"),
    ("Show the folders above", "Mostrar las carpetas de arriba"),
    ("Applications", "Aplicaciones"),
    (
        "Not connected. Click to connect",
        "Sin conexión. Pulsa para conectar",
    ),
    ("Can't connect {}", "No se puede conectar {}"),
    ("Can't disconnect {}", "No se puede desconectar {}"),
    ("Disconnect", "Desconectar"),
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
    ("Properties", "Propiedades"),
    ("Open With…", "Abrir con…"),
    ("Folder", "Carpeta"),
    ("Location", "Ubicación"),
    ("Accessed", "Último acceso"),
    ("Permissions", "Permisos"),
    ("Calculating…", "Calculando…"),
    ("{} files", "{} archivos"),
    ("Eject", "Expulsar"),
    ("Can't unmount {}", "No se puede desmontar {}"),
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
