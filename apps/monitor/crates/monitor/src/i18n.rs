//! The monitor's Spanish strings.

const ES: &[(&str, &str)] = &[
    ("System Monitor", "Monitor del sistema"),
    ("Processes", "Procesos"),
    ("Resources", "Recursos"),
    ("File systems", "Sistemas de archivos"),
    ("Search processes", "Buscar procesos"),
    ("End process", "Terminar proceso"),
    ("Force quit", "Forzar cierre"),
    ("Stop", "Detener"),
    ("Continue", "Continuar"),
    ("Name", "Nombre"),
    ("User", "Usuario"),
    ("% CPU", "% CPU"),
    ("Memory", "Memoria"),
    ("ID", "ID"),
    ("Threads", "Hilos"),
    ("State", "Estado"),
    ("Command", "Comando"),
    ("CPU", "CPU"),
    ("Swap", "Intercambio"),
    ("Network", "Red"),
    ("Download", "Descarga"),
    ("Upload", "Subida"),
    ("Cores", "Núcleos"),
    ("of", "de"),
    ("Used", "Usado"),
    ("Free", "Libre"),
    ("About", "Acerca de"),
    ("Running", "En ejecución"),
    ("Sleeping", "Durmiendo"),
    ("Waiting for disk", "Esperando al disco"),
    ("Zombie", "Zombi"),
    ("Stopped", "Detenido"),
    ("Idle", "Inactivo"),
    ("Unknown", "Desconocido"),
    (
        "No mounted file systems",
        "No hay sistemas de archivos montados",
    ),
    (
        "A system monitor for Nodalix.",
        "Un monitor del sistema para Nodalix.",
    ),
];

/// The text in the user's language.
pub fn tr(text: &'static str) -> &'static str {
    nodalix_ui::translate(text, ES)
}
