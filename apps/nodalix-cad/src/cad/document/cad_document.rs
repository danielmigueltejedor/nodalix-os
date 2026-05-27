use crate::cad::entities::CADEntity;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CADDocument {
    pub id: String,
    pub name: String,
    pub version: u32,
    pub units: DrawingUnit,
    pub modified: bool,
    pub model_space: ModelSpace,
    pub layouts: Vec<Layout>,
    pub active_layout_id: String,
    pub layers: Vec<Layer>,
    pub active_layer_id: String,
    pub blocks: Vec<BlockDefinition>,
    pub settings: CADSettings,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum DrawingUnit {
    #[default]
    Millimeters,
    Centimeters,
    Meters,
    Inches,
}

impl DrawingUnit {
    pub fn label(self) -> &'static str {
        match self {
            DrawingUnit::Millimeters => "mm",
            DrawingUnit::Centimeters => "cm",
            DrawingUnit::Meters => "m",
            DrawingUnit::Inches => "inch",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelSpace {
    pub entities: Vec<CADEntity>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub color: String,
    pub visible: bool,
    pub locked: bool,
    pub line_type: String,
    pub line_weight: f64,
    pub printable: bool,
}

impl Layer {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            id: id.into(),
            name: name.clone(),
            color: "#a3a7b0".to_string(),
            visible: true,
            locked: false,
            line_type: "Continuous".to_string(),
            line_weight: 0.25,
            printable: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    pub id: String,
    pub name: String,
    pub kind: LayoutKind,
    pub paper_background: bool,
    pub paper: PaperSetup,
    pub page_setup: PageSetup,
    pub paper_entities: Vec<CADEntity>,
    pub viewports: Vec<LayoutViewport>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LayoutKind {
    Model,
    Paper,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayoutViewport {
    pub id: String,
    pub center: crate::cad::geometry::Point2,
    pub width: f64,
    pub height: f64,
    pub view_center: crate::cad::geometry::Point2,
    pub view_height: f64,
    pub model_zoom: f64,
    pub scale_paper_units: f64,
    pub scale_model_units: f64,
    pub twist: f64,
    pub locked: bool,
    pub border_visible: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BlockDefinition {
    pub name: String,
    pub entities: Vec<CADEntity>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CADSettings {
    pub grid_visible: bool,
    pub snap_enabled: bool,
}

impl Default for CADSettings {
    fn default() -> Self {
        Self {
            grid_visible: true,
            snap_enabled: true,
        }
    }
}

impl CADDocument {
    pub fn new_empty() -> Self {
        let default_layer = Layer::new("layer-default", "Default");
        let model_layout = Layout {
            id: "layout-model".to_string(),
            name: "Model".to_string(),
            kind: LayoutKind::Model,
            paper_background: false,
            paper: PaperSetup::default(),
            page_setup: PageSetup::default(),
            paper_entities: Vec::new(),
            viewports: Vec::new(),
        };
        Self {
            id: uuid_simple(),
            name: "Untitled".to_string(),
            version: super::serialization::CAD_DOCUMENT_FORMAT_VERSION,
            units: DrawingUnit::Millimeters,
            modified: false,
            model_space: ModelSpace::default(),
            layouts: vec![model_layout],
            active_layout_id: "layout-model".to_string(),
            layers: vec![
                default_layer.clone(),
                Layer::new("layer-construction", "Construction"),
                Layer::new("layer-dimensions", "Dimensions"),
            ],
            active_layer_id: default_layer.id,
            blocks: Vec::new(),
            settings: CADSettings::default(),
        }
    }

    pub fn next_entity_id(&self) -> String {
        let max_numeric = self
            .model_space
            .entities
            .iter()
            .chain(self.layouts.iter().flat_map(|l| l.paper_entities.iter()))
            .filter_map(|entity| entity.id().strip_prefix("entity-"))
            .filter_map(|suffix| suffix.parse::<u64>().ok())
            .max()
            .unwrap_or(0);
        format!("entity-{}", max_numeric + 1)
    }

    pub fn layer_by_id(&self, layer_id: &str) -> Option<&Layer> {
        self.layers.iter().find(|layer| layer.id == layer_id)
    }

    pub fn layer_by_name(&self, name: &str) -> Option<&Layer> {
        self.layers.iter().find(|layer| layer.name == name)
    }

    pub fn active_layout(&self) -> Option<&Layout> {
        self.layouts
            .iter()
            .find(|layout| layout.id == self.active_layout_id)
    }

    pub fn entities_in_active_space(&self) -> &[CADEntity] {
        if self.active_layout_id == "layout-model" {
            &self.model_space.entities
        } else if let Some(layout) = self.active_layout() {
            &layout.paper_entities
        } else {
            &[]
        }
    }

    pub fn entities_in_active_space_mut(&mut self) -> &mut Vec<CADEntity> {
        if self.active_layout_id == "layout-model" {
            &mut self.model_space.entities
        } else {
            let active_id = self.active_layout_id.clone();
            self.layouts
                .iter_mut()
                .find(|layout| layout.id == active_id)
                .map(|layout| &mut layout.paper_entities)
                .unwrap_or(&mut self.model_space.entities)
        }
    }

    pub fn add_entity(&mut self, entity: CADEntity) {
        self.entities_in_active_space_mut().push(entity);
        self.modified = true;
    }

    pub fn remove_entity(&mut self, id: &str) -> bool {
        let space = self.entities_in_active_space_mut();
        let before = space.len();
        space.retain(|entity| entity.id() != id);
        let removed = space.len() != before;
        if removed {
            self.modified = true;
        }
        removed
    }

    pub fn find_entity(&self, id: &str) -> Option<&CADEntity> {
        self.model_space
            .entities
            .iter()
            .chain(self.layouts.iter().flat_map(|l| l.paper_entities.iter()))
            .find(|entity| entity.id() == id)
    }
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("doc-{nanos}")
}
