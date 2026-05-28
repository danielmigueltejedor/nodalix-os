//! Temporary bridge between `crate::document::Document` (legacy UI/IO) and `CADDocument` (new core).
//!
// TODO(phase-2): route save/open through `CADDocument` with v2 schema while keeping v1 fallback.

use super::cad_document::{
    CADDocument, DrawingUnit, Layer, Layout, LayoutKind, LayoutViewport, ModelSpace, PageSetup,
    PaperSetup,
};
use crate::cad::entities::{
    BaseEntity, BlockReferenceEntity, CADEntity, CircleEntity, DimensionEntity, HatchEntity,
    LineEntity, PolylineEntity, TextEntity,
};
use crate::cad::geometry::Point2;
use crate::document::{
    Document as LegacyDocument, Entity as LegacyEntity, Layer as LegacyLayer,
    Layout as LegacyLayout, LayoutKind as LegacyLayoutKind, LayoutViewport as LegacyViewport,
};
use crate::units::Unit;

pub fn from_legacy_document(legacy: &LegacyDocument) -> CADDocument {
    let layers = legacy
        .layers
        .iter()
        .map(legacy_layer_to_cad)
        .collect::<Vec<_>>();
    let active_layer_id = layers
        .first()
        .map(|layer| layer.id.clone())
        .unwrap_or_else(|| "layer-default".to_string());

    let mut cad = CADDocument {
        id: format!("doc-legacy-{}", legacy.name.replace(' ', "-")),
        name: legacy.name.clone(),
        version: super::serialization::CAD_DOCUMENT_FORMAT_VERSION,
        units: legacy_unit_to_cad(legacy.units),
        modified: legacy.modified,
        model_space: ModelSpace {
            entities: Vec::new(),
        },
        layouts: legacy
            .layouts
            .iter()
            .map(|layout| legacy_layout_to_cad(layout, legacy))
            .collect(),
        active_layout_id: layout_id_from_name(&legacy.active_layout),
        layers,
        active_layer_id,
        blocks: Vec::new(),
        settings: Default::default(),
    };

    for entity in &legacy.entities {
        let Some(cad_entity) = legacy_entity_to_cad(entity) else {
            continue;
        };
        let layout_name = legacy.entity_layout(entity.id());
        if layout_name == "Model" {
            cad.model_space.entities.push(cad_entity);
        } else if let Some(layout) = cad.layouts.iter_mut().find(|l| l.name == layout_name) {
            layout.paper_entities.push(cad_entity);
        } else {
            cad.model_space.entities.push(cad_entity);
        }
    }

    for viewport in &legacy.layout_viewports {
        if let Some(layout) = cad.layouts.iter_mut().find(|l| l.name == viewport.layout) {
            layout.viewports.push(legacy_viewport_to_cad(viewport));
        }
    }

    cad
}

pub fn to_legacy_document(cad: &CADDocument) -> LegacyDocument {
    let mut legacy = LegacyDocument::new_empty();
    legacy.name = cad.name.clone();
    legacy.modified = cad.modified;
    legacy.units = cad_unit_to_legacy(cad.units);
    legacy.layers = cad.layers.iter().map(cad_layer_to_legacy).collect();
    legacy.layouts = cad.layouts.iter().map(cad_layout_to_legacy).collect();
    legacy.active_layout = cad
        .layouts
        .iter()
        .find(|l| l.id == cad.active_layout_id)
        .map(|l| l.name.clone())
        .unwrap_or_else(|| "Model".to_string());

    for entity in &cad.model_space.entities {
        if let Some(legacy_entity) = cad_entity_to_legacy(entity, &cad.layers) {
            legacy.entities.push(legacy_entity);
        }
    }

    for layout in &cad.layouts {
        if layout.kind == LayoutKind::Paper {
            for entity in &layout.paper_entities {
                if let Some(legacy_entity) = cad_entity_to_legacy(entity, &cad.layers) {
                    let id = legacy_entity.id();
                    legacy.entities.push(legacy_entity);
                    legacy.entity_layouts.insert(id, layout.name.clone());
                }
            }
        }
        for viewport in &layout.viewports {
            legacy
                .layout_viewports
                .push(cad_viewport_to_legacy(viewport, &layout.name));
        }
    }

    legacy
}

fn layout_id_from_name(name: &str) -> String {
    if name == "Model" {
        "layout-model".to_string()
    } else {
        format!("layout-{}", slugify(name))
    }
}

fn slugify(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn legacy_layer_to_cad(layer: &LegacyLayer) -> Layer {
    Layer {
        id: format!("layer-{}", slugify(&layer.name)),
        name: layer.name.clone(),
        color: layer.color.clone(),
        visible: layer.visible,
        locked: layer.locked,
        line_type: layer.line_type.clone(),
        line_weight: layer.line_weight,
        printable: layer.printable,
    }
}

fn cad_layer_to_legacy(layer: &Layer) -> LegacyLayer {
    LegacyLayer {
        name: layer.name.clone(),
        color: layer.color.clone(),
        visible: layer.visible,
        locked: layer.locked,
        line_type: layer.line_type.clone(),
        line_weight: layer.line_weight,
        printable: layer.printable,
    }
}

fn legacy_layout_to_cad(layout: &LegacyLayout, _legacy: &LegacyDocument) -> Layout {
    Layout {
        id: layout_id_from_name(&layout.name),
        name: layout.name.clone(),
        kind: match layout.kind {
            LegacyLayoutKind::Model => LayoutKind::Model,
            LegacyLayoutKind::Paper => LayoutKind::Paper,
        },
        paper_background: layout.paper_background,
        paper: PaperSetup {
            width: layout.paper.width,
            height: layout.paper.height,
            unit: layout.paper.unit.clone(),
            orientation: layout.paper.orientation.clone(),
            preset: layout.paper.preset.clone(),
        },
        page_setup: PageSetup {
            margin_top: layout.page_setup.margin_top,
            margin_right: layout.page_setup.margin_right,
            margin_bottom: layout.page_setup.margin_bottom,
            margin_left: layout.page_setup.margin_left,
            export_scale: layout.page_setup.export_scale,
        },
        paper_entities: Vec::new(),
        viewports: Vec::new(),
    }
}

fn cad_layout_to_legacy(layout: &Layout) -> LegacyLayout {
    LegacyLayout {
        name: layout.name.clone(),
        kind: match layout.kind {
            LayoutKind::Model => LegacyLayoutKind::Model,
            LayoutKind::Paper => LegacyLayoutKind::Paper,
        },
        paper_background: layout.paper_background,
        paper: crate::document::PaperSetup {
            width: layout.paper.width,
            height: layout.paper.height,
            unit: layout.paper.unit.clone(),
            orientation: layout.paper.orientation.clone(),
            preset: layout.paper.preset.clone(),
        },
        page_setup: crate::document::PageSetup {
            margin_top: layout.page_setup.margin_top,
            margin_right: layout.page_setup.margin_right,
            margin_bottom: layout.page_setup.margin_bottom,
            margin_left: layout.page_setup.margin_left,
            export_scale: layout.page_setup.export_scale,
        },
    }
}

fn legacy_viewport_to_cad(viewport: &LegacyViewport) -> LayoutViewport {
    LayoutViewport {
        id: format!("viewport-{}", viewport.id),
        center: Point2::new(viewport.center.x, viewport.center.y),
        width: viewport.width,
        height: viewport.height,
        view_center: Point2::new(viewport.view_center.x, viewport.view_center.y),
        view_height: viewport.view_height,
        model_zoom: viewport.model_zoom,
        scale_paper_units: viewport.scale_paper_units,
        scale_model_units: viewport.scale_model_units,
        twist: viewport.twist,
        locked: viewport.locked,
        border_visible: viewport.border_visible,
    }
}

fn cad_viewport_to_legacy(viewport: &LayoutViewport, layout_name: &str) -> LegacyViewport {
    let id = viewport
        .id
        .strip_prefix("viewport-")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    LegacyViewport {
        id,
        layout: layout_name.to_string(),
        center: crate::geometry::Point {
            x: viewport.center.x,
            y: viewport.center.y,
        },
        width: viewport.width,
        height: viewport.height,
        view_center: crate::geometry::Point {
            x: viewport.view_center.x,
            y: viewport.view_center.y,
        },
        view_height: viewport.view_height,
        model_zoom: viewport.model_zoom,
        scale_paper_units: viewport.scale_paper_units,
        scale_model_units: viewport.scale_model_units,
        twist: viewport.twist,
        visible: true,
        locked: viewport.locked,
        border_visible: viewport.border_visible,
        visible_layers: Vec::new(),
        hidden_layers: Vec::new(),
    }
}

fn legacy_entity_to_cad(entity: &LegacyEntity) -> Option<CADEntity> {
    let id = format!("entity-{}", entity.id());
    let layer_id = format!("layer-{}", slugify(entity.layer()));
    let base = BaseEntity::new(id, layer_id);
    match entity {
        LegacyEntity::Line { start, end, .. } => Some(CADEntity::Line(LineEntity::new(
            base,
            Point2::new(start.x, start.y),
            Point2::new(end.x, end.y),
        ))),
        LegacyEntity::Circle { center, radius, .. } => Some(CADEntity::Circle(CircleEntity::new(
            base,
            Point2::new(center.x, center.y),
            *radius,
        ))),
        LegacyEntity::Polyline { points, closed, .. } => {
            Some(CADEntity::Polyline(PolylineEntity::new(
                base,
                points.iter().map(|p| Point2::new(p.x, p.y)).collect(),
                *closed,
            )))
        }
        LegacyEntity::Text {
            origin,
            text,
            height,
            rotation,
            ..
        } => Some(CADEntity::Text(TextEntity::new(
            base,
            Point2::new(origin.x, origin.y),
            text.clone(),
            *height,
            *rotation,
        ))),
        LegacyEntity::Dimension {
            start,
            end,
            label,
            style,
            precision,
            ..
        } => Some(CADEntity::Dimension(DimensionEntity::new(
            base,
            Point2::new(start.x, start.y),
            Point2::new(end.x, end.y),
            label.clone(),
            style.clone(),
            *precision,
        ))),
        LegacyEntity::Hatch {
            boundary,
            pattern,
            scale,
            angle,
            ..
        } => Some(CADEntity::Hatch(HatchEntity::new(
            base,
            boundary.iter().map(|p| Point2::new(p.x, p.y)).collect(),
            pattern.clone(),
            *scale,
            *angle,
        ))),
        LegacyEntity::BlockReference {
            name,
            insertion,
            scale,
            rotation,
            ..
        } => Some(CADEntity::BlockReference(BlockReferenceEntity::new(
            base,
            name.clone(),
            Point2::new(insertion.x, insertion.y),
            *scale,
            *rotation,
        ))),
        // TODO(phase-2): map Point, Spline, Table, Guideline, etc.
        _ => None,
    }
}

fn cad_entity_to_legacy(entity: &CADEntity, layers: &[Layer]) -> Option<LegacyEntity> {
    let id = entity
        .id()
        .strip_prefix("entity-")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let layer = layer_name_for_entity(entity.layer_id(), layers);
    match entity {
        CADEntity::Line(e) => Some(LegacyEntity::Line {
            id,
            layer: layer.clone(),
            start: crate::geometry::Point {
                x: e.start.x,
                y: e.start.y,
            },
            end: crate::geometry::Point {
                x: e.end.x,
                y: e.end.y,
            },
        }),
        CADEntity::Circle(e) => Some(LegacyEntity::Circle {
            id,
            layer: layer.clone(),
            center: crate::geometry::Point {
                x: e.center.x,
                y: e.center.y,
            },
            radius: e.radius,
        }),
        CADEntity::Polyline(e) => Some(LegacyEntity::Polyline {
            id,
            layer: layer.clone(),
            points: e
                .points
                .iter()
                .map(|p| crate::geometry::Point { x: p.x, y: p.y })
                .collect(),
            closed: e.closed,
        }),
        CADEntity::Text(e) => Some(LegacyEntity::Text {
            id,
            layer: layer.clone(),
            origin: crate::geometry::Point {
                x: e.origin.x,
                y: e.origin.y,
            },
            text: e.text.clone(),
            height: e.height,
            rotation: e.rotation,
        }),
        CADEntity::Dimension(e) => Some(LegacyEntity::Dimension {
            id,
            layer: layer.clone(),
            start: crate::geometry::Point {
                x: e.start.x,
                y: e.start.y,
            },
            end: crate::geometry::Point {
                x: e.end.x,
                y: e.end.y,
            },
            label: e.label.clone(),
            style: e.style.clone(),
            precision: e.precision,
        }),
        CADEntity::Hatch(e) => Some(LegacyEntity::Hatch {
            id,
            layer: layer.clone(),
            boundary: e
                .boundary
                .iter()
                .map(|p| crate::geometry::Point { x: p.x, y: p.y })
                .collect(),
            pattern: e.pattern.clone(),
            scale: e.scale,
            angle: e.angle,
            solid: e.pattern.eq_ignore_ascii_case("SOLID"),
        }),
        CADEntity::BlockReference(e) => Some(LegacyEntity::BlockReference {
            id,
            layer: layer.clone(),
            name: e.name.clone(),
            insertion: crate::geometry::Point {
                x: e.insertion.x,
                y: e.insertion.y,
            },
            scale: e.scale,
            rotation: e.rotation,
        }),
    }
}

fn layer_name_for_entity(layer_id: &str, layers: &[Layer]) -> String {
    layers
        .iter()
        .find(|layer| layer.id == layer_id)
        .map(|layer| layer.name.clone())
        .unwrap_or_else(|| {
            layer_id
                .strip_prefix("layer-")
                .unwrap_or("Default")
                .to_string()
        })
}

fn legacy_unit_to_cad(unit: Unit) -> DrawingUnit {
    match unit {
        Unit::Millimeters => DrawingUnit::Millimeters,
        Unit::Centimeters => DrawingUnit::Centimeters,
        Unit::Meters => DrawingUnit::Meters,
        Unit::Inches => DrawingUnit::Inches,
    }
}

fn cad_unit_to_legacy(unit: DrawingUnit) -> Unit {
    match unit {
        DrawingUnit::Millimeters => Unit::Millimeters,
        DrawingUnit::Centimeters => Unit::Centimeters,
        DrawingUnit::Meters => Unit::Meters,
        DrawingUnit::Inches => Unit::Inches,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::geometry::Point2;
    use crate::document::Entity;

    #[test]
    fn roundtrip_line_entity() {
        let mut legacy = LegacyDocument::new_empty();
        legacy.add_entity(Entity::Line {
            id: 1,
            layer: "Default".to_string(),
            start: crate::geometry::Point { x: 0.0, y: 0.0 },
            end: crate::geometry::Point { x: 10.0, y: 0.0 },
        });
        let cad = from_legacy_document(&legacy);
        assert_eq!(cad.model_space.entities.len(), 1);
        let back = to_legacy_document(&cad);
        assert_eq!(back.entities.len(), 1);
    }

    #[test]
    fn cad_circle_roundtrip_geometry() {
        let mut cad = CADDocument::new_empty();
        cad.add_entity(CADEntity::Circle(CircleEntity::new(
            BaseEntity::new("entity-1", "layer-default"),
            Point2::new(5.0, 5.0),
            3.0,
        )));
        let legacy = to_legacy_document(&cad);
        let restored = from_legacy_document(&legacy);
        assert_eq!(restored.model_space.entities.len(), 1);
    }

    #[test]
    fn roundtrip_text_entity() {
        let mut legacy = LegacyDocument::new_empty();
        legacy.add_entity(Entity::Text {
            id: 9,
            layer: "Default".to_string(),
            origin: crate::geometry::Point { x: 1.0, y: 2.0 },
            text: "Label".to_string(),
            height: 2.5,
            rotation: 15.0,
        });
        let cad = from_legacy_document(&legacy);
        assert_eq!(cad.model_space.entities.len(), 1);
        let back = to_legacy_document(&cad);
        assert_eq!(back.entities.len(), 1);
        match &back.entities[0] {
            Entity::Text { text, height, .. } => {
                assert_eq!(text, "Label");
                assert!((*height - 2.5).abs() < f64::EPSILON);
            }
            other => panic!("expected text entity, got {other:?}"),
        }
    }
}
