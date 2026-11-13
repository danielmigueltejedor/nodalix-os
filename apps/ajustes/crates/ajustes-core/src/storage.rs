//! The storage report: how full the disk is and what fills it, by kind, as the storage helper
//! (`/usr/lib/nodalix/storage-manager scan`) prints it. Measuring a home folder takes a while, so
//! the page asks for it in the background; this module only reads the answer.

use serde::Deserialize;

/// The kinds of thing that use space, in the order they are shown, with the icon of each.
pub const CATEGORIES: [(&str, &str); 6] = [
    ("apps", "view-app-grid-symbolic"),
    ("documents", "x-office-document-symbolic"),
    ("music", "audio-speakers-symbolic"),
    ("media", "image-x-generic-symbolic"),
    ("downloads", "folder-download-symbolic"),
    ("development", "application-x-executable-symbolic"),
];

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct Volume {
    pub total: u64,
    pub used: u64,
    pub free: u64,
}

impl Volume {
    /// How full the disk is, from 0 to 1.
    pub fn fraction(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.used as f64 / self.total as f64).clamp(0.0, 1.0)
        }
    }
}

/// One thing that takes space: a file or folder, or an application.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct Item {
    pub name: String,
    /// A file or folder.
    #[serde(default)]
    pub path: String,
    /// An application's identifier.
    #[serde(default)]
    pub id: String,
    /// Where a file is, or what kind of application it is.
    #[serde(default)]
    pub detail: String,
    /// Not every kind of application can be measured.
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub icon: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct Report {
    pub volume: Volume,
    #[serde(default)]
    pub categories: std::collections::HashMap<String, Vec<Item>>,
    #[serde(default)]
    pub totals: std::collections::HashMap<String, u64>,
    /// The measuring stopped at its limit, so the figures are a lower bound.
    #[serde(default)]
    pub partial: bool,
}

impl Report {
    pub fn parse(text: &str) -> Result<Report, serde_json::Error> {
        serde_json::from_str(text)
    }

    pub fn total(&self, category: &str) -> u64 {
        self.totals.get(category).copied().unwrap_or(0)
    }

    pub fn items(&self, category: &str) -> &[Item] {
        self.categories
            .get(category)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// What the categories add up to, which is a part of what the disk holds.
    pub fn measured(&self) -> u64 {
        CATEGORIES.iter().map(|(key, _)| self.total(key)).sum()
    }

    /// The part of the disk used by something the report does not list (the system itself).
    pub fn other(&self) -> u64 {
        self.volume.used.saturating_sub(self.measured())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"volume":{"total":1000,"used":600,"free":400},
        "categories":{"documents":[{"name":"a.pdf","path":"/h/a.pdf","detail":"/h","size":50}],
                      "apps":[{"name":"Firefox","id":"firefox.desktop","detail":"Paquete del sistema","size":null,"icon":"firefox"}]},
        "totals":{"documents":50,"apps":0,"music":10},"partial":true,"extra":1}"#;

    #[test]
    fn a_report_is_read_with_its_categories() {
        let report = Report::parse(SAMPLE).unwrap();
        assert_eq!(report.volume.total, 1000);
        assert!(report.partial);
        assert_eq!(report.items("documents")[0].name, "a.pdf");
        assert_eq!(report.items("apps")[0].size, None);
        assert!(report.items("music").is_empty());
        assert_eq!(report.total("music"), 10);
        assert_eq!(report.total("media"), 0);
    }

    #[test]
    fn what_the_categories_do_not_cover_is_other() {
        let report = Report::parse(SAMPLE).unwrap();
        assert_eq!(report.measured(), 60);
        assert_eq!(report.other(), 540);
        assert!((report.volume.fraction() - 0.6).abs() < 1e-9);
    }

    #[test]
    fn an_empty_disk_is_not_a_division_by_zero() {
        assert_eq!(Volume::default().fraction(), 0.0);
        assert!(Report::parse("nope").is_err());
    }
}
