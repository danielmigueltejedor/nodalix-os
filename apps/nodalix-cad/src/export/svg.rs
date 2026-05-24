use crate::document::{Document, Entity};
use std::{fmt::Write, fs, path::Path};

pub fn export(document: &Document, path: &Path) -> Result<(), String> {
    let mut out = String::new();
    out.push_str(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-250 -250 500 500">"#);
    out.push_str(r##"<rect x="-250" y="-250" width="500" height="500" fill="#08070d"/>"##);
    for entity in &document.entities {
        match entity {
            Entity::Line { start, end, .. } => {
                write!(
                    out,
                    r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#cba6f7" stroke-width="1"/>"##,
                    start.x, -start.y, end.x, -end.y
                )
                .map_err(|err| err.to_string())?;
            }
            Entity::Circle { center, radius, .. } => {
                write!(
                    out,
                    r##"<circle cx="{}" cy="{}" r="{}" fill="none" stroke="#cba6f7" stroke-width="1"/>"##,
                    center.x, -center.y, radius
                )
                .map_err(|err| err.to_string())?;
            }
            Entity::Polyline { points, closed, .. } => {
                let pts = points
                    .iter()
                    .map(|p| format!("{},{}", p.x, -p.y))
                    .collect::<Vec<_>>()
                    .join(" ");
                let tag = if *closed { "polygon" } else { "polyline" };
                write!(
                    out,
                    r##"<{} points="{}" fill="none" stroke="#cba6f7" stroke-width="1"/>"##,
                    tag, pts
                )
                .map_err(|err| err.to_string())?;
            }
            _ => {}
        }
    }
    out.push_str("</svg>");
    fs::write(path, out).map_err(|err| err.to_string())
}
