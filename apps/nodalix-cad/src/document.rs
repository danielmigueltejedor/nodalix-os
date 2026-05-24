use crate::{
    geometry::{BoundingBox3, Point},
    mesh::mesh::MeshTransform,
    units::Unit,
};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

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
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
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
        self.entities.push(entity);
        self.modified = true;
    }

    pub fn translate_entity(&mut self, id: u64, dx: f64, dy: f64) {
        if let Some(entity) = self.entities.iter_mut().find(|entity| entity.id() == id) {
            entity.translate(dx, dy);
            self.modified = true;
        }
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
            | Entity::Circle { layer, .. }
            | Entity::Text { layer, .. }
            | Entity::Dimension { layer, .. }
            | Entity::Hatch { layer, .. }
            | Entity::Table { layer, .. }
            | Entity::BlockReference { layer, .. }
            | Entity::Guideline { layer, .. } => *layer = new_layer.to_string(),
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
