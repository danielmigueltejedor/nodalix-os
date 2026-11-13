//! Which colour profiles a device can use, for the list a person picks from.

/// A profile as colord knows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    pub path: String,
    pub title: String,
    /// `rgb`, `gray`, `cmyk`...
    pub colorspace: String,
}

/// A device kind and the colour spaces its profiles are in: a screen shows RGB, a scanner or
/// camera captures it; a printer takes several.
pub fn spaces_for(kind: &str) -> &'static [&'static str] {
    match kind {
        "display" | "scanner" | "camera" | "webcam" => &["rgb"],
        "printer" => &["rgb", "cmyk", "gray"],
        _ => &["rgb", "cmyk", "gray"],
    }
}

/// The profiles to list for a device: those already assigned in their order (the first is the
/// one in use), then the others that fit its kind, by title. Each comes with whether it is
/// assigned.
pub fn choices(kind: &str, assigned: &[String], all: &[Profile]) -> Vec<(Profile, bool)> {
    let mut out: Vec<(Profile, bool)> = assigned
        .iter()
        .filter_map(|path| all.iter().find(|p| &p.path == path))
        .map(|p| (p.clone(), true))
        .collect();
    let mut others: Vec<&Profile> = all
        .iter()
        .filter(|p| !assigned.contains(&p.path))
        .filter(|p| spaces_for(kind).contains(&p.colorspace.as_str()))
        .collect();
    others.sort_by_key(|p| p.title.to_lowercase());
    out.extend(others.into_iter().map(|p| (p.clone(), false)));
    out
}

/// Whether a file name is a colour profile a person may import.
pub fn is_profile_file(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".icc") || lower.ends_with(".icm")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(path: &str, title: &str, space: &str) -> Profile {
        Profile {
            path: path.to_owned(),
            title: title.to_owned(),
            colorspace: space.to_owned(),
        }
    }

    #[test]
    fn assigned_profiles_come_first_then_the_fitting_others_by_title() {
        let all = vec![
            p("/b", "sRGB", "rgb"),
            p("/a", "AdobeRGB", "rgb"),
            p("/c", "Printer CMYK", "cmyk"),
            p("/d", "Panel", "rgb"),
        ];
        let got = choices("display", &["/d".to_owned()], &all);
        let titles: Vec<&str> = got.iter().map(|(p, _)| p.title.as_str()).collect();
        assert_eq!(titles, ["Panel", "AdobeRGB", "sRGB"]);
        assert!(got[0].1 && !got[1].1);
    }

    #[test]
    fn a_printer_may_use_cmyk_profiles() {
        let all = vec![p("/c", "Press", "cmyk")];
        assert_eq!(choices("printer", &[], &all).len(), 1);
        assert!(choices("display", &[], &all).is_empty());
    }

    #[test]
    fn an_assigned_path_colord_no_longer_lists_is_skipped() {
        assert!(choices("display", &["/gone".to_owned()], &[]).is_empty());
    }

    #[test]
    fn only_icc_files_are_imported() {
        assert!(is_profile_file("Monitor.ICC"));
        assert!(is_profile_file("a.icm"));
        assert!(!is_profile_file("a.png"));
    }
}
