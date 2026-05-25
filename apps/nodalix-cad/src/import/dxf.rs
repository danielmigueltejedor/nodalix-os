use crate::{
    document::{Document, Entity, Layer},
    geometry::Point,
};
use std::collections::BTreeMap;
use std::{fs, path::Path};

use super::ImportSummary;

const ARC_SEGMENTS: usize = 48;
const ELLIPSE_SEGMENTS: usize = 72;

pub fn import_dxf(path: &Path, document: &mut Document) -> Result<ImportSummary, String> {
    let data = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let pairs = group_pairs(&data);
    let detected_layouts = detect_layout_names(&pairs);
    let layout_owners = detect_layout_owner_map(&pairs);
    for layout in &detected_layouts {
        document.ensure_layout(&layout);
    }
    let mut imported = 0usize;
    let mut unsupported: BTreeMap<String, usize> = BTreeMap::new();
    let Some((mut i, end)) = section_bounds(&pairs, "ENTITIES") else {
        return Ok(ImportSummary {
            file_name: path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("drawing.dxf")
                .to_string(),
            format: "DXF".to_string(),
            summary: "DXF import completed without an ENTITIES section.".to_string(),
            details: vec![("Imported entities".to_string(), "0".to_string())],
            warnings: vec!["DXF file did not contain an ENTITIES section.".to_string()],
        });
    };
    while i + 1 < end {
        if pairs[i].0 != "0" {
            i += 1;
            continue;
        }
        let entity_type = pairs[i].1.to_ascii_uppercase();
        i += 1;
        let start = i;
        while i + 1 < end && pairs[i].0 != "0" {
            i += 1;
        }
        let chunk = &pairs[start..i];
        match entity_type.as_str() {
            "LINE" => {
                if let Some(entity) = parse_line(chunk, document.next_id()) {
                    add_imported_entity(document, entity, chunk, &layout_owners);
                    imported += 1;
                }
            }
            "LWPOLYLINE" => {
                if let Some(entity) = parse_lwpolyline(chunk, document.next_id()) {
                    add_imported_entity(document, entity, chunk, &layout_owners);
                    imported += 1;
                }
            }
            "POLYLINE" => {
                let mut vertex_chunks = Vec::new();
                while i + 1 < end && pairs[i].0 == "0" && pairs[i].1 == "VERTEX" {
                    i += 1;
                    let vertex_start = i;
                    while i + 1 < end && pairs[i].0 != "0" {
                        i += 1;
                    }
                    vertex_chunks.push(&pairs[vertex_start..i]);
                }
                if i + 1 < end && pairs[i].0 == "0" && pairs[i].1 == "SEQEND" {
                    i += 1;
                }
                if let Some(entity) = parse_polyline(chunk, &vertex_chunks, document.next_id()) {
                    add_imported_entity(document, entity, chunk, &layout_owners);
                    imported += 1;
                }
            }
            "CIRCLE" => {
                if let Some(entity) = parse_circle(chunk, document.next_id()) {
                    add_imported_entity(document, entity, chunk, &layout_owners);
                    imported += 1;
                }
            }
            "ARC" => {
                if let Some(entity) = parse_arc(chunk, document.next_id()) {
                    add_imported_entity(document, entity, chunk, &layout_owners);
                    imported += 1;
                }
            }
            "ELLIPSE" => {
                if let Some(entity) = parse_ellipse(chunk, document.next_id()) {
                    add_imported_entity(document, entity, chunk, &layout_owners);
                    imported += 1;
                }
            }
            "SPLINE" => {
                if let Some(entity) = parse_spline(chunk, document.next_id()) {
                    add_imported_entity(document, entity, chunk, &layout_owners);
                    imported += 1;
                }
            }
            "HATCH" => {
                if let Some(entity) = parse_hatch(chunk, document.next_id()) {
                    add_imported_entity(document, entity, chunk, &layout_owners);
                    imported += 1;
                }
            }
            "DIMENSION" => {
                if let Some(entity) = parse_dimension(chunk, document.next_id()) {
                    add_imported_entity(document, entity, chunk, &layout_owners);
                    imported += 1;
                }
            }
            "TEXT" | "MTEXT" => {
                if let Some(entity) = parse_text(chunk, document.next_id()) {
                    add_imported_entity(document, entity, chunk, &layout_owners);
                    imported += 1;
                }
            }
            "INSERT" => {
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
        "DXF import supports LINE, LWPOLYLINE/POLYLINE, CIRCLE, ARC, ELLIPSE, SPLINE, HATCH boundaries, DIMENSION, TEXT and MTEXT in this iteration."
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
        details: vec![
            ("Imported entities".to_string(), imported.to_string()),
            (
                "Detected layouts".to_string(),
                detected_layouts
                    .iter()
                    .filter(|name| name.as_str() != "Model")
                    .count()
                    .to_string(),
            ),
        ],
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

fn add_imported_entity(
    document: &mut Document,
    entity: Entity,
    pairs: &[(String, String)],
    layout_owners: &BTreeMap<String, String>,
) {
    let id = entity.id();
    let layout = entity_layout_name(document, pairs, layout_owners);
    ensure_layer(document, entity.layer());
    document.add_entity_on_layout(entity, Some(&layout));
    if let Some(color) = dxf_color_value(pairs) {
        document.set_entity_color(id, color);
    }
}

fn section_bounds(pairs: &[(String, String)], section_name: &str) -> Option<(usize, usize)> {
    let mut i = 0usize;
    while i + 3 < pairs.len() {
        if pairs[i].0 == "0"
            && pairs[i].1.eq_ignore_ascii_case("SECTION")
            && pairs[i + 1].0 == "2"
            && pairs[i + 1].1.eq_ignore_ascii_case(section_name)
        {
            let start = i + 2;
            let mut end = start;
            while end + 1 < pairs.len() {
                if pairs[end].0 == "0" && pairs[end].1.eq_ignore_ascii_case("ENDSEC") {
                    break;
                }
                end += 1;
            }
            return Some((start, end));
        }
        i += 1;
    }
    None
}

fn detect_layout_names(pairs: &[(String, String)]) -> Vec<String> {
    let mut names = Vec::new();
    let mut i = 0usize;
    while i + 1 < pairs.len() {
        if pairs[i].0 != "0" || !pairs[i].1.eq_ignore_ascii_case("LAYOUT") {
            i += 1;
            continue;
        }

        i += 1;
        let start = i;
        while i + 1 < pairs.len() && pairs[i].0 != "0" {
            i += 1;
        }

        let chunk = &pairs[start..i];
        let mut in_layout_section = false;
        for (code, value) in chunk {
            if code == "100" && value.eq_ignore_ascii_case("AcDbLayout") {
                in_layout_section = true;
                continue;
            }
            if in_layout_section && (code == "1" || code == "2") {
                let name = value.trim();
                if !name.is_empty() && !names.iter().any(|existing| existing == name) {
                    names.push(name.to_string());
                }
                break;
            }
        }
    }
    names
}

fn detect_layout_owner_map(pairs: &[(String, String)]) -> BTreeMap<String, String> {
    let mut owners = BTreeMap::new();
    let mut i = 0usize;
    while i + 1 < pairs.len() {
        if pairs[i].0 != "0" || !pairs[i].1.eq_ignore_ascii_case("LAYOUT") {
            i += 1;
            continue;
        }

        i += 1;
        let start = i;
        while i + 1 < pairs.len() && pairs[i].0 != "0" {
            i += 1;
        }

        let chunk = &pairs[start..i];
        let mut in_layout_section = false;
        let mut name = None::<String>;
        let mut owner = None::<String>;
        for (code, value) in chunk {
            if code == "100" && value.eq_ignore_ascii_case("AcDbLayout") {
                in_layout_section = true;
                continue;
            }
            if !in_layout_section {
                continue;
            }
            if name.is_none() && (code == "1" || code == "2") {
                let value = value.trim();
                if !value.is_empty() {
                    name = Some(value.to_string());
                }
            } else if owner.is_none() && code == "330" {
                let value = value.trim();
                if !value.is_empty() {
                    owner = Some(value.to_ascii_uppercase());
                }
            }
        }
        if let (Some(owner), Some(name)) = (owner, name) {
            owners.insert(owner, name);
        }
    }
    owners
}

fn entity_layout_name(
    document: &Document,
    pairs: &[(String, String)],
    layout_owners: &BTreeMap<String, String>,
) -> String {
    if let Some(layout) = text_value(pairs, "410") {
        return layout;
    }

    if let Some(owner) = text_value(pairs, "330") {
        if let Some(layout) = layout_owners.get(&owner.to_ascii_uppercase()) {
            return layout.clone();
        }
    }

    if integer_value(pairs, "67") == Some(1) {
        return document
            .layouts
            .iter()
            .find(|layout| layout.name != "Model")
            .map(|layout| layout.name.clone())
            .unwrap_or_else(|| "Layout 1".to_string());
    }

    "Model".to_string()
}

fn dxf_color_value(pairs: &[(String, String)]) -> Option<&'static str> {
    match integer_value(pairs, "62")? {
        1 => Some("#ef4444"),
        2 => Some("#facc15"),
        3 => Some("#22c55e"),
        4 => Some("#38bdf8"),
        5 => Some("#2563eb"),
        6 => Some("#d946ef"),
        7 => Some("#d1d5db"),
        8 | 9 => Some("#6b7280"),
        _ => None,
    }
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

fn parse_arc(pairs: &[(String, String)], id: u64) -> Option<Entity> {
    let layer = text_value(pairs, "8").unwrap_or_else(|| "Default".to_string());
    let center = Point {
        x: number_value(pairs, "10")?,
        y: number_value(pairs, "20")?,
    };
    let radius = number_value(pairs, "40")?;
    let start = number_value(pairs, "50")?.to_radians();
    let mut end = number_value(pairs, "51")?.to_radians();
    while end < start {
        end += std::f64::consts::TAU;
    }
    let sweep = end - start;
    let segments = ((sweep.abs() / std::f64::consts::TAU) * ARC_SEGMENTS as f64)
        .ceil()
        .max(8.0) as usize;
    let points = (0..=segments)
        .map(|index| {
            let t = start + sweep * index as f64 / segments as f64;
            Point {
                x: center.x + radius * t.cos(),
                y: center.y + radius * t.sin(),
            }
        })
        .collect::<Vec<_>>();
    Some(Entity::Polyline {
        id,
        layer,
        points,
        closed: false,
    })
}

fn parse_ellipse(pairs: &[(String, String)], id: u64) -> Option<Entity> {
    let layer = text_value(pairs, "8").unwrap_or_else(|| "Default".to_string());
    let center = Point {
        x: number_value(pairs, "10")?,
        y: number_value(pairs, "20")?,
    };
    let major = Point {
        x: number_value(pairs, "11")?,
        y: number_value(pairs, "21")?,
    };
    let ratio = number_value(pairs, "40").unwrap_or(1.0).abs();
    let start = number_value(pairs, "41").unwrap_or(0.0);
    let mut end = number_value(pairs, "42").unwrap_or(std::f64::consts::TAU);
    while end < start {
        end += std::f64::consts::TAU;
    }
    let sweep = end - start;
    let major_len = (major.x * major.x + major.y * major.y).sqrt();
    if major_len <= f64::EPSILON {
        return None;
    }
    let unit_major = Point {
        x: major.x / major_len,
        y: major.y / major_len,
    };
    let unit_minor = Point {
        x: -unit_major.y,
        y: unit_major.x,
    };
    let segments = ((sweep.abs() / std::f64::consts::TAU) * ELLIPSE_SEGMENTS as f64)
        .ceil()
        .max(12.0) as usize;
    let points = (0..=segments)
        .map(|index| {
            let t = start + sweep * index as f64 / segments as f64;
            let major_offset = major_len * t.cos();
            let minor_offset = major_len * ratio * t.sin();
            Point {
                x: center.x + unit_major.x * major_offset + unit_minor.x * minor_offset,
                y: center.y + unit_major.y * major_offset + unit_minor.y * minor_offset,
            }
        })
        .collect::<Vec<_>>();
    Some(Entity::Polyline {
        id,
        layer,
        points,
        closed: (sweep - std::f64::consts::TAU).abs() < 1e-6,
    })
}

fn parse_spline(pairs: &[(String, String)], id: u64) -> Option<Entity> {
    let layer = text_value(pairs, "8").unwrap_or_else(|| "Default".to_string());
    let points = collect_points_by_codes(pairs, "10", "20");
    let closed = integer_value(pairs, "70")
        .map(|flags| flags & 1 == 1)
        .unwrap_or(false);
    (points.len() >= 2).then_some(Entity::Spline {
        id,
        layer,
        control_points: points,
        closed,
    })
}

fn parse_hatch(pairs: &[(String, String)], id: u64) -> Option<Entity> {
    let layer = text_value(pairs, "8").unwrap_or_else(|| "Default".to_string());
    let points = collect_hatch_boundary_points(pairs);
    (points.len() >= 3).then_some(Entity::Hatch {
        id,
        layer,
        boundary: points,
        pattern: text_value(pairs, "2").unwrap_or_else(|| "SOLID".to_string()),
        scale: number_value(pairs, "41").unwrap_or(1.0),
        angle: number_value(pairs, "52").unwrap_or(0.0),
    })
}

fn parse_dimension(pairs: &[(String, String)], id: u64) -> Option<Entity> {
    let layer = text_value(pairs, "8").unwrap_or_else(|| "Dimensions".to_string());
    let start = Point {
        x: number_value(pairs, "13").or_else(|| number_value(pairs, "10"))?,
        y: number_value(pairs, "23").or_else(|| number_value(pairs, "20"))?,
    };
    let end = Point {
        x: number_value(pairs, "14").or_else(|| number_value(pairs, "11"))?,
        y: number_value(pairs, "24").or_else(|| number_value(pairs, "21"))?,
    };
    Some(Entity::Dimension {
        id,
        layer,
        start,
        end,
        label: text_value(pairs, "1").unwrap_or_else(|| format!("{:.2}", start.distance_to(end))),
        style: text_value(pairs, "3").unwrap_or_else(|| "Imported".to_string()),
        precision: 2,
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

fn collect_points_by_codes(pairs: &[(String, String)], x_code: &str, y_code: &str) -> Vec<Point> {
    let mut points = Vec::new();
    let mut x = None;
    for (code, value) in pairs {
        if code == x_code {
            x = value.parse::<f64>().ok();
        } else if code == y_code {
            if let (Some(x), Ok(y)) = (x.take(), value.parse::<f64>()) {
                points.push(Point { x, y });
            }
        }
    }
    points
}

fn collect_hatch_boundary_points(pairs: &[(String, String)]) -> Vec<Point> {
    let Some(loop_start) = pairs.iter().position(|(code, _)| code == "92") else {
        return Vec::new();
    };
    let loop_end = pairs
        .iter()
        .enumerate()
        .skip(loop_start + 1)
        .find_map(|(index, (code, _))| {
            (code == "92" || code == "97" || code == "98").then_some(index)
        })
        .unwrap_or(pairs.len());
    let loop_pairs = &pairs[loop_start..loop_end];
    let vertex_count = loop_pairs
        .iter()
        .position(|(code, _)| code == "93")
        .and_then(|index| loop_pairs.get(index + 1))
        .and_then(|(_, value)| value.parse::<usize>().ok())
        .unwrap_or(0);

    let mut polyline_points = Vec::new();
    if vertex_count > 0 {
        let after_count = loop_pairs
            .iter()
            .position(|(code, _)| code == "93")
            .map(|index| index + 2)
            .unwrap_or(loop_pairs.len());
        let mut x = None;
        for (code, value) in &loop_pairs[after_count..] {
            match code.as_str() {
                "10" => x = value.parse::<f64>().ok(),
                "20" => {
                    if let (Some(x), Ok(y)) = (x.take(), value.parse::<f64>()) {
                        polyline_points.push(Point { x, y });
                        if polyline_points.len() >= vertex_count {
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
        if polyline_points.len() >= 3 {
            return polyline_points;
        }
    }

    let mut edge_points = Vec::new();
    let mut start_x = None;
    let mut end_x = None;
    for (code, value) in loop_pairs {
        match code.as_str() {
            "10" => start_x = value.parse::<f64>().ok(),
            "20" => {
                if let (Some(x), Ok(y)) = (start_x.take(), value.parse::<f64>()) {
                    edge_points.push(Point { x, y });
                }
            }
            "11" => end_x = value.parse::<f64>().ok(),
            "21" => {
                if let (Some(x), Ok(y)) = (end_x.take(), value.parse::<f64>()) {
                    edge_points.push(Point { x, y });
                }
            }
            _ => {}
        }
    }
    dedupe_consecutive_points(edge_points)
}

fn dedupe_consecutive_points(points: Vec<Point>) -> Vec<Point> {
    let mut deduped = Vec::with_capacity(points.len());
    for point in points {
        if deduped
            .last()
            .map(|last: &Point| last.distance_to(point) > 1e-9)
            .unwrap_or(true)
        {
            deduped.push(point);
        }
    }
    deduped
}

fn group_pairs(data: &str) -> Vec<(String, String)> {
    let mut lines = data.lines();
    let mut pairs = Vec::new();
    while let (Some(code), Some(value)) = (lines.next(), lines.next()) {
        pairs.push((code.trim().to_string(), value.trim().to_string()));
    }
    pairs
}
