use nodalix_profile::resolve_avatar;
use std::{fs, path::PathBuf};

#[derive(Clone, Debug)]
pub struct GreeterUser {
    pub username: String,
    pub display_name: String,
    pub avatar: Option<PathBuf>,
}

pub fn load_human_users() -> Vec<GreeterUser> {
    let Ok(passwd) = fs::read_to_string("/etc/passwd") else {
        eprintln!("nodalix-greeter: unable to read /etc/passwd");
        return Vec::new();
    };

    let mut users = passwd
        .lines()
        .filter_map(parse_passwd_line)
        .filter(is_human_user)
        .map(|entry| {
            let profile = nodalix_profile::load_profile_for_home(&entry.home);
            let display_name = profile
                .display_name
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| display_name(&entry.gecos, &entry.fallback_name));
            let avatar = resolve_avatar(&entry.username, &entry.home);
            GreeterUser {
                username: entry.username,
                display_name,
                avatar,
            }
        })
        .collect::<Vec<_>>();

    users.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
    });
    users
}

pub fn demo_users() -> Vec<GreeterUser> {
    vec![
        GreeterUser {
            username: "dani".to_string(),
            display_name: "Dani".to_string(),
            avatar: None,
        },
        GreeterUser {
            username: "nodalix".to_string(),
            display_name: "Nodalix Demo".to_string(),
            avatar: None,
        },
    ]
}

pub fn initials(name: &str) -> String {
    nodalix_profile::initials_from_name(name)
}

#[derive(Debug)]
struct PasswdEntry {
    username: String,
    fallback_name: String,
    uid: u32,
    gecos: String,
    home: PathBuf,
    shell: String,
}

fn parse_passwd_line(line: &str) -> Option<PasswdEntry> {
    let mut parts = line.split(':');
    let username = parts.next()?.to_string();
    let _password = parts.next()?;
    let uid = parts.next()?.parse::<u32>().ok()?;
    let _gid = parts.next()?;
    let gecos = parts.next().unwrap_or_default().to_string();
    let home = PathBuf::from(parts.next().unwrap_or_default());
    let shell = parts.next().unwrap_or_default().to_string();

    Some(PasswdEntry {
        fallback_name: username.clone(),
        username,
        uid,
        gecos,
        home,
        shell,
    })
}

fn is_human_user(entry: &PasswdEntry) -> bool {
    entry.uid >= 1000
        && entry.username != "nobody"
        && entry.username != "greeter"
        && !entry.shell.contains("nologin")
        && !entry.shell.contains("false")
}

fn display_name(gecos: &str, username: &str) -> String {
    let full_name = gecos.split(',').next().unwrap_or_default().trim();
    if full_name.is_empty() {
        username.to_string()
    } else {
        full_name.to_string()
    }
}
