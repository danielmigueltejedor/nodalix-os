use crate::{
    cad::layouts::{layout_debug_log, normalize_layout_name},
    document::{
        normalized_layout_name, BlockDefinition as DocumentBlockDefinition, Document, Entity,
        Layer, LayoutViewport, PaperSetup,
    },
    geometry::Point,
};
use std::collections::BTreeMap;
use std::{fs, path::Path};

use super::ImportSummary;

const ARC_SEGMENTS: usize = 48;
const ELLIPSE_SEGMENTS: usize = 72;

#[derive(Clone, Debug)]
struct DxfBlockDefinition {
    base: Point,
    entities: Vec<Entity>,
}

pub fn import_dxf(path: &Path, document: &mut Document) -> Result<ImportSummary, String> {
    let data = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let pairs = group_pairs(&data);
    let detected_layouts = detect_layout_names(&pairs);
    let layout_papers = detect_layout_papers(&pairs);
    let layout_owners = detect_layout_owner_map(&pairs);
    let block_definitions = parse_block_definitions(&pairs);
    for (name, def) in &block_definitions {
        document.insert_block_definition(DocumentBlockDefinition {
            name: name.clone(),
            base_point: def.base,
            entities: def.entities.clone(),
        });
    }
    for layout in &detected_layouts {
        let name = normalized_layout_name(layout);
        document.ensure_layout(&name);
        if let Some(paper) = layout_papers
            .get(layout)
            .or_else(|| layout_papers.get(&name))
            .cloned()
        {
            document.set_layout_paper(&name, paper);
        }
        layout_debug_log(&format!("layout detected name={name}"));
    }
    let paper_layouts: Vec<String> = detected_layouts
        .iter()
        .map(|name| normalized_layout_name(name))
        .filter(|name| name != "Model")
        .collect();
    let single_paper_layout = (paper_layouts.len() == 1).then(|| paper_layouts[0].clone());
    let mut paper_layout_hint = single_paper_layout.clone();
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
                    add_imported_entity(
                        document,
                        entity,
                        chunk,
                        &layout_owners,
                        paper_layout_hint.as_deref(),
                    );
                    imported += 1;
                }
            }
            "LWPOLYLINE" => {
                if let Some(entity) = parse_lwpolyline(chunk, document.next_id()) {
                    add_imported_entity(
                        document,
                        entity,
                        chunk,
                        &layout_owners,
                        paper_layout_hint.as_deref(),
                    );
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
                    add_imported_entity(
                        document,
                        entity,
                        chunk,
                        &layout_owners,
                        paper_layout_hint.as_deref(),
                    );
                    imported += 1;
                }
            }
            "CIRCLE" => {
                if let Some(entity) = parse_circle(chunk, document.next_id()) {
                    add_imported_entity(
                        document,
                        entity,
                        chunk,
                        &layout_owners,
                        paper_layout_hint.as_deref(),
                    );
                    imported += 1;
                }
            }
            "ARC" => {
                if let Some(entity) = parse_arc(chunk, document.next_id()) {
                    add_imported_entity(
                        document,
                        entity,
                        chunk,
                        &layout_owners,
                        paper_layout_hint.as_deref(),
                    );
                    imported += 1;
                }
            }
            "ELLIPSE" => {
                if let Some(entity) = parse_ellipse(chunk, document.next_id()) {
                    add_imported_entity(
                        document,
                        entity,
                        chunk,
                        &layout_owners,
                        paper_layout_hint.as_deref(),
                    );
                    imported += 1;
                }
            }
            "SPLINE" => {
                if let Some(entity) = parse_spline(chunk, document.next_id()) {
                    add_imported_entity(
                        document,
                        entity,
                        chunk,
                        &layout_owners,
                        paper_layout_hint.as_deref(),
                    );
                    imported += 1;
                }
            }
            "HATCH" => {
                if let Some(entity) = parse_hatch(chunk, document.next_id()) {
                    add_imported_entity(
                        document,
                        entity,
                        chunk,
                        &layout_owners,
                        paper_layout_hint.as_deref(),
                    );
                    imported += 1;
                }
            }
            "DIMENSION" => {
                if let Some(entity) = parse_dimension(chunk, document.next_id()) {
                    add_imported_entity(
                        document,
                        entity,
                        chunk,
                        &layout_owners,
                        paper_layout_hint.as_deref(),
                    );
                    imported += 1;
                }
            }
            "VIEWPORT" => {
                if let Some(viewport) = parse_viewport(chunk, document.next_id(), &layout_owners) {
                    paper_layout_hint = Some(viewport.layout.clone());
                    document.add_layout_viewport(viewport);
                    imported += 1;
                }
            }
            "TEXT" | "MTEXT" | "ATTRIB" => {
                if let Some(entity) = parse_text(chunk, document.next_id()) {
                    add_imported_entity(
                        document,
                        entity,
                        chunk,
                        &layout_owners,
                        paper_layout_hint.as_deref(),
                    );
                    imported += 1;
                }
            }
            "INSERT" => {
                if let Some(entity) = parse_insert(chunk, document.next_id()) {
                    add_imported_entity(
                        document,
                        entity,
                        chunk,
                        &layout_owners,
                        paper_layout_hint.as_deref(),
                    );
                    imported += 1;
                } else if let Some(inserted) =
                    expand_insert(chunk, document.next_id(), &block_definitions)
                {
                    for entity in inserted {
                        add_imported_entity(
                            document,
                            entity,
                            chunk,
                            &layout_owners,
                            paper_layout_hint.as_deref(),
                        );
                        imported += 1;
                    }
                } else {
                    *unsupported.entry(entity_type).or_default() += 1;
                }
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
    paper_layout_hint: Option<&str>,
) {
    let id = entity.id();
    let layout = entity_layout_name(pairs, layout_owners, paper_layout_hint);
    ensure_layer(document, entity.layer());
    document.add_entity_on_layout(entity, Some(&layout));
    if layout != "Model" {
        layout_debug_log(&format!(
            "entity assigned layout={layout} paper=true id={id}"
        ));
    }
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

fn parse_block_definitions(pairs: &[(String, String)]) -> BTreeMap<String, DxfBlockDefinition> {
    let mut blocks = BTreeMap::new();
    let Some((mut i, end)) = section_bounds(pairs, "BLOCKS") else {
        return blocks;
    };
    while i + 1 < end {
        if pairs[i].0 != "0" || !pairs[i].1.eq_ignore_ascii_case("BLOCK") {
            i += 1;
            continue;
        }
        i += 1;
        let block_start = i;
        while i + 1 < end && !(pairs[i].0 == "0" && pairs[i].1.eq_ignore_ascii_case("ENDBLK")) {
            i += 1;
        }
        let block_pairs = &pairs[block_start..i];
        let Some(name) = text_value(block_pairs, "2") else {
            i += 1;
            continue;
        };
        let base = Point {
            x: number_value(block_pairs, "10").unwrap_or(0.0),
            y: number_value(block_pairs, "20").unwrap_or(0.0),
        };
        let entities = parse_block_entities(block_pairs);
        if !entities.is_empty() {
            blocks.insert(
                name.to_ascii_uppercase(),
                DxfBlockDefinition { base, entities },
            );
        }
        i += 1;
    }
    blocks
}

fn parse_block_entities(pairs: &[(String, String)]) -> Vec<Entity> {
    let mut entities = Vec::new();
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
            "LINE" => parse_line(chunk, 0)
                .into_iter()
                .for_each(|entity| entities.push(entity)),
            "LWPOLYLINE" => parse_lwpolyline(chunk, 0)
                .into_iter()
                .for_each(|entity| entities.push(entity)),
            "CIRCLE" => parse_circle(chunk, 0)
                .into_iter()
                .for_each(|entity| entities.push(entity)),
            "ARC" => parse_arc(chunk, 0)
                .into_iter()
                .for_each(|entity| entities.push(entity)),
            "ELLIPSE" => parse_ellipse(chunk, 0)
                .into_iter()
                .for_each(|entity| entities.push(entity)),
            "SPLINE" => parse_spline(chunk, 0)
                .into_iter()
                .for_each(|entity| entities.push(entity)),
            "HATCH" => parse_hatch(chunk, 0)
                .into_iter()
                .for_each(|entity| entities.push(entity)),
            "TEXT" | "MTEXT" | "ATTRIB" => parse_text(chunk, 0)
                .into_iter()
                .for_each(|entity| entities.push(entity)),
            _ => {}
        }
    }
    entities
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
        if let Some(name) = layout_tab_name_from_chunk(chunk) {
            let name = normalized_layout_name(&name);
            if !names.iter().any(|existing| existing == &name) {
                names.push(name);
            }
        }
    }
    if !names.iter().any(|name| name == "Model") {
        names.insert(0, "Model".to_string());
    }
    names
}

fn layout_tab_name_from_chunk(chunk: &[(String, String)]) -> Option<String> {
    let mut in_layout_section = false;
    let mut tab_name = None::<String>;
    let mut block_name = None::<String>;
    for (code, value) in chunk {
        if code == "100" && value.eq_ignore_ascii_case("AcDbLayout") {
            in_layout_section = true;
            continue;
        }
        if !in_layout_section {
            continue;
        }
        if code == "1" && tab_name.is_none() {
            let value = value.trim();
            if !value.is_empty() {
                tab_name = Some(value.to_string());
            }
        } else if code == "2" && block_name.is_none() {
            let value = value.trim();
            if !value.is_empty() && !value.starts_with('*') {
                block_name = Some(value.to_string());
            }
        }
    }
    tab_name.or(block_name)
}

fn detect_layout_papers(pairs: &[(String, String)]) -> BTreeMap<String, PaperSetup> {
    let mut papers = BTreeMap::new();
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
        for (code, value) in chunk {
            if code == "100" && value.eq_ignore_ascii_case("AcDbLayout") {
                in_layout_section = true;
                continue;
            }
            if in_layout_section && name.is_none() && (code == "1" || code == "2") {
                let value = value.trim();
                if !value.is_empty() {
                    name = Some(value.to_string());
                }
            }
        }
        let Some(name) = name else {
            continue;
        };
        let width = (number_value(chunk, "11").unwrap_or(297.0)
            - number_value(chunk, "10").unwrap_or(0.0))
        .abs();
        let height = (number_value(chunk, "21").unwrap_or(210.0)
            - number_value(chunk, "20").unwrap_or(0.0))
        .abs();
        if width > 1.0 && height > 1.0 {
            papers.insert(
                name,
                PaperSetup {
                    width,
                    height,
                    unit: "mm".to_string(),
                    orientation: if height > width {
                        "portrait".to_string()
                    } else {
                        "landscape".to_string()
                    },
                    preset: paper_preset(width, height).to_string(),
                },
            );
        }
    }
    papers
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
            owners.insert(owner, normalized_layout_name(&name));
        }
    }
    owners
}

fn paper_preset(width: f64, height: f64) -> &'static str {
    let short = width.min(height).round() as i32;
    let long = width.max(height).round() as i32;
    match (short, long) {
        (210, 297) => "A4",
        (297, 420) => "A3",
        (420, 594) => "A2",
        (594, 841) => "A1",
        (841, 1189) => "A0",
        (216, 279) => "Letter",
        _ => "Custom",
    }
}

fn entity_layout_name(
    pairs: &[(String, String)],
    layout_owners: &BTreeMap<String, String>,
    paper_layout_hint: Option<&str>,
) -> String {
    if let Some(layout) = text_value(pairs, "410") {
        return normalized_layout_name(&layout);
    }

    if let Some(owner) = text_value(pairs, "330") {
        if let Some(layout) = layout_owners.get(&owner.to_ascii_uppercase()) {
            return normalized_layout_name(layout);
        }
    }

    if integer_value(pairs, "67") == Some(1) {
        if let Some(layout) = paper_layout_hint {
            return normalized_layout_name(layout);
        }
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

fn parse_viewport(
    pairs: &[(String, String)],
    id: u64,
    layout_owners: &BTreeMap<String, String>,
) -> Option<LayoutViewport> {
    let viewport_number = integer_value(pairs, "69").unwrap_or(1);
    if viewport_number <= 1 {
        return None;
    }
    let layout = text_value(pairs, "410")
        .or_else(|| {
            text_value(pairs, "330")
                .and_then(|owner| layout_owners.get(&owner.to_ascii_uppercase()).cloned())
        })
        .map(|name| normalized_layout_name(&name))
        .filter(|name| name != "Model")?;
    let center = Point {
        x: number_value(pairs, "10")?,
        y: number_value(pairs, "20")?,
    };
    let width = number_value(pairs, "40").unwrap_or(1.0).abs().max(1.0);
    let height = number_value(pairs, "41").unwrap_or(1.0).abs().max(1.0);
    let view_center = Point {
        x: number_value(pairs, "12").unwrap_or(0.0),
        y: number_value(pairs, "22").unwrap_or(0.0),
    };
    let view_height = number_value(pairs, "45").unwrap_or(height).abs().max(1.0);
    let twist = number_value(pairs, "51").unwrap_or(0.0).to_radians();
    let flags = integer_value(pairs, "90").unwrap_or(0);
    let visible = flags & 512 != 512;
    let locked = flags & 16384 == 16384;
    layout_debug_log(&format!(
        "viewport imported layout={layout} center=({:.3},{:.3}) scale={:.6}",
        center.x,
        center.y,
        height / view_height.max(1.0)
    ));
    Some(LayoutViewport {
        id,
        layout,
        center,
        width,
        height,
        view_center,
        view_height,
        model_zoom: height / view_height.max(1.0),
        scale_paper_units: 1.0,
        scale_model_units: (view_height / height.max(1.0)).max(1.0),
        twist,
        visible,
        locked,
        border_visible: true,
        visible_layers: Vec::new(),
        hidden_layers: Vec::new(),
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
    let pattern = text_value(pairs, "2").unwrap_or_else(|| "SOLID".to_string());
    let solid = pattern.eq_ignore_ascii_case("SOLID");
    (points.len() >= 3).then_some(Entity::Hatch {
        id,
        layer,
        boundary: points,
        pattern,
        scale: number_value(pairs, "41").unwrap_or(1.0),
        angle: number_value(pairs, "52").unwrap_or(0.0),
        solid,
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
        label: text_value(pairs, "1")
            .map(|value| normalize_dxf_text(&value))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| format!("{:.2}", start.distance_to(end))),
        style: text_value(pairs, "3").unwrap_or_else(|| "Imported".to_string()),
        precision: 2,
    })
}

fn parse_text(pairs: &[(String, String)], id: u64) -> Option<Entity> {
    let layer = text_value(pairs, "8").unwrap_or_else(|| "Default".to_string());
    let raw_text = joined_text_value(pairs);
    Some(Entity::Text {
        id,
        layer,
        origin: Point {
            x: number_value(pairs, "10")?,
            y: number_value(pairs, "20")?,
        },
        text: normalize_dxf_text(&raw_text),
        height: number_value(pairs, "40").unwrap_or(2.5),
        rotation: number_value(pairs, "50").unwrap_or(0.0),
    })
}

fn joined_text_value(pairs: &[(String, String)]) -> String {
    let chunks = pairs
        .iter()
        .filter_map(|(code, value)| (code == "1" || code == "3").then_some(value.as_str()))
        .collect::<Vec<_>>();
    if chunks.is_empty() {
        return String::new();
    }
    chunks.join("")
}

fn normalize_dxf_text(raw: &str) -> String {
    let mut output = String::new();
    let chars = raw.chars().collect::<Vec<_>>();
    let mut i = 0usize;

    while i < chars.len() {
        let ch = chars[i];
        if ch == '{' || ch == '}' {
            i += 1;
            continue;
        }
        if ch != '\\' {
            output.push(ch);
            i += 1;
            continue;
        }

        i += 1;
        if i >= chars.len() {
            break;
        }
        let command = chars[i];
        i += 1;
        match command {
            'P' => output.push(' '),
            '~' => output.push(' '),
            '\\' => output.push('\\'),
            '{' => output.push('{'),
            '}' => output.push('}'),
            'L' | 'l' | 'O' | 'o' | 'K' | 'k' => {}
            'S' | 's' => {
                let start = i;
                while i < chars.len() && chars[i] != ';' {
                    i += 1;
                }
                let stacked = chars[start..i]
                    .iter()
                    .collect::<String>()
                    .replace(['#', '^'], "/");
                output.push_str(&stacked);
                if i < chars.len() && chars[i] == ';' {
                    i += 1;
                }
            }
            'A' | 'a' | 'C' | 'c' | 'F' | 'f' | 'H' | 'h' | 'Q' | 'q' | 'T' | 't' | 'W' | 'w'
            | 'p' => {
                while i < chars.len() && chars[i] != ';' {
                    i += 1;
                }
                if i < chars.len() && chars[i] == ';' {
                    i += 1;
                }
            }
            other if other.is_ascii_alphabetic() => {
                while i < chars.len() && chars[i] != ';' {
                    i += 1;
                }
                if i < chars.len() && chars[i] == ';' {
                    i += 1;
                }
            }
            other => output.push(other),
        }
    }

    output
        .replace("%%c", "diameter ")
        .replace("%%C", "diameter ")
        .replace("%%d", "deg")
        .replace("%%D", "deg")
        .replace("%%p", "+/-")
        .replace("%%P", "+/-")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_insert(pairs: &[(String, String)], id: u64) -> Option<Entity> {
    Some(Entity::BlockReference {
        id,
        layer: text_value(pairs, "8").unwrap_or_else(|| "Default".to_string()),
        name: text_value(pairs, "2")?,
        insertion: Point {
            x: number_value(pairs, "10")?,
            y: number_value(pairs, "20")?,
        },
        scale: number_value(pairs, "41").unwrap_or(1.0),
        scale_y: {
            let scale_x = number_value(pairs, "41").unwrap_or(1.0);
            let scale_y = number_value(pairs, "42").unwrap_or(scale_x);
            if (scale_y - scale_x).abs() > f64::EPSILON {
                Some(scale_y)
            } else {
                None
            }
        },
        rotation: number_value(pairs, "50").unwrap_or(0.0).to_radians(),
    })
}

fn expand_insert(
    pairs: &[(String, String)],
    first_id: u64,
    block_definitions: &BTreeMap<String, DxfBlockDefinition>,
) -> Option<Vec<Entity>> {
    let name = text_value(pairs, "2")?;
    let block = block_definitions.get(&name.to_ascii_uppercase())?;
    let insertion = Point {
        x: number_value(pairs, "10").unwrap_or(0.0),
        y: number_value(pairs, "20").unwrap_or(0.0),
    };
    let scale_x = number_value(pairs, "41").unwrap_or(1.0);
    let scale_y = number_value(pairs, "42").unwrap_or(scale_x);
    let rotation = number_value(pairs, "50").unwrap_or(0.0).to_radians();
    let mut next_id = first_id;
    let entities = block
        .entities
        .iter()
        .cloned()
        .map(|mut entity| {
            entity.set_id_for_import(next_id);
            next_id = next_id.saturating_add(1);
            transform_entity_for_insert(
                &mut entity,
                block.base,
                insertion,
                scale_x,
                scale_y,
                rotation,
            );
            entity
        })
        .collect::<Vec<_>>();
    (!entities.is_empty()).then_some(entities)
}

fn transform_entity_for_insert(
    entity: &mut Entity,
    base: Point,
    insertion: Point,
    scale_x: f64,
    scale_y: f64,
    rotation: f64,
) {
    let transform = |point: &mut Point| {
        let x = (point.x - base.x) * scale_x;
        let y = (point.y - base.y) * scale_y;
        let cos = rotation.cos();
        let sin = rotation.sin();
        point.x = insertion.x + x * cos - y * sin;
        point.y = insertion.y + x * sin + y * cos;
    };
    match entity {
        Entity::Point { point, .. } | Entity::Text { origin: point, .. } => transform(point),
        Entity::BlockReference {
            insertion: point, ..
        } => transform(point),
        Entity::Line { start, end, .. }
        | Entity::Dimension { start, end, .. }
        | Entity::Guideline { start, end, .. } => {
            transform(start);
            transform(end);
        }
        Entity::Polyline { points, .. }
        | Entity::Spline {
            control_points: points,
            ..
        } => {
            for point in points {
                transform(point);
            }
        }
        Entity::Hatch { boundary, .. } => {
            for point in boundary {
                transform(point);
            }
        }
        Entity::Circle { center, radius, .. } => {
            transform(center);
            *radius *= scale_x.abs().max(scale_y.abs());
        }
        Entity::Table {
            origin,
            cell_width,
            cell_height,
            ..
        } => {
            transform(origin);
            *cell_width *= scale_x.abs();
            *cell_height *= scale_y.abs();
        }
    }
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

#[cfg(test)]
mod tests {
    use super::{
        entity_layout_name, group_pairs, layout_tab_name_from_chunk, normalize_dxf_text,
        parse_viewport,
    };
    use crate::document::Document;
    use std::collections::BTreeMap;

    #[test]
    fn entity_layout_from_group_410() {
        let pairs = vec![
            ("67".to_string(), "1".to_string()),
            ("410".to_string(), "Layout1".to_string()),
        ];
        assert_eq!(
            entity_layout_name(&pairs, &BTreeMap::new(), None),
            "Layout1"
        );
    }

    #[test]
    fn entity_model_space_without_paper_flag() {
        let pairs = vec![("8".to_string(), "0".to_string())];
        assert_eq!(entity_layout_name(&pairs, &BTreeMap::new(), None), "Model");
    }

    #[test]
    fn paper_entity_uses_layout_hint_when_missing_410() {
        let pairs = vec![("67".to_string(), "1".to_string())];
        assert_eq!(
            entity_layout_name(&pairs, &BTreeMap::new(), Some("Layout2")),
            "Layout2"
        );
    }

    #[test]
    fn viewport_import_skips_paper_space_viewport_number_one() {
        let pairs = vec![
            ("69".to_string(), "1".to_string()),
            ("10".to_string(), "0".to_string()),
            ("20".to_string(), "0".to_string()),
            ("40".to_string(), "100".to_string()),
            ("41".to_string(), "80".to_string()),
            ("410".to_string(), "Layout1".to_string()),
        ];
        assert!(parse_viewport(&pairs, 1, &BTreeMap::new()).is_none());
    }

    #[test]
    fn viewport_import_creates_layout_viewport() {
        let pairs = vec![
            ("69".to_string(), "2".to_string()),
            ("10".to_string(), "50".to_string()),
            ("20".to_string(), "40".to_string()),
            ("40".to_string(), "100".to_string()),
            ("41".to_string(), "80".to_string()),
            ("12".to_string(), "10".to_string()),
            ("22".to_string(), "20".to_string()),
            ("45".to_string(), "200".to_string()),
            ("410".to_string(), "Layout1".to_string()),
        ];
        let viewport = parse_viewport(&pairs, 9, &BTreeMap::new()).unwrap();
        assert_eq!(viewport.layout, "Layout1");
        assert!((viewport.view_height - 200.0).abs() < 1e-6);
        assert!((viewport.model_zoom - 0.4).abs() < 1e-6);
    }

    #[test]
    fn layout_tab_name_prefers_group_one() {
        let chunk = vec![
            ("100".to_string(), "AcDbLayout".to_string()),
            ("1".to_string(), "Layout1".to_string()),
            ("2".to_string(), "Layout1_Paper".to_string()),
        ];
        assert_eq!(
            layout_tab_name_from_chunk(&chunk).as_deref(),
            Some("Layout1")
        );
    }

    #[test]
    fn dxf_import_assigns_layout_from_group_410() {
        let dxf = "\
0\nLINE\n8\n0\n10\n0\n20\n0\n11\n10\n21\n0\n67\n1\n410\nLayout1\n\
0\nLINE\n8\n0\n10\n0\n20\n0\n11\n5\n21\n5\n\
0\nEOF\n";
        let mut doc = Document::new_empty();
        let pairs = group_pairs(dxf);
        let mut owners = BTreeMap::new();
        let layout = entity_layout_name(&pairs[0..12], &owners, None);
        assert_eq!(layout, "Layout1");
        doc.ensure_layout("Layout1");
        doc.add_entity_on_layout(
            crate::document::Entity::Line {
                id: 1,
                layer: "0".to_string(),
                start: crate::geometry::Point { x: 0.0, y: 0.0 },
                end: crate::geometry::Point { x: 10.0, y: 0.0 },
            },
            Some("Layout1"),
        );
        doc.add_entity_on_layout(
            crate::document::Entity::Line {
                id: 2,
                layer: "0".to_string(),
                start: crate::geometry::Point { x: 0.0, y: 0.0 },
                end: crate::geometry::Point { x: 5.0, y: 5.0 },
            },
            None,
        );
        doc.set_active_layout("Layout1");
        assert!(doc.entity_visible_in_active_layout(1));
        assert!(!doc.entity_visible_in_active_layout(2));
        let _ = owners;
    }

    #[test]
    fn normalizes_autocad_mtext_format_codes() {
        assert_eq!(
            normalize_dxf_text(r"\pxqc;{\fISOCPEUR|b1|i1|c0|p34;W1.4;CAD}"),
            "W1.4;CAD"
        );
        assert_eq!(
            normalize_dxf_text(r"\pxqc;Escala\P\pxqc;Fecha"),
            "Escala Fecha"
        );
    }
}
