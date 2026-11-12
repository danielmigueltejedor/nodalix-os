//! A tiny catalog: English is the source text, Spanish the one translation. The OS ships its own
//! locale handling for the rest of the desktop; this app only needs its few dozen strings.

use std::sync::OnceLock;

fn spanish() -> bool {
    static ES: OnceLock<bool> = OnceLock::new();
    *ES.get_or_init(|| {
        ["LC_ALL", "LC_MESSAGES", "LANGUAGE", "LANG"]
            .iter()
            .filter_map(|k| std::env::var(k).ok())
            .find(|v| !v.is_empty())
            .is_some_and(|v| v.starts_with("es"))
    })
}

/// The text in the user's language.
pub fn tr(text: &'static str) -> &'static str {
    if !spanish() {
        return text;
    }
    match text {
        "System Monitor" => "Monitor del sistema",
        "Processes" => "Procesos",
        "Resources" => "Recursos",
        "File systems" => "Sistemas de archivos",
        "Search processes" => "Buscar procesos",
        "End process" => "Terminar proceso",
        "Force quit" => "Forzar cierre",
        "Stop" => "Detener",
        "Continue" => "Continuar",
        "Name" => "Nombre",
        "User" => "Usuario",
        "% CPU" => "% CPU",
        "Memory" => "Memoria",
        "ID" => "ID",
        "Threads" => "Hilos",
        "State" => "Estado",
        "Command" => "Comando",
        "CPU" => "CPU",
        "Swap" => "Intercambio",
        "Network" => "Red",
        "Download" => "Descarga",
        "Upload" => "Subida",
        "Cores" => "Núcleos",
        "Used" => "Usado",
        "Free" => "Libre",
        "About" => "Acerca de",
        "Running" => "En ejecución",
        "Sleeping" => "Durmiendo",
        "Waiting for disk" => "Esperando al disco",
        "Zombie" => "Zombi",
        "Stopped" => "Detenido",
        "Idle" => "Inactivo",
        "Unknown" => "Desconocido",
        "No mounted file systems" => "No hay sistemas de archivos montados",
        "A system monitor for Nodalix." => "Un monitor del sistema para Nodalix.",
        _ => text,
    }
}
