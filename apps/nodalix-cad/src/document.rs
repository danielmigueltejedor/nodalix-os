use crate::{
    geometry::{BoundingBox3, Point},
    mesh::mesh::MeshTransform,
    units::Unit,
};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::BTreeMap, fs, path::Path};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Document {
    pub version: u32,
    pub name: String,
    pub units: Unit,
    pub modified: bool,
    pub layers: Vec<Layer>,
    #[serde(default = "default_active_layer_name")]
    pub active_layer_name: String,
    pub entities: Vec<Entity>,
    pub imported_references: Vec<ImportedReference>,
    pub mesh_references: Vec<MeshReference>,
    pub views: Vec<DrawingView>,
    pub sheets: Vec<DrawingSheet>,
    pub metadata: DocumentMetadata,
    #[serde(default)]
    pub entity_colors: BTreeMap<u64, String>,
    #[serde(default)]
    pub entity_line_weights: BTreeMap<u64, f64>,
    #[serde(default)]
    pub entity_line_types: BTreeMap<u64, String>,
    #[serde(default = "default_active_layout")]
    pub active_layout: String,
    #[serde(default = "default_layouts")]
    pub layouts: Vec<Layout>,
    #[serde(default)]
    pub entity_layouts: BTreeMap<u64, String>,
    #[serde(default)]
    pub layout_viewports: Vec<LayoutViewport>,
    #[serde(default)]
    pub block_definitions: BTreeMap<String, BlockDefinition>,
    #[serde(skip, default)]
    entity_bounds_cache: RefCell<BTreeMap<u64, (Point, Point)>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BlockDefinition {
    pub name: String,
    pub base_point: Point,
    pub entities: Vec<Entity>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Layer {
    pub name: String,
    #[serde(default = "default_layer_color")]
    pub color: String,
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default = "default_layer_linetype")]
    pub line_type: String,
    #[serde(default = "default_layer_lineweight")]
    pub line_weight: f64,
    #[serde(default = "default_layer_printable")]
    pub printable: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Layout {
    pub name: String,
    pub kind: LayoutKind,
    pub paper_background: bool,
    #[serde(default)]
    pub paper: PaperSetup,
    #[serde(default)]
    pub page_setup: PageSetup,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum LayoutKind {
    Model,
    Paper,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PaperSetup {
    pub width: f64,
    pub height: f64,
    pub unit: String,
    pub orientation: String,
    pub preset: String,
}

impl Default for PaperSetup {
    fn default() -> Self {
        Self {
            width: 297.0,
            height: 210.0,
            unit: "mm".to_string(),
            orientation: "landscape".to_string(),
            preset: "A4".to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PageSetup {
    pub margin_top: f64,
    pub margin_right: f64,
    pub margin_bottom: f64,
    pub margin_left: f64,
    pub export_scale: f64,
}

impl Default for PageSetup {
    fn default() -> Self {
        Self {
            margin_top: 10.0,
            margin_right: 10.0,
            margin_bottom: 10.0,
            margin_left: 10.0,
            export_scale: 1.0,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LayoutViewport {
    pub id: u64,
    pub layout: String,
    pub center: Point,
    pub width: f64,
    pub height: f64,
    pub view_center: Point,
    pub view_height: f64,
    #[serde(default = "default_viewport_zoom")]
    pub model_zoom: f64,
    #[serde(default = "default_scale_paper_units")]
    pub scale_paper_units: f64,
    #[serde(default = "default_scale_model_units")]
    pub scale_model_units: f64,
    pub twist: f64,
    #[serde(default = "default_viewport_visible")]
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default = "default_viewport_border_visible")]
    pub border_visible: bool,
    #[serde(default)]
    pub visible_layers: Vec<String>,
    #[serde(default)]
    pub hidden_layers: Vec<String>,
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
        #[serde(default)]
        solid: bool,
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
        #[serde(default)]
        scale_y: Option<f64>,
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
            active_layer_name: "Default".to_string(),
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
            entity_line_weights: BTreeMap::new(),
            entity_line_types: BTreeMap::new(),
            active_layout: "Model".to_string(),
            layouts: default_layouts(),
            entity_layouts: BTreeMap::new(),
            layout_viewports: Vec::new(),
            block_definitions: BTreeMap::new(),
            entity_bounds_cache: RefCell::new(BTreeMap::new()),
        }
    }

    pub fn block_definition_key(name: &str) -> String {
        name.trim().to_ascii_uppercase()
    }

    pub fn block_definition(&self, name: &str) -> Option<&BlockDefinition> {
        self.block_definitions
            .get(&Self::block_definition_key(name))
    }

    pub fn insert_block_definition(&mut self, definition: BlockDefinition) {
        let key = Self::block_definition_key(&definition.name);
        self.block_definitions.insert(key, definition);
        self.modified = true;
    }

    pub fn remove_block_definition(&mut self, name: &str) -> Option<BlockDefinition> {
        let removed = self
            .block_definitions
            .remove(&Self::block_definition_key(name));
        if removed.is_some() {
            self.modified = true;
        }
        removed
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
        let layout = normalized_layout_name(layout.unwrap_or(&self.active_layout));
        if !layout.is_empty() {
            self.ensure_layout(&layout);
            if layout != "Model" {
                self.entity_layouts.insert(id, layout);
            }
        }
        self.entities.push(entity);
        self.invalidate_entity_bounds(id);
        self.modified = true;
    }

    pub fn invalidate_entity_bounds(&self, id: u64) {
        self.entity_bounds_cache.borrow_mut().remove(&id);
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
            paper: PaperSetup::default(),
            page_setup: PageSetup::default(),
        });
    }

    pub fn set_layout_paper(&mut self, name: &str, paper: PaperSetup) {
        let name = normalized_layout_name(name);
        self.ensure_layout(&name);
        if let Some(layout) = self.layouts.iter_mut().find(|layout| layout.name == name) {
            layout.paper = paper;
            layout.paper_background = layout.kind == LayoutKind::Paper;
        }
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
                    paper: PaperSetup::default(),
                    page_setup: PageSetup::default(),
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

    pub fn delete_layout(&mut self, name: &str) -> bool {
        let name = normalized_layout_name(name);
        if name == "Model" || !self.layouts.iter().any(|layout| layout.name == name) {
            return false;
        }
        self.layouts.retain(|layout| layout.name != name);
        let ids_to_remove = self
            .entity_layouts
            .iter()
            .filter_map(|(id, layout)| (layout == &name).then_some(*id))
            .collect::<Vec<_>>();
        for id in ids_to_remove {
            self.remove_entity(id);
        }
        self.layout_viewports
            .retain(|viewport| viewport.layout != name);
        if self.active_layout == name {
            self.active_layout = "Model".to_string();
        }
        self.modified = true;
        true
    }

    pub fn duplicate_layout(&mut self, name: &str) -> Option<String> {
        let name = normalized_layout_name(name);
        if name == "Model" || !self.layouts.iter().any(|layout| layout.name == name) {
            return None;
        }
        let mut index = 2usize;
        let new_name = loop {
            let candidate = format!("{name} copy {index}");
            if !self.layouts.iter().any(|layout| layout.name == candidate) {
                break candidate;
            }
            index += 1;
        };
        let source_layout = self
            .layouts
            .iter()
            .find(|layout| layout.name == name)
            .cloned();
        self.layouts.push(Layout {
            name: new_name.clone(),
            kind: LayoutKind::Paper,
            paper_background: true,
            paper: source_layout
                .as_ref()
                .map(|layout| layout.paper.clone())
                .unwrap_or_default(),
            page_setup: source_layout
                .as_ref()
                .map(|layout| layout.page_setup.clone())
                .unwrap_or_default(),
        });
        let entities = self
            .entities
            .iter()
            .filter(|entity| self.entity_layout(entity.id()) == name)
            .cloned()
            .collect::<Vec<_>>();
        for entity in entities {
            let mut copy = entity.clone();
            let old_id = copy.id();
            let new_id = self.next_id();
            copy.set_id(new_id);
            self.entities.push(copy);
            self.entity_layouts.insert(new_id, new_name.clone());
            if let Some(color) = self.entity_colors.get(&old_id).cloned() {
                self.entity_colors.insert(new_id, color);
            }
            if let Some(weight) = self.entity_line_weights.get(&old_id).copied() {
                self.entity_line_weights.insert(new_id, weight);
            }
            if let Some(line_type) = self.entity_line_types.get(&old_id).cloned() {
                self.entity_line_types.insert(new_id, line_type);
            }
            self.entity_bounds_cache.borrow_mut().remove(&new_id);
        }
        let mut viewports = self
            .layout_viewports
            .iter()
            .filter(|viewport| viewport.layout == name)
            .cloned()
            .collect::<Vec<_>>();
        for viewport in &mut viewports {
            viewport.id = self.next_id();
            viewport.layout = new_name.clone();
        }
        self.layout_viewports.extend(viewports);
        self.active_layout = new_name.clone();
        self.modified = true;
        Some(new_name)
    }

    pub fn create_viewport_for_active_layout(&mut self) -> Option<u64> {
        if self.active_layout_kind() != LayoutKind::Paper {
            return None;
        }
        let layout = self.active_layout_ref()?.clone();
        let id = self.next_id();
        let margin_x = layout.page_setup.margin_left + layout.page_setup.margin_right;
        let margin_y = layout.page_setup.margin_top + layout.page_setup.margin_bottom;
        let width = (layout.paper.width - margin_x).max(layout.paper.width * 0.65);
        let height = (layout.paper.height - margin_y).max(layout.paper.height * 0.65);
        let center = Point {
            x: layout.paper.width * 0.5,
            y: layout.paper.height * 0.5,
        };
        let (view_center, view_height) = self
            .model_bounds()
            .map(|(min, max)| {
                (
                    Point {
                        x: (min.x + max.x) * 0.5,
                        y: (min.y + max.y) * 0.5,
                    },
                    (max.y - min.y).abs().max(1.0),
                )
            })
            .unwrap_or((Point::default(), height.max(1.0)));
        self.layout_viewports.push(LayoutViewport {
            id,
            layout: self.active_layout.clone(),
            center,
            width,
            height,
            view_center,
            view_height,
            model_zoom: height / view_height.max(1.0),
            scale_paper_units: 1.0,
            scale_model_units: (view_height / height.max(1.0)).max(1.0),
            twist: 0.0,
            visible: true,
            locked: false,
            border_visible: true,
            visible_layers: Vec::new(),
            hidden_layers: Vec::new(),
        });
        self.modified = true;
        Some(id)
    }

    pub fn set_active_layout_viewport_scale(&mut self, model_units: f64) -> bool {
        if self.active_layout_kind() != LayoutKind::Paper || model_units <= 0.0 {
            return false;
        }
        let Some(viewport) = self
            .layout_viewports
            .iter_mut()
            .rev()
            .find(|viewport| viewport.layout == self.active_layout && !viewport.locked)
        else {
            return false;
        };
        viewport.scale_paper_units = 1.0;
        viewport.scale_model_units = model_units;
        viewport.view_height = viewport.height * model_units;
        viewport.model_zoom = viewport.height / viewport.view_height.max(1.0);
        self.modified = true;
        true
    }

    pub fn toggle_active_layout_viewport_lock(&mut self) -> Option<bool> {
        let viewport = self
            .layout_viewports
            .iter_mut()
            .rev()
            .find(|viewport| viewport.layout == self.active_layout)?;
        viewport.locked = !viewport.locked;
        self.modified = true;
        Some(viewport.locked)
    }

    pub fn add_layout_viewport(&mut self, viewport: LayoutViewport) {
        self.ensure_layout(&viewport.layout);
        self.layout_viewports.push(viewport);
        self.modified = true;
    }

    pub fn active_layout_kind(&self) -> LayoutKind {
        self.layouts
            .iter()
            .find(|layout| layout.name == self.active_layout)
            .map(|layout| layout.kind)
            .unwrap_or(LayoutKind::Model)
    }

    pub fn active_layout_ref(&self) -> Option<&Layout> {
        self.layouts
            .iter()
            .find(|layout| layout.name == self.active_layout)
    }

    pub fn entity_layout(&self, id: u64) -> &str {
        self.entity_layouts
            .get(&id)
            .map(String::as_str)
            .unwrap_or("Model")
    }

    pub fn entity_visible_in_active_layout(&self, id: u64) -> bool {
        normalized_layout_name(self.entity_layout(id))
            == normalized_layout_name(&self.active_layout)
            && self.entity_layer_visible(id)
    }

    pub fn is_model_entity(&self, id: u64) -> bool {
        self.entity_layout(id) == "Model"
    }

    pub fn model_bounds(&self) -> Option<(Point, Point)> {
        self.entities
            .iter()
            .filter(|entity| self.is_model_entity(entity.id()))
            .filter_map(|entity| self.cached_entity_bounds(entity))
            .reduce(merge_bounds)
    }

    pub fn translate_entity(&mut self, id: u64, dx: f64, dy: f64) {
        if self.entity_layer_locked(id) {
            return;
        }
        if let Some(entity) = self.entities.iter_mut().find(|entity| entity.id() == id) {
            entity.translate(dx, dy);
            self.entity_bounds_cache.borrow_mut().remove(&id);
            self.modified = true;
        }
    }

    pub fn remove_entity(&mut self, id: u64) -> bool {
        if self.entity_layer_locked(id) {
            return false;
        }
        let original_len = self.entities.len();
        self.entities.retain(|entity| entity.id() != id);
        let removed = self.entities.len() != original_len;
        if removed {
            self.entity_colors.remove(&id);
            self.entity_line_weights.remove(&id);
            self.entity_line_types.remove(&id);
            self.entity_layouts.remove(&id);
            self.entity_bounds_cache.borrow_mut().remove(&id);
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
        self.entity_bounds_cache.borrow_mut().remove(&copy_id);
        if let Some(color) = self.entity_colors.get(&id).cloned() {
            self.entity_colors.insert(copy_id, color);
        }
        if let Some(weight) = self.entity_line_weights.get(&id).copied() {
            self.entity_line_weights.insert(copy_id, weight);
        }
        if let Some(line_type) = self.entity_line_types.get(&id).cloned() {
            self.entity_line_types.insert(copy_id, line_type);
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

    pub fn paste_entities_translated(&mut self, entities: &[Entity], dx: f64, dy: f64) -> Vec<u64> {
        let mut pasted = Vec::with_capacity(entities.len());
        for entity in entities {
            let mut copy = entity.clone();
            let source_id = copy.id();
            let id = self.next_id();
            copy.set_id(id);
            copy.translate(dx, dy);
            self.entities.push(copy);
            self.entity_bounds_cache.borrow_mut().remove(&id);
            if let Some(color) = self.entity_colors.get(&source_id).cloned() {
                self.entity_colors.insert(id, color);
            }
            if let Some(weight) = self.entity_line_weights.get(&source_id).copied() {
                self.entity_line_weights.insert(id, weight);
            }
            if let Some(line_type) = self.entity_line_types.get(&source_id).cloned() {
                self.entity_line_types.insert(id, line_type);
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
        self.entity_colors
            .get(&id)
            .map(String::as_str)
            .or_else(|| self.entity_layer(id).map(|layer| layer.color.as_str()))
    }

    pub fn entity_line_type(&self, id: u64) -> &str {
        if let Some(line_type) = self.entity_line_types.get(&id) {
            return line_type.as_str();
        }
        self.entity_layer(id)
            .map(|layer| layer.line_type.as_str())
            .unwrap_or("Continuous")
    }

    pub fn entity_line_weight(&self, id: u64) -> f64 {
        if let Some(weight) = self.entity_line_weights.get(&id) {
            return *weight;
        }
        self.entity_layer(id)
            .map(|layer| layer.line_weight)
            .unwrap_or(0.25)
    }

    pub fn entity_layer(&self, id: u64) -> Option<&Layer> {
        let entity_layer = self
            .entities
            .iter()
            .find(|entity| entity.id() == id)
            .map(Entity::layer);
        let name = entity_layer?;
        self.layers.iter().find(|layer| layer.name == name)
    }

    pub fn active_layer(&self) -> Option<&Layer> {
        self.layers
            .iter()
            .find(|layer| layer.name == self.active_layer_name)
    }

    pub fn set_active_layer_name(&mut self, layer_name: &str) -> bool {
        if self.layers.iter().any(|layer| layer.name == layer_name) {
            self.active_layer_name = layer_name.to_string();
            self.modified = true;
            return true;
        }
        false
    }

    pub fn ensure_active_layer_exists(&mut self) {
        if self.layers.is_empty() {
            self.layers.push(Layer::new("Default"));
        }
        if !self
            .layers
            .iter()
            .any(|layer| layer.name == self.active_layer_name)
        {
            self.active_layer_name = self
                .layers
                .first()
                .map(|layer| layer.name.clone())
                .unwrap_or_else(|| "Default".to_string());
        }
    }

    pub fn create_layer(&mut self, name: &str) -> bool {
        let name = name.trim();
        if name.is_empty() || self.layers.iter().any(|layer| layer.name == name) {
            return false;
        }
        self.layers.push(Layer::new(name));
        self.modified = true;
        true
    }

    pub fn rename_layer(&mut self, old_name: &str, new_name: &str) -> bool {
        let new_name = new_name.trim();
        if new_name.is_empty() || self.layers.iter().any(|layer| layer.name == new_name) {
            return false;
        }
        let Some(layer) = self.layers.iter_mut().find(|layer| layer.name == old_name) else {
            return false;
        };
        layer.name = new_name.to_string();
        for entity in &mut self.entities {
            if entity.layer() == old_name {
                entity.set_layer(new_name);
            }
        }
        if self.active_layer_name == old_name {
            self.active_layer_name = new_name.to_string();
        }
        self.modified = true;
        true
    }

    pub fn can_delete_layer(&self, layer_name: &str) -> bool {
        self.layers.len() > 1
            && self.layers.iter().any(|layer| layer.name == layer_name)
            && !self
                .entities
                .iter()
                .any(|entity| entity.layer() == layer_name)
    }

    pub fn delete_layer_if_empty(&mut self, layer_name: &str) -> bool {
        if !self.can_delete_layer(layer_name) {
            return false;
        }
        self.layers.retain(|layer| layer.name != layer_name);
        self.ensure_active_layer_exists();
        self.modified = true;
        true
    }

    pub fn set_layer_visible(&mut self, layer_name: &str, visible: bool) -> bool {
        let Some(layer) = self
            .layers
            .iter_mut()
            .find(|layer| layer.name == layer_name)
        else {
            return false;
        };
        if layer.visible == visible {
            return false;
        }
        layer.visible = visible;
        self.modified = true;
        true
    }

    pub fn set_layer_locked(&mut self, layer_name: &str, locked: bool) -> bool {
        let Some(layer) = self
            .layers
            .iter_mut()
            .find(|layer| layer.name == layer_name)
        else {
            return false;
        };
        if layer.locked == locked {
            return false;
        }
        layer.locked = locked;
        self.modified = true;
        true
    }

    pub fn set_layer_color(&mut self, layer_name: &str, color: &str) -> bool {
        let Some(layer) = self
            .layers
            .iter_mut()
            .find(|layer| layer.name == layer_name)
        else {
            return false;
        };
        let normalized = normalize_color_value(color).unwrap_or_else(default_layer_color);
        if layer.color == normalized {
            return false;
        }
        layer.color = normalized;
        self.modified = true;
        true
    }

    pub fn set_layer_line_weight(&mut self, layer_name: &str, line_weight: f64) -> bool {
        if !line_weight.is_finite() || line_weight <= 0.0 {
            return false;
        }
        let Some(layer) = self
            .layers
            .iter_mut()
            .find(|layer| layer.name == layer_name)
        else {
            return false;
        };
        let value = line_weight.max(0.01);
        if (layer.line_weight - value).abs() < f64::EPSILON {
            return false;
        }
        layer.line_weight = value;
        self.modified = true;
        true
    }

    pub fn set_layer_line_type(&mut self, layer_name: &str, line_type: &str) -> bool {
        let Some(layer) = self
            .layers
            .iter_mut()
            .find(|layer| layer.name == layer_name)
        else {
            return false;
        };
        let normalized = normalized_linetype(line_type);
        if layer.line_type == normalized {
            return false;
        }
        layer.line_type = normalized;
        self.modified = true;
        true
    }

    pub fn layer_visible(&self, layer_name: &str) -> bool {
        self.layers
            .iter()
            .find(|layer| layer.name == layer_name)
            .map(|layer| layer.visible)
            .unwrap_or(true)
    }

    pub fn layer_locked(&self, layer_name: &str) -> bool {
        self.layers
            .iter()
            .find(|layer| layer.name == layer_name)
            .map(|layer| layer.locked)
            .unwrap_or(false)
    }

    pub fn entity_layer_visible(&self, id: u64) -> bool {
        self.entities
            .iter()
            .find(|entity| entity.id() == id)
            .map(|entity| self.layer_visible(entity.layer()))
            .unwrap_or(true)
    }

    pub fn entity_layer_locked(&self, id: u64) -> bool {
        self.entities
            .iter()
            .find(|entity| entity.id() == id)
            .map(|entity| self.layer_locked(entity.layer()))
            .unwrap_or(false)
    }

    /// Text for the attribute-bar layer field: active layer when nothing is selected,
    /// the entity layer for a single selection, or `(mixed)` when layers differ.
    pub fn selection_layer_field_text(&self, selected: &[u64]) -> String {
        if selected.is_empty() {
            return self.active_layer_name.clone();
        }
        let layers: Vec<String> = selected
            .iter()
            .filter_map(|id| {
                self.entities
                    .iter()
                    .find(|entity| entity.id() == *id)
                    .map(|entity| entity.layer().to_string())
            })
            .collect();
        if layers.is_empty() {
            return self.active_layer_name.clone();
        }
        let first = &layers[0];
        if layers.iter().all(|layer| layer == first) {
            first.clone()
        } else {
            "(mixed)".to_string()
        }
    }

    pub fn ensure_layer_with_style(
        &mut self,
        name: &str,
        color: &str,
        line_type: &str,
        line_weight: f64,
    ) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        if let Some(layer) = self.layers.iter_mut().find(|layer| layer.name == name) {
            layer.color = normalize_color_value(color).unwrap_or_else(default_layer_color);
            layer.line_type = normalized_linetype(line_type);
            layer.line_weight = line_weight.max(0.01);
        } else {
            self.layers.push(Layer {
                name: name.to_string(),
                color: normalize_color_value(color).unwrap_or_else(default_layer_color),
                visible: true,
                locked: false,
                line_type: normalized_linetype(line_type),
                line_weight: line_weight.max(0.01),
                printable: true,
            });
        }
    }

    pub fn set_entity_color(&mut self, id: u64, color: &str) -> bool {
        if self.entity_layer_locked(id) {
            return false;
        }
        if !self.entities.iter().any(|entity| entity.id() == id) {
            return false;
        }
        let color = normalize_color_value(color);
        if let Some(color) = color {
            self.entity_colors.insert(id, color);
        } else {
            self.entity_colors.remove(&id);
        }
        self.modified = true;
        true
    }

    pub fn cached_entity_bounds(&self, entity: &Entity) -> Option<(Point, Point)> {
        let id = entity.id();
        if let Some(bounds) = self.entity_bounds_cache.borrow().get(&id).copied() {
            return Some(bounds);
        }
        let bounds = self.entity_bounds_for(entity)?;
        self.entity_bounds_cache.borrow_mut().insert(id, bounds);
        Some(bounds)
    }

    pub fn entity_bounds_for(&self, entity: &Entity) -> Option<(Point, Point)> {
        if let Entity::BlockReference {
            name,
            insertion,
            scale,
            scale_y,
            rotation,
            ..
        } = entity
        {
            return crate::cad::geometry::blocks::block_reference_world_bounds(
                self, name, *insertion, *scale, *scale_y, *rotation,
            );
        }
        entity_bounds(entity)
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
        if self.entity_layer_locked(id) {
            return false;
        }
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

    pub fn set_entity_line_weight(&mut self, id: u64, line_weight: f64) -> bool {
        if self.entity_layer_locked(id) {
            return false;
        }
        if !self.entities.iter().any(|entity| entity.id() == id) {
            return false;
        }
        if !line_weight.is_finite() || line_weight <= 0.0 {
            return false;
        }
        self.entity_line_weights.insert(id, line_weight.max(0.01));
        self.modified = true;
        true
    }

    pub fn clear_entity_line_weight(&mut self, id: u64) -> bool {
        if self.entity_layer_locked(id) {
            return false;
        }
        if !self.entities.iter().any(|entity| entity.id() == id) {
            return false;
        }
        let changed = self.entity_line_weights.remove(&id).is_some();
        if changed {
            self.modified = true;
        }
        changed
    }

    pub fn set_entity_line_type(&mut self, id: u64, line_type: &str) -> bool {
        if self.entity_layer_locked(id) {
            return false;
        }
        if !self.entities.iter().any(|entity| entity.id() == id) {
            return false;
        }
        let line_type = normalized_linetype(line_type);
        if line_type.is_empty() {
            return false;
        }
        self.entity_line_types.insert(id, line_type);
        self.modified = true;
        true
    }

    pub fn clear_entity_line_type(&mut self, id: u64) -> bool {
        if self.entity_layer_locked(id) {
            return false;
        }
        if !self.entities.iter().any(|entity| entity.id() == id) {
            return false;
        }
        let changed = self.entity_line_types.remove(&id).is_some();
        if changed {
            self.modified = true;
        }
        changed
    }

    pub fn set_text_entity_text(&mut self, id: u64, value: &str) -> bool {
        if self.entity_layer_locked(id) {
            return false;
        }
        if let Some(Entity::Text { text, .. }) =
            self.entities.iter_mut().find(|entity| entity.id() == id)
        {
            *text = value.to_string();
            self.entity_bounds_cache.borrow_mut().remove(&id);
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
        let mut document: Self = serde_json::from_str(&data).map_err(|err| err.to_string())?;
        document.ensure_active_layer_exists();
        Ok(document)
    }

    pub fn entity_count_on_layer(&self, layer_name: &str) -> usize {
        self.entities
            .iter()
            .filter(|entity| entity.layer() == layer_name)
            .count()
    }

    /// Bridge to the new CAD core (`crate::cad`). Legacy UI/IO continues to use `Document` directly.
    pub fn to_cad_core(&self) -> crate::cad::CADDocument {
        crate::cad::adapter::from_legacy_document(self)
    }

    pub fn from_cad_core(cad: &crate::cad::CADDocument) -> Self {
        crate::cad::adapter::to_legacy_document(cad)
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

fn default_active_layer_name() -> String {
    "Default".to_string()
}

fn default_layouts() -> Vec<Layout> {
    vec![Layout {
        name: "Model".to_string(),
        kind: LayoutKind::Model,
        paper_background: false,
        paper: PaperSetup::default(),
        page_setup: PageSetup::default(),
    }]
}

fn default_viewport_zoom() -> f64 {
    1.0
}

fn default_scale_paper_units() -> f64 {
    1.0
}

fn default_scale_model_units() -> f64 {
    1.0
}

fn default_viewport_border_visible() -> bool {
    true
}

fn default_viewport_visible() -> bool {
    true
}

/// Normalize layout tab names (`Model` vs paper layouts).
pub fn normalized_layout_name(name: &str) -> String {
    crate::cad::layouts::normalize_layout_name(name)
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
            color: default_layer_color(),
            visible: true,
            locked: false,
            line_type: default_layer_linetype(),
            line_weight: default_layer_lineweight(),
            printable: true,
        }
    }
}

fn default_layer_color() -> String {
    "#a3a7b0".to_string()
}

fn default_layer_linetype() -> String {
    "Continuous".to_string()
}

fn default_layer_lineweight() -> f64 {
    0.25
}

fn default_layer_printable() -> bool {
    true
}

fn normalized_linetype(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "dashed" | "dash" | "discontinua" => "Dashed",
        "dotted" | "dot" | "punteada" => "Dotted",
        "center" | "centre" | "eje" => "Center",
        _ => "Continuous",
    }
    .to_string()
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

    pub(crate) fn set_id_for_import(&mut self, new_id: u64) {
        self.set_id(new_id);
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

#[cfg(test)]
mod layer_tests {
    use super::*;
    use crate::cad::history::{
        apply_entity_property_state, apply_layer_state, capture_entity_property_state,
        capture_layer_state, EntityPropertyChange, LegacyHistoryManager,
        LegacyUpdateEntityPropertiesAction, LegacyUpdateLayersAction,
    };
    use crate::cad::selection::legacy_hit::legacy_hit_entity_at;
    use crate::geometry::Point;

    fn sample_line(doc: &mut Document, layer: &str) -> u64 {
        let id = doc.next_id();
        doc.add_entity(Entity::Line {
            id,
            layer: layer.to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        });
        id
    }

    #[test]
    fn create_layer_adds_to_document() {
        let mut doc = Document::new_empty();
        assert!(doc.create_layer("Walls"));
        assert!(doc.layers.iter().any(|layer| layer.name == "Walls"));
    }

    #[test]
    fn rename_layer_updates_entities() {
        let mut doc = Document::new_empty();
        doc.create_layer("Old");
        let id = sample_line(&mut doc, "Old");
        assert!(doc.rename_layer("Old", "New"));
        assert_eq!(
            doc.entities.iter().find(|e| e.id() == id).unwrap().layer(),
            "New"
        );
    }

    #[test]
    fn set_active_layer_name() {
        let mut doc = Document::new_empty();
        doc.create_layer("Walls");
        assert!(doc.set_active_layer_name("Walls"));
        assert_eq!(doc.active_layer_name, "Walls");
    }

    #[test]
    fn cannot_delete_layer_with_entities() {
        let mut doc = Document::new_empty();
        doc.create_layer("Used");
        sample_line(&mut doc, "Used");
        assert!(!doc.can_delete_layer("Used"));
        assert!(!doc.delete_layer_if_empty("Used"));
    }

    #[test]
    fn delete_empty_layer() {
        let mut doc = Document::new_empty();
        doc.create_layer("Empty");
        assert!(doc.can_delete_layer("Empty"));
        assert!(doc.delete_layer_if_empty("Empty"));
        assert!(!doc.layers.iter().any(|layer| layer.name == "Empty"));
    }

    #[test]
    fn cannot_delete_only_layer() {
        let mut doc = Document::new_empty();
        doc.layers.retain(|layer| layer.name == "Default");
        assert!(!doc.can_delete_layer("Default"));
    }

    #[test]
    fn ensure_active_layer_exists_recovers_invalid_name() {
        let mut doc = Document::new_empty();
        doc.active_layer_name = "Missing".to_string();
        doc.ensure_active_layer_exists();
        assert!(doc
            .layers
            .iter()
            .any(|layer| layer.name == doc.active_layer_name));
    }

    #[test]
    fn new_entity_keeps_explicit_layer() {
        let mut doc = Document::new_empty();
        doc.set_active_layer_name("Walls");
        let id = sample_line(&mut doc, "Walls");
        assert_eq!(
            doc.entities.iter().find(|e| e.id() == id).unwrap().layer(),
            "Walls"
        );
    }

    #[test]
    fn save_load_preserves_layouts_and_viewports() {
        let mut doc = Document::new_empty();
        doc.ensure_layout("Layout1");
        doc.add_entity_on_layout(
            Entity::Line {
                id: doc.next_id(),
                layer: "Default".to_string(),
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 1.0, y: 0.0 },
            },
            Some("Layout1"),
        );
        doc.add_layout_viewport(LayoutViewport {
            id: 99,
            layout: "Layout1".to_string(),
            center: Point { x: 50.0, y: 50.0 },
            width: 100.0,
            height: 80.0,
            view_center: Point::default(),
            view_height: 100.0,
            model_zoom: 1.0,
            scale_paper_units: 1.0,
            scale_model_units: 1.0,
            twist: 0.0,
            visible: true,
            locked: false,
            border_visible: true,
            visible_layers: Vec::new(),
            hidden_layers: Vec::new(),
        });
        let path = std::env::temp_dir().join("lixcad_layout_test.nodcad");
        doc.save_nodcad(&path).expect("save");
        let loaded = Document::open_nodcad(&path).expect("load");
        assert!(loaded.layouts.iter().any(|l| l.name == "Layout1"));
        assert_eq!(loaded.layout_viewports.len(), 1);
        assert_eq!(loaded.layout_viewports[0].layout, "Layout1");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn hidden_layer_not_visible_in_active_layout() {
        let mut doc = Document::new_empty();
        doc.create_layer("Hidden");
        doc.set_layer_visible("Hidden", false);
        let id = sample_line(&mut doc, "Hidden");
        assert!(!doc.entity_visible_in_active_layout(id));
    }

    #[test]
    fn hidden_layer_excluded_from_hit_test() {
        let mut doc = Document::new_empty();
        doc.create_layer("Hidden");
        doc.set_layer_visible("Hidden", false);
        sample_line(&mut doc, "Hidden");
        let hit = legacy_hit_entity_at(&doc, Point { x: 5.0, y: 0.0 }, 1.0);
        assert_eq!(hit, None);
    }

    #[test]
    fn locked_layer_blocks_remove() {
        let mut doc = Document::new_empty();
        doc.create_layer("Locked");
        doc.set_layer_locked("Locked", true);
        let id = sample_line(&mut doc, "Locked");
        assert!(!doc.remove_entity(id));
        assert_eq!(doc.entities.len(), 1);
    }

    #[test]
    fn locked_layer_blocks_translate() {
        let mut doc = Document::new_empty();
        doc.create_layer("Locked");
        doc.set_layer_locked("Locked", true);
        let id = sample_line(&mut doc, "Locked");
        doc.translate_entity(id, 5.0, 0.0);
        let Entity::Line { start, .. } = doc.entities[0] else {
            panic!("expected line");
        };
        assert!((start.x).abs() < f64::EPSILON);
    }

    #[test]
    fn locked_layer_blocks_set_entity_layer() {
        let mut doc = Document::new_empty();
        doc.create_layer("Locked");
        doc.create_layer("Other");
        doc.set_layer_locked("Locked", true);
        let id = sample_line(&mut doc, "Locked");
        assert!(!doc.set_entity_layer(id, "Other"));
    }

    #[test]
    fn locked_layer_blocks_color_weight_type_and_text() {
        let mut doc = Document::new_empty();
        doc.create_layer("Locked");
        doc.set_layer_locked("Locked", true);
        let id = doc.next_id();
        doc.add_entity(Entity::Text {
            id,
            layer: "Locked".to_string(),
            origin: Point { x: 1.0, y: 2.0 },
            text: "A".to_string(),
            height: 2.5,
            rotation: 0.0,
        });
        assert!(!doc.set_entity_color(id, "#ff0000"));
        assert!(!doc.set_entity_line_weight(id, 0.5));
        assert!(!doc.set_entity_line_type(id, "Dashed"));
        assert!(!doc.set_text_entity_text(id, "B"));
    }

    #[test]
    fn bylayer_color_fallback() {
        let mut doc = Document::new_empty();
        doc.set_layer_color("Default", "#112233");
        let id = sample_line(&mut doc, "Default");
        assert_eq!(doc.entity_color(id), Some("#112233"));
    }

    #[test]
    fn bylayer_line_weight_fallback() {
        let mut doc = Document::new_empty();
        doc.set_layer_line_weight("Default", 0.7);
        let id = sample_line(&mut doc, "Default");
        assert!((doc.entity_line_weight(id) - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn bylayer_line_type_fallback() {
        let mut doc = Document::new_empty();
        doc.set_layer_line_type("Default", "Dashed");
        let id = sample_line(&mut doc, "Default");
        assert_eq!(doc.entity_line_type(id), "Dashed");
    }

    #[test]
    fn entity_override_wins_over_layer() {
        let mut doc = Document::new_empty();
        doc.set_layer_color("Default", "#112233");
        let id = sample_line(&mut doc, "Default");
        doc.set_entity_color(id, "#aabbcc");
        assert_eq!(doc.entity_color(id), Some("#aabbcc"));
    }

    #[test]
    fn layer_create_undo_redo_via_history() {
        let mut doc = Document::new_empty();
        let mut history = LegacyHistoryManager::new();
        let before = capture_layer_state(&doc);
        assert!(doc.create_layer("Walls"));
        let after = capture_layer_state(&doc);
        history.record(Box::new(LegacyUpdateLayersAction { before, after }));
        assert!(history.undo(&mut doc));
        assert!(!doc.layers.iter().any(|layer| layer.name == "Walls"));
        assert!(history.redo(&mut doc));
        assert!(doc.layers.iter().any(|layer| layer.name == "Walls"));
    }

    #[test]
    fn layer_rename_undo_redo() {
        let mut doc = Document::new_empty();
        doc.create_layer("Old");
        let mut history = LegacyHistoryManager::new();
        let before = capture_layer_state(&doc);
        assert!(doc.rename_layer("Old", "New"));
        let after = capture_layer_state(&doc);
        history.record(Box::new(LegacyUpdateLayersAction { before, after }));
        assert!(history.undo(&mut doc));
        assert!(doc.layers.iter().any(|layer| layer.name == "Old"));
        assert!(history.redo(&mut doc));
        assert!(doc.layers.iter().any(|layer| layer.name == "New"));
    }

    #[test]
    fn layer_visible_toggle_undo_redo() {
        let mut doc = Document::new_empty();
        let mut history = LegacyHistoryManager::new();
        let before = capture_layer_state(&doc);
        assert!(doc.set_layer_visible("Default", false));
        let after = capture_layer_state(&doc);
        history.record(Box::new(LegacyUpdateLayersAction { before, after }));
        assert!(!doc.layer_visible("Default"));
        assert!(history.undo(&mut doc));
        assert!(doc.layer_visible("Default"));
        assert!(history.redo(&mut doc));
        assert!(!doc.layer_visible("Default"));
    }

    #[test]
    fn layer_locked_toggle_undo_redo() {
        let mut doc = Document::new_empty();
        let mut history = LegacyHistoryManager::new();
        let before = capture_layer_state(&doc);
        assert!(doc.set_layer_locked("Default", true));
        let after = capture_layer_state(&doc);
        history.record(Box::new(LegacyUpdateLayersAction { before, after }));
        assert!(history.undo(&mut doc));
        assert!(!doc.layer_locked("Default"));
        assert!(history.redo(&mut doc));
        assert!(doc.layer_locked("Default"));
    }

    #[test]
    fn layer_color_change_undo_redo() {
        let mut doc = Document::new_empty();
        let mut history = LegacyHistoryManager::new();
        let before = capture_layer_state(&doc);
        assert!(doc.set_layer_color("Default", "#ff00aa"));
        let after = capture_layer_state(&doc);
        history.record(Box::new(LegacyUpdateLayersAction { before, after }));
        assert!(history.undo(&mut doc));
        assert_ne!(doc.layers[0].color, "#ff00aa");
        assert!(history.redo(&mut doc));
        assert_eq!(doc.layers[0].color, "#ff00aa");
    }

    #[test]
    fn move_entity_layer_undo_redo() {
        let mut doc = Document::new_empty();
        doc.create_layer("Target");
        let id = sample_line(&mut doc, "Default");
        let before = capture_entity_property_state(&doc, id).expect("entity");
        assert!(doc.set_entity_layer(id, "Target"));
        let after = capture_entity_property_state(&doc, id).expect("entity");
        let mut history = LegacyHistoryManager::new();
        history.record(Box::new(LegacyUpdateEntityPropertiesAction {
            changes: vec![EntityPropertyChange {
                entity_id: id,
                before,
                after,
            }],
        }));
        assert!(history.undo(&mut doc));
        assert_eq!(
            doc.entities.iter().find(|e| e.id() == id).unwrap().layer(),
            "Default"
        );
        assert!(history.redo(&mut doc));
        assert_eq!(
            doc.entities.iter().find(|e| e.id() == id).unwrap().layer(),
            "Target"
        );
    }
}
