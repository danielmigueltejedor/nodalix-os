use super::{command_exists, run_command_stdout, spawn_detached};

#[derive(Debug, Clone)]
pub struct UpdateReport {
    pub pacman_pending: usize,
    pub aur_pending: usize,
    pub lines: Vec<String>,
}

impl UpdateReport {
    pub fn total(&self) -> usize {
        self.pacman_pending + self.aur_pending
    }

    pub fn summary(&self) -> String {
        if self.total() == 0 {
            "Sistema actualizado".to_string()
        } else {
            format!(
                "{} actualización(es) · {} repo · {} AUR",
                self.total(),
                self.pacman_pending,
                self.aur_pending
            )
        }
    }
}

pub fn check_updates() -> Result<UpdateReport, String> {
    let pacman_result = run_command_stdout("checkupdates", &[]);
    let pacman_lines: Vec<String> = pacman_result
        .as_ref()
        .map(|o| {
            o.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();
    let pacman_pending = pacman_lines.len();

    let mut aur_pending = 0;
    let mut aur_lines = Vec::new();
    if command_exists("yay") {
        if let Ok(out) = run_command_stdout("yay", &["-Qu", "--noconfirm"]) {
            aur_lines = out
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(String::from)
                .collect();
            aur_pending = aur_lines.len();
        }
    }

    if pacman_result.is_err() && !command_exists("yay") {
        return Err(
            "Instala pacman-contrib (checkupdates) o yay para comprobar actualizaciones"
                .to_string(),
        );
    }

    let mut lines = pacman_lines;
    lines.extend(aur_lines);

    Ok(UpdateReport {
        pacman_pending,
        aur_pending,
        lines,
    })
}

pub fn launch_update_terminal() -> Result<(), String> {
    let cmd = if command_exists("yay") {
        "echo 'Actualizando sistema Nodalix…'; yay -Syu"
    } else {
        "echo 'Actualizando sistema…'; sudo pacman -Syu"
    };

    if command_exists("ghostty") {
        return spawn_detached("ghostty", &["-e", "bash", "-lc", cmd]);
    }
    if command_exists("foot") {
        return spawn_detached("foot", &["bash", "-lc", cmd]);
    }
    Err(format!(
        "Abre una terminal y ejecuta:\n  {cmd}\n\nSe pedirá contraseña para paquetes del sistema."
    ))
}
