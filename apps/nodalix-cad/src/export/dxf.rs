use crate::document::{Document, Entity};
use std::{fmt::Write, fs, path::Path};

pub fn export(document: &Document, path: &Path) -> Result<(), String> {
    let mut out = String::new();
    out.push_str("0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n");
    out.push_str("0\nSECTION\n2\nENTITIES\n");
    for entity in &document.entities {
        match entity {
            Entity::Line { start, end, .. } => {
                write!(
                    out,
                    "0\nLINE\n8\n{}\n10\n{}\n20\n{}\n30\n0\n11\n{}\n21\n{}\n31\n0\n",
                    entity.layer(),
                    start.x,
                    start.y,
                    end.x,
                    end.y
                )
                .map_err(|err| err.to_string())?;
            }
            Entity::Polyline { points, closed, .. } => {
                out.push_str("0\nLWPOLYLINE\n");
                writeln!(
                    out,
                    "8\n{}\n90\n{}\n70\n{}",
                    entity.layer(),
                    points.len(),
                    if *closed { 1 } else { 0 }
                )
                .map_err(|err| err.to_string())?;
                for point in points {
                    writeln!(out, "10\n{}\n20\n{}", point.x, point.y)
                        .map_err(|err| err.to_string())?;
                }
            }
            Entity::Spline {
                control_points,
                closed,
                ..
            } => {
                writeln!(
                    out,
                    "0\nSPLINE\n8\n{}\n70\n{}\n71\n3\n72\n0\n73\n{}\n74\n0",
                    entity.layer(),
                    if *closed { 1 } else { 0 },
                    control_points.len()
                )
                .map_err(|err| err.to_string())?;
                for point in control_points {
                    writeln!(out, "10\n{}\n20\n{}\n30\n0", point.x, point.y)
                        .map_err(|err| err.to_string())?;
                }
            }
            Entity::Circle { center, radius, .. } => {
                writeln!(
                    out,
                    "0\nCIRCLE\n8\n{}\n10\n{}\n20\n{}\n30\n0\n40\n{}",
                    entity.layer(),
                    center.x,
                    center.y,
                    radius
                )
                .map_err(|err| err.to_string())?;
            }
            Entity::Text { origin, text, .. } => {
                writeln!(
                    out,
                    "0\nTEXT\n8\n{}\n10\n{}\n20\n{}\n40\n2.5\n1\n{}",
                    entity.layer(),
                    origin.x,
                    origin.y,
                    text
                )
                .map_err(|err| err.to_string())?;
            }
            Entity::Dimension {
                start, end, label, ..
            } => {
                writeln!(
                    out,
                    "0\nLINE\n8\n{}\n10\n{}\n20\n{}\n30\n0\n11\n{}\n21\n{}\n31\n0\n0\nTEXT\n8\n{}\n10\n{}\n20\n{}\n40\n2.5\n1\n{}",
                    entity.layer(),
                    start.x,
                    start.y,
                    end.x,
                    end.y,
                    entity.layer(),
                    (start.x + end.x) / 2.0,
                    (start.y + end.y) / 2.0,
                    label
                )
                .map_err(|err| err.to_string())?;
            }
            Entity::Point { point, .. } => {
                writeln!(
                    out,
                    "0\nPOINT\n8\n{}\n10\n{}\n20\n{}\n30\n0",
                    entity.layer(),
                    point.x,
                    point.y
                )
                .map_err(|err| err.to_string())?;
            }
            Entity::Hatch { boundary, .. } => {
                out.push_str("0\nLWPOLYLINE\n");
                writeln!(out, "8\n{}\n90\n{}\n70\n1", entity.layer(), boundary.len())
                    .map_err(|err| err.to_string())?;
                for point in boundary {
                    writeln!(out, "10\n{}\n20\n{}", point.x, point.y)
                        .map_err(|err| err.to_string())?;
                }
            }
            Entity::Table {
                origin,
                rows,
                columns,
                cell_width,
                cell_height,
                ..
            } => {
                let width = *columns as f64 * *cell_width;
                let height = *rows as f64 * *cell_height;
                write_rectangle_polyline(
                    &mut out,
                    entity.layer(),
                    origin.x,
                    origin.y,
                    width,
                    height,
                )?;
                for row in 1..*rows {
                    let y = origin.y + *cell_height * row as f64;
                    write_line(&mut out, entity.layer(), origin.x, y, origin.x + width, y)?;
                }
                for column in 1..*columns {
                    let x = origin.x + *cell_width * column as f64;
                    write_line(&mut out, entity.layer(), x, origin.y, x, origin.y + height)?;
                }
            }
            Entity::BlockReference {
                name,
                insertion,
                scale,
                rotation,
                ..
            } => {
                writeln!(
                    out,
                    "0\nINSERT\n8\n{}\n2\n{}\n10\n{}\n20\n{}\n30\n0\n41\n{}\n42\n{}\n43\n{}\n50\n{}",
                    entity.layer(),
                    name,
                    insertion.x,
                    insertion.y,
                    scale,
                    scale,
                    scale,
                    rotation
                )
                .map_err(|err| err.to_string())?;
            }
            Entity::Guideline { start, end, .. } => {
                write_line(&mut out, entity.layer(), start.x, start.y, end.x, end.y)?;
            }
        }
    }
    out.push_str("0\nENDSEC\n0\nEOF\n");
    fs::write(path, out).map_err(|err| err.to_string())
}

fn write_line(
    out: &mut String,
    layer: &str,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
) -> Result<(), String> {
    write!(
        out,
        "0\nLINE\n8\n{}\n10\n{}\n20\n{}\n30\n0\n11\n{}\n21\n{}\n31\n0\n",
        layer, x1, y1, x2, y2
    )
    .map_err(|err| err.to_string())
}

fn write_rectangle_polyline(
    out: &mut String,
    layer: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    writeln!(
        out,
        "0\nLWPOLYLINE\n8\n{}\n90\n4\n70\n1\n10\n{}\n20\n{}\n10\n{}\n20\n{}\n10\n{}\n20\n{}\n10\n{}\n20\n{}",
        layer,
        x,
        y,
        x + width,
        y,
        x + width,
        y + height,
        x,
        y + height
    )
    .map_err(|err| err.to_string())
}
