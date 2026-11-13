//! The icons the apps need that NDL does not draw, drawn the way NDL draws its own: a 24 px grid,
//! a 2.25 px round stroke, and the foreground colour for the stroke, so they follow the text.
//!
//! They are installed as part of NDL's own icon theme (`Nodalia`), under the names the
//! freedesktop specification gives them, so they win over the ones Adwaita would answer with and
//! an app only asks for the name. Where NDL has the icon, nothing is added.

use std::path::PathBuf;

use ndl::gtk::{self, glib};
use nodalia_gtk as ndl;

/// The theme NDL installs, which the icons go into.
const THEME: &str = "Nodalia";

/// A folder, to which some of the icons add a small picture.
pub(crate) const FOLDER: &str = "M3.5 18.5v-12A1.5 1.5 0 0 1 5 5h4l2 2.5h8A1.5 1.5 0 0 1 20.5 9v9.5A1.5 1.5 0 0 1 19 20H5a1.5 1.5 0 0 1-1.5-1.5z";

/// One piece of a drawing.
pub(crate) enum Part {
    /// A line, an outline.
    Line(&'static str),
    /// A shape filled with the foreground colour.
    Solid(&'static str),
    /// A line drawn faintly: the part of a gauge that is not reached.
    Faint(&'static str),
    /// A circle: centre x, centre y, radius.
    Circle(f32, f32, f32),
    /// A rounded rectangle: x, y, width, height, corner radius.
    Rect(f32, f32, f32, f32, f32),
}

use Part::{Circle, Faint, Line, Rect, Solid};

const STROKE: &str =
    r#"stroke="rgb(0,0,0)" stroke-width="2.25" stroke-linecap="round" stroke-linejoin="round""#;

pub(crate) fn svg(parts: &[Part]) -> String {
    let mut out = String::from(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">"#,
    );
    for part in parts {
        match part {
            Line(d) => out.push_str(&format!(
                r#"<path class="transparent-fill foreground-stroke" fill="none" {STROKE} d="{d}"/>"#
            )),
            Faint(d) => out.push_str(&format!(
                r#"<path class="transparent-fill foreground-stroke" fill="none" {STROKE} stroke-opacity="0.3" d="{d}"/>"#
            )),
            Solid(d) => out.push_str(&format!(
                r#"<path class="foreground-fill foreground-stroke" fill="rgb(0,0,0)" {STROKE} d="{d}"/>"#
            )),
            Circle(x, y, r) => out.push_str(&format!(
                r#"<circle class="transparent-fill foreground-stroke" fill="none" {STROKE} cx="{x}" cy="{y}" r="{r}"/>"#
            )),
            Rect(x, y, w, h, r) => out.push_str(&format!(
                r#"<rect class="transparent-fill foreground-stroke" fill="none" {STROKE} x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}"/>"#
            )),
        }
    }
    out.push_str("</svg>");
    out
}

/// Every icon: its name, and its drawing.
fn drawings() -> Vec<(&'static str, String)> {
    let folder = |extra: &'static [Part]| {
        let mut parts = vec![Line(FOLDER)];
        parts.extend(extra.iter().map(|p| match p {
            Line(d) => Line(d),
            Solid(d) => Solid(d),
            Faint(d) => Faint(d),
            Circle(a, b, c) => Circle(*a, *b, *c),
            Rect(a, b, c, d, e) => Rect(*a, *b, *c, *d, *e),
        }));
        svg(&parts)
    };
    let mut all = vec![
        // The places and the files.
        ("computer-symbolic", svg(&[Rect(3.0, 4.0, 18.0, 12.0, 2.0), Line("M8 20h8M12 16v4")])),
        ("user-desktop-symbolic", svg(&[Rect(3.0, 4.0, 18.0, 14.0, 2.0), Line("M3 8.5h18M8.5 21h7")])),
        ("folder-documents-symbolic", folder(&[Line("M9 12.5h6M9 16h4")])),
        ("folder-download-symbolic", folder(&[Line("M12 11v6M9.5 14.5 12 17l2.5-2.5")])),
        ("folder-pictures-symbolic", folder(&[Line("M8 17.2l3-3.7 2.4 2.7 1.6-1.7 2 2.7z")])),
        ("folder-videos-symbolic", folder(&[Line("M10.5 11.5v5.2l4.4-2.6z")])),
        ("drive-removable-media-symbolic", svg(&[Rect(7.0, 8.0, 10.0, 13.0, 2.0), Line("M9.5 8V3h5v5M10.5 13h.01M13.5 13h.01")])),
        ("network-server-symbolic", svg(&[Rect(3.5, 4.0, 17.0, 6.5, 2.0), Rect(3.5, 13.5, 17.0, 6.5, 2.0), Line("M7.5 7.25h.01M7.5 16.75h.01")])),
        ("media-eject-symbolic", svg(&[Line("M12 5 5.5 13h13z"), Line("M5.5 18h13")])),
        ("user-trash-full-symbolic", svg(&[Line("M4 7h16M9 7V4h6v3"), Solid("M6 7l1 13h10l1-13z")])),
        // Actions.
        ("document-edit-symbolic", svg(&[Line("M13 20h8M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4z")])),
        ("document-properties-symbolic", svg(&[Line("M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8zM14 3v5h5M9 13h6M9 17h4")])),
        ("edit-select-all-symbolic", svg(&[Rect(3.5, 3.5, 17.0, 17.0, 2.5), Line("m8 12 3 3 5-6")])),
        ("system-run-symbolic", svg(&[Line("M7 5l12 7-12 7z")])),
        ("process-stop-symbolic", svg(&[Circle(12.0, 12.0, 9.0), Line("M9 9l6 6M15 9l-6 6")])),
        ("view-columns-symbolic", svg(&[Rect(3.5, 4.5, 17.0, 15.0, 2.0), Line("M9.2 4.5v15M14.8 4.5v15")])),
        ("view-app-grid-symbolic", svg(&[Rect(4.0, 4.0, 6.0, 6.0, 1.5), Rect(14.0, 4.0, 6.0, 6.0, 1.5), Rect(4.0, 14.0, 6.0, 6.0, 1.5), Rect(14.0, 14.0, 6.0, 6.0, 1.5)])),
        // The cloud services, by the mark each is known by, drawn as plain outlines.
        ("nodalix-cloud-drive-symbolic", svg(&[Line("M8.5 3.5h7l6 10.5-3.5 6h-12L2.5 14z"), Line("M15.5 3.5 9 14.5h12.5M9 14.5 6 20")])),
        ("nodalix-cloud-onedrive-symbolic", svg(&[Line("M7 18.5a4.5 4.5 0 0 1-.6-8.95A6 6 0 0 1 18 10.5a4 4 0 0 1-.5 8z")])),
        ("nodalix-cloud-dropbox-symbolic", svg(&[Line("M7 4l5 3.25L7 10.5 2 7.25zM17 4l5 3.25-5 3.25-5-3.25zM7 10.5l5 3.25L7 17 2 13.75zM17 10.5l5 3.25L17 17l-5-3.25zM7 18.5l5 3 5-3")])),
        ("nodalix-cloud-icloud-symbolic", svg(&[Line("M7 18.5a4.5 4.5 0 0 1-.6-8.95A6 6 0 0 1 18 10.5a4 4 0 0 1-.5 8z"), Line("M12 16.5v-5M9.7 13.7l2.3-2.3 2.3 2.3")])),
        // Settings.
        ("applications-graphics-symbolic", svg(&[Line("M12 21.5a9.5 9.5 0 1 1 9.5-9.5c0 3-2 4-4.5 4h-2a2 2 0 0 0-1.4 3.4 2 2 0 0 1-1.6 2.1z"), Line("M7.5 11.5h.01M10.5 7.5h.01M15.5 8h.01")])),
        ("preferences-color-symbolic", svg(&[Line("M12 3s6.5 6.4 6.5 11a6.5 6.5 0 0 1-13 0C5.5 9.4 12 3 12 3z")])),
        ("battery-symbolic", svg(&[Rect(2.5, 7.0, 16.0, 10.0, 2.0), Line("M21.5 11v2M6.5 10.5v3")])),
        ("bluetooth-symbolic", svg(&[Line("m7 7 10 10-5 5V2l5 5L7 17")])),
        ("goa-panel-symbolic", svg(&[Line("M17.5 19H9a7 7 0 1 1 6.7-9h1.8a4.5 4.5 0 1 1 0 9z")])),
        ("input-keyboard-symbolic", svg(&[Rect(2.5, 6.0, 19.0, 12.0, 2.0), Line("M6.5 10h.01M10 10h.01M14 10h.01M17.5 10h.01M7.5 14h9")])),
        ("input-mouse-symbolic", svg(&[Rect(5.5, 2.5, 13.0, 19.0, 6.5), Line("M12 7v3.5")])),
        ("network-transmit-receive-symbolic", svg(&[Line("M8 20V4M4 8l4-4 4 4M16 4v16M12 16l4 4 4-4")])),
        ("network-workgroup-symbolic", svg(&[Circle(12.0, 12.0, 9.0), Line("M3 12h18M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18")])),
        ("preferences-desktop-accessibility-symbolic", svg(&[Circle(12.0, 12.0, 9.0), Line("M7.5 10.2 12 11l4.5-.8M12 11v3M9.8 17.5 12 14l2.2 3.5M12 7.5h.01")])),
        ("preferences-desktop-locale-symbolic", svg(&[Line("m5 8 6 6M4 14l6-6 2-3M2 5h12M7 2h1M22 22l-5-10-5 10M14 18h6")])),
        ("preferences-system-notifications-symbolic", svg(&[Line("M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9M10.3 21a1.9 1.9 0 0 0 3.4 0")])),
        ("preferences-system-privacy-symbolic", svg(&[Rect(4.5, 10.5, 15.0, 10.0, 2.0), Line("M8 10.5V7a4 4 0 0 1 8 0v3.5")])),
        ("preferences-system-sharing-symbolic", svg(&[Circle(6.0, 12.0, 2.5), Circle(18.0, 6.0, 2.5), Circle(18.0, 18.0, 2.5), Line("M8.2 10.8l7.6-3.6M8.2 13.2l7.6 3.6")])),
        ("software-update-available-symbolic", svg(&[Line("M12 3v12M7 11l5 5 5-5M5 20h14")])),
    ];
    all.extend(crate::icons_settings::drawings());
    all
}

fn folder() -> PathBuf {
    glib::user_cache_dir().join("nodalix").join("ui-icons")
}

/// Adds the icons to NDL's theme. Call after NDL is initialised.
pub(crate) fn install() {
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };
    let root = folder();
    let dir = root.join(THEME).join("scalable").join("actions");
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    for (name, drawing) in drawings() {
        let file = dir.join(format!("{name}.svg"));
        if std::fs::read_to_string(&file).ok().as_deref() != Some(drawing.as_str()) {
            let _ = std::fs::write(&file, drawing);
        }
    }
    gtk::IconTheme::for_display(&display).add_search_path(&root);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_is_symbolic_and_named_once() {
        let drawings = drawings();
        let mut names: Vec<_> = drawings.iter().map(|(n, _)| *n).collect();
        assert!(names.iter().all(|n| n.ends_with("-symbolic")));
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len());
    }

    #[test]
    fn none_is_one_ndl_already_draws() {
        for (name, _) in drawings() {
            assert!(!ndl::has_icon(name), "{name} is already NDL's");
        }
    }

    #[test]
    fn the_drawings_are_svg() {
        for (name, drawing) in drawings() {
            assert!(
                drawing.starts_with("<svg") && drawing.ends_with("</svg>"),
                "{name}"
            );
            assert!(!drawing.contains("NaN"), "{name}");
        }
    }
}
