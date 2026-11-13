//! Icon and cursor themes found in the icon folders: a folder with an `index.theme` is a theme;
//! it is an icon theme when the index lists directories and a cursor theme when it has a
//! `cursors` folder.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    /// The folder name, which is what GSettings stores.
    pub id: String,
    /// The theme's own name, for the list.
    pub name: String,
    pub icons: bool,
    pub cursors: bool,
}

/// A theme from its folder name, its `index.theme` text and whether it has a `cursors` folder.
/// `None` for a hidden theme (`Hidden=true`) or one that is neither kind.
pub fn classify(id: &str, index: &str, has_cursors_dir: bool) -> Option<Theme> {
    let mut name = None;
    let mut directories = false;
    let mut hidden = false;
    let mut in_main = false;
    for line in index.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            // Only the `[Icon Theme]` group describes the theme itself.
            in_main = line == "[Icon Theme]";
            continue;
        }
        if !in_main {
            continue;
        }
        if let Some(value) = line.strip_prefix("Name=") {
            name.get_or_insert_with(|| value.trim().to_owned());
        } else if let Some(value) = line.strip_prefix("Directories=") {
            directories = !value.trim().is_empty();
        } else if line.eq_ignore_ascii_case("Hidden=true") {
            hidden = true;
        }
    }
    if hidden || (!directories && !has_cursors_dir) {
        return None;
    }
    Some(Theme {
        id: id.to_owned(),
        name: name
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| id.to_owned()),
        icons: directories,
        cursors: has_cursors_dir,
    })
}

/// Themes of one kind, by name, without repeats (the first folder with an id wins, so a theme in
/// the user's folder hides the system one).
pub fn sorted(mut themes: Vec<Theme>, cursors: bool) -> Vec<Theme> {
    themes.retain(|t| if cursors { t.cursors } else { t.icons });
    let mut seen = std::collections::HashSet::new();
    themes.retain(|t| seen.insert(t.id.clone()));
    themes.sort_by_key(|t| t.name.to_lowercase());
    themes
}

#[cfg(test)]
mod tests {
    use super::*;

    const ICONS: &str = "[Icon Theme]\nName=Colloid Teal\nDirectories=scalable/apps,16x16/apps\n\n[scalable/apps]\nSize=64\n";

    #[test]
    fn an_icon_theme_is_recognised_by_its_directories() {
        let t = classify("Colloid-Teal", ICONS, false).unwrap();
        assert!(t.icons && !t.cursors);
        assert_eq!(t.name, "Colloid Teal");
    }

    #[test]
    fn a_cursor_theme_is_recognised_by_its_folder() {
        let t = classify("Bibata", "[Icon Theme]\nName=Bibata\n", true).unwrap();
        assert!(t.cursors && !t.icons);
    }

    #[test]
    fn hidden_and_empty_themes_are_skipped() {
        assert!(classify("x", "[Icon Theme]\nHidden=true\nDirectories=a\n", false).is_none());
        assert!(classify("x", "[Icon Theme]\nName=Empty\n", false).is_none());
    }

    #[test]
    fn a_name_in_another_group_is_not_the_themes() {
        let index = "[Icon Theme]\nDirectories=a\n[a]\nName=Wrong\n";
        assert_eq!(classify("folder", index, false).unwrap().name, "folder");
    }

    #[test]
    fn the_list_has_one_theme_per_id_by_name() {
        let a = classify("b", "[Icon Theme]\nName=Beta\nDirectories=a\n", false).unwrap();
        let b = classify("a", "[Icon Theme]\nName=alpha\nDirectories=a\n", false).unwrap();
        let again = Theme {
            name: "Other".into(),
            ..a.clone()
        };
        let list = sorted(vec![a, b, again], false);
        assert_eq!(
            list.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
            ["a", "b"]
        );
    }
}
