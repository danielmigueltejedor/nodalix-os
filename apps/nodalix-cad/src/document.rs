use crate::{
    geometry::{BoundingBox3, Point},
    mesh::mesh::MeshTransform,
    units::Unit,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Document {
    pub version: u32,
    pub name: String,
    pub units: Unit,
    pub modified: bool,
    pub layers: Vec<Layer>,
    pub entities: Vec<Entity>,
    pub imported_references: Vec<ImportedReference>,
    pub mesh_references: Vec<MeshReference>,
    pub views: Vec<DrawingView>,
    pub sheets: Vec<DrawingSheet>,
    pub metadata: DocumentMetadata,
    #[serde(default)]
    pub entity_colors: BTreeMap<u64, String>,
    #[serde(default = "default_active_layout")]
    pub active_layout: String,
    #[serde(default = "default_layouts")]
    pub layouts: Vec<Layout>,
    #[serde(default)]
    pub entity_layouts: BTreeMap<u64, String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Layout {
    pub name: String,
    pub kind: LayoutKind,
    pub paper_background: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum LayoutKind {
    Model,
    Paper,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Entity {
    Point {
        id: u64,
        layer: String,
        point: Point,
    },
    Line {
        id: u64,
        layer: String,
        start: Point,
        end: Point,
    },
    Polyline {
        id: u64,
        layer: String,
        points: Vec<Point>,
        closed: bool,
    },
    Spline {
        id: u64,
        layer: String,
        control_points: Vec<Point>,
        closed: bool,
    },
    Circle {
        id: u64,
        layer: String,
        center: Point,
        radius: f64,
    },
    Text {
        id: u64,
        layer: String,
        origin: Point,
        text: String,
        height: f64,
        rotation: f64,
    },
    Dimension {
        id: u64,
        layer: String,
        start: Point,
        end: Point,
        label: String,
        style: String,
        precision: u8,
    },
    Hatch {
        id: u64,
        layer: String,
        boundary: Vec<Point>,
        pattern: String,
        scale: f64,
        angle: f64,
    },
    Table {
        id: u64,
        layer: String,
        origin: Point,
        rows: u32,
        columns: u32,
        cell_width: f64,
        cell_height: f64,
    },
    BlockReference {
        id: u64,
        layer: String,
        name: String,
        insertion: Point,
        scale: f64,
        rotation: f64,
    },
    Guideline {
        id: u64,
        layer: String,
        start: Point,
        end: Point,
        construction: bool,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ImportedReference {
    pub path: String,
    pub format: String,
    pub summary: String,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MeshReference {
    pub path: String,
    pub format: String,
    pub triangle_count: usize,
    pub bounding_box: Option<BoundingBox3>,
    pub transform: MeshTransform,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DrawingView {
    pub name: String,
    pub view_type: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DrawingSheet {
    pub name: String,
    pub format: String,
    pub orientation: String,
    pub projection: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DocumentMetadata {
    pub scale_factor: f64,
    pub measured_distance: Option<f64>,
    pub real_distance: Option<f64>,
}

impl Document {
    pub fn new_empty() -> Self {
        Self {
            version: 1,
            name: "Untitled".to_string(),
            units: Unit::Millimeters,
            modified: false,
            layers: vec![
                Layer::new("Default"),
                Layer::new("Construction"),
                Layer::new("Dimensions"),
                Layer::new("Mesh Reference"),
            ],
            entities: Vec::new(),
            imported_references: Vec::new(),
            mesh_references: Vec::new(),
            views: vec![
                DrawingView::new("Front", "front"),
                DrawingView::new("Top", "top"),
                DrawingView::new("Right", "right"),
                DrawingView::new("Isometric", "isometric-placeholder"),
            ],
            sheets: vec![DrawingSheet {
                name: "A4 Drawing".to_string(),
                format: "A4".to_string(),
                orientation: "landscape".to_string(),
                projection: "first-angle".to_string(),
            }],
            metadata: DocumentMetadata {
                scale_factor: 1.0,
                measured_distance: None,
                real_distance: None,
            },
            entity_colors: BTreeMap::new(),
            active_layout: "Model".to_string(),
            layouts: default_layouts(),
            entity_layouts: BTreeMap::new(),
        }
    }

    pub fn next_id(&self) -> u64 {
        self.entities
            .iter()
            .map(Entity::id)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }

    pub fn add_entity(&mut self, entity: Entity) {
        self.add_entity_on_layout(entity, None);
    }

    pub fn add_entity_on_layout(&mut self, entity: Entity, layout: Option<&str>) {
        let id = entity.id();
        let layout = layout.unwrap_or(&self.active_layout).trim().to_string();
        if !layout.is_empty() {
            self.ensure_layout(&layout);
            if layout != "Model" {
                self.entity_layouts.insert(id, layout);
            }
        }
        self.entities.push(entity);
        self.modified = true;
    }

    pub fn ensure_layout(&mut self, name: &str) {
        let name = normalized_layout_name(name);
        if self.layouts.iter().any(|layout| layout.name == name) {
            return;
        }
        let kind = if name == "Model" {
            LayoutKind::Model
        } else {
            LayoutKind::Paper
        };
        self.layouts.push(Layout {
            name,
            kind,
            paper_background: kind == LayoutKind::Paper,
        });
    }

    pub fn create_paper_layout(&mut self) -> String {
        let mut index = 1usize;
        loop {
            let name = format!("Layout {index}");
            if !self.layouts.iter().any(|layout| layout.name == name) {
                self.layouts.push(Layout {
                    name: name.clone(),
                    kind: LayoutKind::Paper,
                    paper_background: true,
                });
                self.active_layout = name.clone();
                self.modified = true;
                return name;
            }
            index += 1;
        }
    }

    pub fn set_active_layout(&mut self, name: &str) {
        let name = normalized_layout_name(name);
        self.ensure_layout(&name);
        self.active_layout = name;
    }

    pub fn active_layout_kind(&self) -> LayoutKind {
        self.layouts
            .iter()
            .find(|layout| layout.name == self.active_layout)
            .map(|layout| layout.kind)
            .unwrap_or(LayoutKind::Model)
    }

    pub fn entity_layout(&self, id: u64) -> &str {
        self.entity_layouts
            .get(&id)
            .map(String::as_str)
            .unwrap_or("Model")
    }

    pub fn entity_visible_in_active_layout(&self, id: u64) -> bool {
        self.entity_layout(id) == self.active_layout
    }

    pub fn translate_entity(&mut self, id: u64, dx: f64, dy: f64) {
        if let Some(entity) = self.entities.iter_mut().find(|entity| entity.id() == id) {
            entity.translate(dx, dy);
            self.modified = true;
        }
    }

    pub fn remove_entity(&mut self, id: u64) -> bool {
        let original_len = self.entities.len();
        self.entities.retain(|entity| entity.id() != id);
        let removed = self.entities.len() != original_len;
        if removed {
            self.entity_colors.remove(&id);
            self.entity_layouts.remove(&id);
            self.modified = true;
        }
        removed
    }

    pub fn duplicate_entity(&mut self, id: u64) -> Option<u64> {
        let mut copy = self
            .entities
            .iter()
            .find(|entity| entity.id() == id)
            .cloned()?;
        let copy_id = self.next_id();
        copy.set_id(copy_id);
        copy.translate(10.0, 10.0);
        self.entities.push(copy);
        if let Some(color) = self.entity_colors.get(&id).cloned() {
            self.entity_colors.insert(copy_id, color);
        }
        if let Some(layout) = self.entity_layouts.get(&id).cloned() {
            self.entity_layouts.insert(copy_id, layout);
        }
        self.modified = true;
        Some(copy_id)
    }

    pub fn entities_by_ids(&self, ids: &[u64]) -> Vec<Entity> {
        ids.iter()
            .filter_map(|id| self.entities.iter().find(|entity| entity.id() == *id))
            .cloned()
            .collect()
    }

    pub fn paste_entities(&mut self, entities: &[Entity]) -> Vec<u64> {
        self.paste_entities_translated(entities, 10.0, 10.0)
    }

    pub fn paste_entities_at(&mut self, entities: &[Entity], target: Point) -> Vec<u64> {
        let Some((min, max)) = entity_group_bounds(entities) else {
            return self.paste_entities(entities);
        };
        let center = Point {
            x: (min.x + max.x) * 0.5,
            y: (min.y + max.y) * 0.5,
        };
        self.paste_entities_translated(entities, target.x - center.x, target.y - center.y)
    }

    fn paste_entities_translated(&mut self, entities: &[Entity], dx: f64, dy: f64) -> Vec<u64> {
        let mut pasted = Vec::with_capacity(entities.len());
        for entity in entities {
            let mut copy = entity.clone();
            let source_id = copy.id();
            let id = self.next_id();
            copy.set_id(id);
            copy.translate(dx, dy);
            self.entities.push(copy);
            if let Some(color) = self.entity_colors.get(&source_id).cloned() {
                self.entity_colors.insert(id, color);
            }
            if self.active_layout != "Model" {
                self.entity_layouts.insert(id, self.active_layout.clone());
            }
            pasted.push(id);
        }
        if !pasted.is_empty() {
            self.modified = true;
        }
        pasted
    }

    pub fn entity_color(&self, id: u64) -> Option<&str> {
        self.entity_colors.get(&id).map(String::as_str)
    }

    pub fn set_entity_color(&mut self, id: u64, color: &str) -> bool {
        if !self.entities.iter().any(|entity| entity.id() == id) {
            return false;
        }
        let color = normalize_color_value(color);
        if color.is_none() {
            self.entity_colors.remove(&id);
        } else {
            self.entity_colors.insert(id, color.unwrap());
        }
        self.modified = true;
        true
    }

    pub fn entity_summary(&self, id: u64) -> Option<String> {
        self.entities
            .iter()
            .find(|entity| entity.id() == id)
            .map(|entity| {
                format!(
                    "{} #{} · layer {}",
                    entity.kind(),
                    entity.id(),
                    entity.layer()
                )
            })
    }

    pub fn set_entity_layer(&mut self, id: u64, layer: &str) -> bool {
        if !self.layers.iter().any(|existing| existing.name == layer) {
            self.layers.push(Layer::new(layer));
        }
        if let Some(entity) = self.entities.iter_mut().find(|entity| entity.id() == id) {
            entity.set_layer(layer);
            self.modified = true;
            return true;
        }
        false
    }

    pub fn save_nodcad(&mut self, path: &Path) -> Result<(), String> {
        let data = serde_json::to_string_pretty(self).map_err(|err| err.to_string())?;
        fs::write(path, data).map_err(|err| err.to_string())?;
        self.modified = false;
        Ok(())
    }

    pub fn open_nodcad(path: &Path) -> Result<Self, String> {
        let data = fs::read_to_string(path).map_err(|err| err.to_string())?;
        serde_json::from_str(&data).map_err(|err| err.to_string())
    }
}

fn normalize_color_value(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("default") {
        return None;
    }
    if let Some((r, g, b)) = parse_color_channels(value) {
        return Some(format!("#{r:02x}{g:02x}{b:02x}"));
    }
    Some(value.to_string())
}

fn default_active_layout() -> String {
    "Model".to_string()
}

fn default_layouts() -> Vec<Layout> {
    vec![Layout {
        name: "Model".to_string(),
        kind: LayoutKind::Model,
        paper_background: false,
    }]
}

fn normalized_layout_name(name: &str) -> String {
    let name = name.trim();
    if name.is_empty() || name.eq_ignore_ascii_case("model") {
        "Model".to_string()
    } else {
        name.to_string()
    }
}

fn parse_color_channels(value: &str) -> Option<(u8, u8, u8)> {
    let value = value.trim();
    let hex = value.strip_prefix('#').unwrap_or(value);
    if hex.len() == 6 && hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        return Some((r, g, b));
    }

    let rgb = value
        .strip_prefix("rgb(")
        .and_then(|inner| inner.strip_suffix(')'))
        .unwrap_or(value);
    let parts: Vec<&str> = rgb
        .split(|ch: char| ch == ',' || ch.is_ascii_whitespace())
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() != 3 {
        return None;
    }
    Some((
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ))
}

fn entity_group_bounds(entities: &[Entity]) -> Option<(Point, Point)> {
    entities
        .iter()
        .filter_map(entity_bounds)
        .reduce(merge_bounds)
}

fn entity_bounds(entity: &Entity) -> Option<(Point, Point)> {
    match entity {
        Entity::Point { point, .. }
        | Entity::Text { origin: point, .. }
        | Entity::BlockReference {
            insertion: point, ..
        } => Some((*point, *point)),
        Entity::Line { start, end, .. }
        | Entity::Dimension { start, end, .. }
        | Entity::Guideline { start, end, .. } => Some(points_bounds([*start, *end])),
        Entity::Polyline { points, .. } => points_iter_bounds(points.iter().copied()),
        Entity::Spline { control_points, .. } => points_iter_bounds(control_points.iter().copied()),
        Entity::Hatch { boundary, .. } => points_iter_bounds(boundary.iter().copied()),
        Entity::Circle { center, radius, .. } => {
            let radius = radius.abs();
            Some((
                Point {
                    x: center.x - radius,
                    y: center.y - radius,
                },
                Point {
                    x: center.x + radius,
                    y: center.y + radius,
                },
            ))
        }
        Entity::Table {
            origin,
            rows,
            columns,
            cell_width,
            cell_height,
            ..
        } => Some(points_bounds([
            *origin,
            Point {
                x: origin.x + *columns as f64 * *cell_width,
                y: origin.y + *rows as f64 * *cell_height,
            },
        ])),
    }
}

fn points_bounds<const N: usize>(points: [Point; N]) -> (Point, Point) {
    let mut min = points[0];
    let mut max = points[0];
    for point in points.into_iter().skip(1) {
        min.x = min.x.min(point.x);
        min.y = min.y.min(point.y);
        max.x = max.x.max(point.x);
        max.y = max.y.max(point.y);
    }
    (min, max)
}

fn points_iter_bounds(mut points: impl Iterator<Item = Point>) -> Option<(Point, Point)> {
    let first = points.next()?;
    let mut min = first;
    let mut max = first;
    for point in points {
        min.x = min.x.min(point.x);
        min.y = min.y.min(point.y);
        max.x = max.x.max(point.x);
        max.y = max.y.max(point.y);
    }
    Some((min, max))
}

fn merge_bounds(first: (Point, Point), second: (Point, Point)) -> (Point, Point) {
    (
        Point {
            x: first.0.x.min(second.0.x),
            y: first.0.y.min(second.0.y),
        },
        Point {
            x: first.1.x.max(second.1.x),
            y: first.1.y.max(second.1.y),
        },
    )
}

impl Layer {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            visible: true,
        }
    }
}

impl Entity {
    pub fn id(&self) -> u64 {
        match self {
            Entity::Point { id, .. }
            | Entity::Line { id, .. }
            | Entity::Polyline { id, .. }
            | Entity::Spline { id, .. }
            | Entity::Circle { id, .. }
            | Entity::Text { id, .. }
            | Entity::Dimension { id, .. }
            | Entity::Hatch { id, .. }
            | Entity::Table { id, .. }
            | Entity::BlockReference { id, .. }
            | Entity::Guideline { id, .. } => *id,
        }
    }

    pub fn layer(&self) -> &str {
        match self {
            Entity::Point { layer, .. }
            | Entity::Line { layer, .. }
            | Entity::Polyline { layer, .. }
            | Entity::Spline { layer, .. }
            | Entity::Circle { layer, .. }
            | Entity::Text { layer, .. }
            | Entity::Dimension { layer, .. }
            | Entity::Hatch { layer, .. }
            | Entity::Table { layer, .. }
            | Entity::BlockReference { layer, .. }
            | Entity::Guideline { layer, .. } => layer,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Entity::Point { .. } => "Point",
            Entity::Line { .. } => "Line",
            Entity::Polyline { .. } => "Polyline",
            Entity::Spline { .. } => "Spline",
            Entity::Circle { .. } => "Circle",
            Entity::Text { .. } => "Text",
            Entity::Dimension { .. } => "Dimension",
            Entity::Hatch { .. } => "Hatch",
            Entity::Table { .. } => "Table",
            Entity::BlockReference { .. } => "Block",
            Entity::Guideline { .. } => "Guideline",
        }
    }

    pub fn translate(&mut self, dx: f64, dy: f64) {
        let move_point = |point: &mut Point| {
            point.x += dx;
            point.y += dy;
        };
        match self {
            Entity::Point { point, .. } => move_point(point),
            Entity::Line { start, end, .. }
            | Entity::Dimension { start, end, .. }
            | Entity::Guideline { start, end, .. } => {
                move_point(start);
                move_point(end);
            }
            Entity::Polyline { points, .. }
            | Entity::Spline {
                control_points: points,
                ..
            }
            | Entity::Hatch {
                boundary: points, ..
            } => {
                for point in points {
                    move_point(point);
                }
            }
            Entity::Circle { center, .. }
            | Entity::Text { origin: center, .. }
            | Entity::Table { origin: center, .. }
            | Entity::BlockReference {
                insertion: center, ..
            } => move_point(center),
        }
    }

    pub fn set_layer(&mut self, new_layer: &str) {
        match self {
            Entity::Point { layer, .. }
            | Entity::Line { layer, .. }
            | Entity::Polyline { layer, .. }
            | Entity::Spline { layer, .. }
            | Entity::Circle { layer, .. }
            | Entity::Text { layer, .. }
            | Entity::Dimension { layer, .. }
            | Entity::Hatch { layer, .. }
            | Entity::Table { layer, .. }
            | Entity::BlockReference { layer, .. }
            | Entity::Guideline { layer, .. } => *layer = new_layer.to_string(),
        }
    }

    fn set_id(&mut self, new_id: u64) {
        match self {
            Entity::Point { id, .. }
            | Entity::Line { id, .. }
            | Entity::Polyline { id, .. }
            | Entity::Spline { id, .. }
            | Entity::Circle { id, .. }
            | Entity::Text { id, .. }
            | Entity::Dimension { id, .. }
            | Entity::Hatch { id, .. }
            | Entity::Table { id, .. }
            | Entity::BlockReference { id, .. }
            | Entity::Guideline { id, .. } => *id = new_id,
        }
    }
}

impl DrawingView {
    fn new(name: &str, view_type: &str) -> Self {
        Self {
            name: name.to_string(),
            view_type: view_type.to_string(),
        }
    }
}
