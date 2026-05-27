use std::{env, fs, path::PathBuf};

#[derive(Clone, Debug)]
pub struct LocalUser {
    pub username: String,
    pub display_name: String,
    pub uid: u32,
    pub home: PathBuf,
    pub avatar: Option<PathBuf>,
}

pub fn current_user() -> LocalUser {
    let username = env::var("USER").unwrap_or_else(|_| "usuario".to_string());
    local_users()
        .into_iter()
        .find(|user| user.username == username)
        .unwrap_or(LocalUser {
            username: username.clone(),
            display_name: username,
            uid: unsafe { libc_getuid() },
            home: env::var("HOME").map(PathBuf::from).unwrap_or_default(),
            avatar: None,
        })
}

pub fn local_users() -> Vec<LocalUser> {
    let Ok(passwd) = fs::read_to_string("/etc/passwd") else {
        return Vec::new();
    };
    let mut users = passwd
        .lines()
        .filter_map(parse_line)
        .filter(|user| user.uid >= 1000 && user.username != "nobody" && user.username != "greeter")
        .collect::<Vec<_>>();
    users.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
    });
    users
}

pub fn set_avatar_from_file(source: &std::path::Path) -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME no definido".to_string())?;
    let dest = home.join(".face");
    std::fs::copy(source, &dest)
        .map_err(|e| format!("No se pudo copiar la imagen: {e}"))?;
    Ok(dest)
}

pub fn avatar_candidates(user: &LocalUser) -> Vec<PathBuf> {
    vec![
        PathBuf::from(format!("/var/lib/AccountsService/icons/{}", user.username)),
        user.home.join(".face"),
        user.home.join(".face.icon"),
    ]
}

fn parse_line(line: &str) -> Option<LocalUser> {
    let mut parts = line.split(':');
    let username = parts.next()?.to_string();
    let _password = parts.next()?;
    let uid = parts.next()?.parse().ok()?;
    let _gid = parts.next()?;
    let gecos = parts.next().unwrap_or_default();
    let home = PathBuf::from(parts.next().unwrap_or_default());
    let shell = parts.next().unwrap_or_default();
    if shell.contains("nologin") || shell.contains("false") {
        return None;
    }
    let display_name = gecos
        .split(',')
        .next()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(&username)
        .trim()
        .to_string();
    let avatar = avatar_candidates(&LocalUser {
        username: username.clone(),
        display_name: display_name.clone(),
        uid,
        home: home.clone(),
        avatar: None,
    })
    .into_iter()
    .find(|path| path.is_file());
    Some(LocalUser {
        username,
        display_name,
        uid,
        home,
        avatar,
    })
}

unsafe fn libc_getuid() -> u32 {
    unsafe extern "C" {
        fn getuid() -> u32;
    }
    getuid()
}
