use crate::{
    document::{Document, Entity, Layer},
    geometry::Point,
};
use std::collections::BTreeMap;
use std::{fs, path::Path};

use super::ImportSummary;

pub fn import_dxf(path: &Path, document: &mut Document) -> Result<ImportSummary, String> {
    let data = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let pairs = group_pairs(&data);
    let mut imported = 0usize;
    let mut unsupported: BTreeMap<String, usize> = BTreeMap::new();
    let mut i = 0usize;
    while i + 1 < pairs.len() {
        if pairs[i].0 != "0" {
            i += 1;
            continue;
        }
        let entity_type = pairs[i].1.to_ascii_uppercase();
        i += 1;
        let start = i;
        while i + 1 < pairs.len() && pairs[i].0 != "0" {
            i += 1;
        }
        let chunk = &pairs[start..i];
        match entity_type.as_str() {
            "LINE" => {
                if let Some(entity) = parse_line(chunk, document.next_id()) {
                    ensure_layer(document, entity.layer());
                    document.add_entity(entity);
                    imported += 1;
                }
            }
            "LWPOLYLINE" => {
                if let Some(entity) = parse_lwpolyline(chunk, document.next_id()) {
                    ensure_layer(document, entity.layer());
                    document.add_entity(entity);
                    imported += 1;
                }
            }
            "POLYLINE" => {
                let mut vertex_chunks = Vec::new();
                while i + 1 < pairs.len() && pairs[i].0 == "0" && pairs[i].1 == "VERTEX" {
                    i += 1;
                    let vertex_start = i;
                    while i + 1 < pairs.len() && pairs[i].0 != "0" {
                        i += 1;
                    }
                    vertex_chunks.push(&pairs[vertex_start..i]);
                }
                if i + 1 < pairs.len() && pairs[i].0 == "0" && pairs[i].1 == "SEQEND" {
                    i += 1;
                }
                if let Some(entity) = parse_polyline(chunk, &vertex_chunks, document.next_id()) {
                    ensure_layer(document, entity.layer());
                    document.add_entity(entity);
                    imported += 1;
                }
            }
            "CIRCLE" => {
                if let Some(entity) = parse_circle(chunk, document.next_id()) {
                    ensure_layer(document, entity.layer());
                    document.add_entity(entity);
                    imported += 1;
                }
            }
            "TEXT" | "MTEXT" => {
                if let Some(entity) = parse_text(chunk, document.next_id()) {
                    ensure_layer(document, entity.layer());
                    document.add_entity(entity);
                    imported += 1;
                }
            }
            "ARC" | "INSERT" | "HATCH" | "DIMENSION" | "SPLINE" => {
                *unsupported.entry(entity_type).or_default() += 1;
            }
            _ => {}
        }
    }

    let mut warnings = Vec::new();
    if !unsupported.is_empty() {
        warnings.push(format!(
            "Unsupported DXF entities skipped: {}",
            unsupported
                .iter()
                .map(|(kind, count)| format!("{kind} x{count}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    warnings.push(
        "DXF import supports LINE, LWPOLYLINE/POLYLINE, CIRCLE, TEXT and MTEXT in this iteration."
            .to_string(),
    );

    Ok(ImportSummary {
        file_name: path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("drawing.dxf")
            .to_string(),
        format: "DXF".to_string(),
        summary: "DXF import completed for supported 2D entities.".to_string(),
        details: vec![("Imported entities".to_string(), imported.to_string())],
        warnings,
    })
}

fn parse_line(pairs: &[(String, String)], id: u64) -> Option<Entity> {
    let layer = text_value(pairs, "8").unwrap_or_else(|| "Default".to_string());
    Some(Entity::Line {
        id,
        layer,
        start: Point {
            x: number_value(pairs, "10")?,
            y: number_value(pairs, "20")?,
        },
        end: Point {
            x: number_value(pairs, "11")?,
            y: number_value(pairs, "21")?,
        },
    })
}

fn parse_lwpolyline(pairs: &[(String, String)], id: u64) -> Option<Entity> {
    let layer = text_value(pairs, "8").unwrap_or_else(|| "Default".to_string());
    let closed = integer_value(pairs, "70")
        .map(|flags| flags & 1 == 1)
        .unwrap_or(false);
    let mut points = Vec::new();
    let mut x = None;
    for (code, value) in pairs {
        match code.as_str() {
            "10" => x = value.parse::<f64>().ok(),
            "20" => {
                if let (Some(x), Ok(y)) = (x.take(), value.parse::<f64>()) {
                    points.push(Point { x, y });
                }
            }
            _ => {}
        }
    }
    (points.len() >= 2).then_some(Entity::Polyline {
        id,
        layer,
        points,
        closed,
    })
}

fn parse_polyline(
    pairs: &[(String, String)],
    vertex_chunks: &[&[(String, String)]],
    id: u64,
) -> Option<Entity> {
    let layer = text_value(pairs, "8").unwrap_or_else(|| "Default".to_string());
    let closed = integer_value(pairs, "70")
        .map(|flags| flags & 1 == 1)
        .unwrap_or(false);
    let points = vertex_chunks
        .iter()
        .filter_map(|vertex| {
            Some(Point {
                x: number_value(vertex, "10")?,
                y: number_value(vertex, "20")?,
            })
        })
        .collect::<Vec<_>>();
    (points.len() >= 2).then_some(Entity::Polyline {
        id,
        layer,
        points,
        closed,
    })
}

fn parse_circle(pairs: &[(String, String)], id: u64) -> Option<Entity> {
    let layer = text_value(pairs, "8").unwrap_or_else(|| "Default".to_string());
    Some(Entity::Circle {
        id,
        layer,
        center: Point {
            x: number_value(pairs, "10")?,
            y: number_value(pairs, "20")?,
        },
        radius: number_value(pairs, "40")?,
    })
}

fn parse_text(pairs: &[(String, String)], id: u64) -> Option<Entity> {
    let layer = text_value(pairs, "8").unwrap_or_else(|| "Default".to_string());
    Some(Entity::Text {
        id,
        layer,
        origin: Point {
            x: number_value(pairs, "10")?,
            y: number_value(pairs, "20")?,
        },
        text: text_value(pairs, "1")
            .or_else(|| text_value(pairs, "3"))
            .unwrap_or_default(),
        height: number_value(pairs, "40").unwrap_or(2.5),
        rotation: number_value(pairs, "50").unwrap_or(0.0),
    })
}

fn ensure_layer(document: &mut Document, layer: &str) {
    if !document
        .layers
        .iter()
        .any(|existing| existing.name == layer)
    {
        document.layers.push(Layer::new(layer));
    }
}

fn text_value(pairs: &[(String, String)], code: &str) -> Option<String> {
    pairs
        .iter()
        .find(|(candidate, _)| candidate == code)
        .map(|(_, value)| value.clone())
}

fn number_value(pairs: &[(String, String)], code: &str) -> Option<f64> {
    text_value(pairs, code)?.parse().ok()
}

fn integer_value(pairs: &[(String, String)], code: &str) -> Option<i64> {
    text_value(pairs, code)?.parse().ok()
}

fn group_pairs(data: &str) -> Vec<(String, String)> {
    let mut lines = data.lines();
    let mut pairs = Vec::new();
    while let (Some(code), Some(value)) = (lines.next(), lines.next()) {
        pairs.push((code.trim().to_string(), value.trim().to_string()));
    }
    pairs
}
