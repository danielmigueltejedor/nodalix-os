//! Network locations: what the "Connect to server" box accepts, and the ones used before.

/// The schemes GVfs can mount and the file manager can then browse.
const SCHEMES: [&str; 9] = [
    "smb", "sftp", "ssh", "ftp", "ftps", "dav", "davs", "nfs", "afp",
];

/// The address as GVfs wants it, or nothing if it is not one it can mount. `ssh://` is how most
/// people write what GVfs calls `sftp://`; a bare `host` or `host/share` is read as `smb://`,
/// which is what a Windows user types.
pub fn normalize(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() || text.contains(char::is_whitespace) {
        return None;
    }
    let text = text.strip_prefix("\\\\").map_or_else(
        || text.to_owned(),
        |unc| format!("smb://{}", unc.replace('\\', "/")),
    );
    let (scheme, rest) = match text.split_once("://") {
        Some((scheme, rest)) => (scheme.to_lowercase(), rest.to_owned()),
        None => ("smb".to_owned(), text),
    };
    if !SCHEMES.contains(&scheme.as_str()) || rest.is_empty() || rest.starts_with('/') {
        return None;
    }
    let scheme = if scheme == "ssh" {
        "sftp".to_owned()
    } else {
        scheme
    };
    Some(format!("{scheme}://{rest}"))
}

/// What to call the address in the sidebar: the host, then the share or folder if there is one.
pub fn label(uri: &str) -> String {
    let rest = uri.split_once("://").map_or(uri, |(_, rest)| rest);
    let rest = rest.rsplit_once('@').map_or(rest, |(_, host)| host);
    let rest = rest.trim_end_matches('/');
    match rest.split_once('/') {
        Some((host, path)) => {
            let last = path.rsplit('/').next().unwrap_or(path);
            format!("{last} on {host}")
        }
        None => rest.to_owned(),
    }
}

/// How many are kept.
pub const KEEP: usize = 8;

/// The addresses in a saved list, one per line, newest first.
pub fn parse(text: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for uri in text.lines().filter_map(normalize) {
        if !found.contains(&uri) {
            found.push(uri);
        }
    }
    found.truncate(KEEP);
    found
}

/// `uri` first, then the others, at most [`KEEP`] in all.
pub fn remember(list: &[String], uri: &str) -> Vec<String> {
    let mut out = vec![uri.to_owned()];
    out.extend(list.iter().filter(|u| *u != uri).cloned());
    out.truncate(KEEP);
    out
}

pub fn serialize(list: &[String]) -> String {
    list.iter().map(|u| format!("{u}\n")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_what_gvfs_can_mount() {
        assert_eq!(
            normalize("smb://nas/photos").as_deref(),
            Some("smb://nas/photos")
        );
        assert_eq!(
            normalize(" SFTP://me@host ").as_deref(),
            Some("sftp://me@host")
        );
        assert_eq!(
            normalize("ssh://host/home").as_deref(),
            Some("sftp://host/home")
        );
        assert_eq!(normalize("nas/photos").as_deref(), Some("smb://nas/photos"));
        assert_eq!(
            normalize("\\\\nas\\photos").as_deref(),
            Some("smb://nas/photos")
        );
    }

    #[test]
    fn refuses_the_rest() {
        for bad in [
            "",
            "   ",
            "http://example.com",
            "smb://",
            "smb:///x",
            "a b",
            "file:///etc",
        ] {
            assert_eq!(normalize(bad), None, "{bad}");
        }
    }

    #[test]
    fn labels_say_where_it_is() {
        assert_eq!(label("smb://nas/photos"), "photos on nas");
        assert_eq!(label("sftp://me@host"), "host");
        assert_eq!(label("sftp://me@host/home/me/"), "me on host");
    }

    #[test]
    fn the_list_is_newest_first_without_repeats_and_bounded() {
        let mut list = Vec::new();
        for i in 0..12 {
            list = remember(&list, &format!("smb://host{i}"));
        }
        assert_eq!(list.len(), KEEP);
        assert_eq!(list[0], "smb://host11");
        let again = remember(&list, "smb://host8");
        assert_eq!(again[0], "smb://host8");
        assert_eq!(again.iter().filter(|u| *u == "smb://host8").count(), 1);
        assert_eq!(parse(&serialize(&list)), list);
    }
}
