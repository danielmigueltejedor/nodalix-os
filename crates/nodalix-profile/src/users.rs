use crate::profile::{configured_avatar_path, load_profile_for_home};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

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
            display_name: username.clone(),
            uid: current_uid(),
            home: env::var("HOME").map(PathBuf::from).unwrap_or_default(),
            avatar: resolve_avatar(
                &username,
                &env::var("HOME").map(PathBuf::from).unwrap_or_default(),
            ),
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

pub fn resolve_avatar(username: &str, home: &Path) -> Option<PathBuf> {
    let profile = load_profile_for_home(home);
    if let Some(path) = configured_avatar_path(home, &profile) {
        return Some(path);
    }

    avatar_candidates(username, home)
        .into_iter()
        .find(|path| path.is_file())
}

pub fn avatar_candidates(username: &str, home: &Path) -> Vec<PathBuf> {
    vec![
        PathBuf::from(format!("/var/lib/AccountsService/icons/{username}")),
        home.join(".face"),
        home.join(".face.icon"),
        crate::paths::default_avatar_path_for_home(home),
    ]
}

pub fn initials_from_name(name: &str) -> String {
    let result = name
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>();
    if result.is_empty() {
        let one = name.chars().take(1).collect::<String>().to_uppercase();
        if one.is_empty() {
            "?".to_string()
        } else {
            one
        }
    } else {
        result.to_uppercase()
    }
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
    let profile = load_profile_for_home(&home);
    let display_name = profile
        .display_name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(display_name);
    let avatar = resolve_avatar(&username, &home);
    Some(LocalUser {
        username,
        display_name,
        uid,
        home,
        avatar,
    })
}

fn current_uid() -> u32 {
    unsafe extern "C" {
        fn getuid() -> u32;
    }
    unsafe { getuid() }
}
