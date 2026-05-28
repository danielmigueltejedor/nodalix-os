//! HATCH tool: pick boundary, preview, create hatch via history.

use crate::{
    cad::geometry::hatch::{hatch_boundary_from_entity, hatch_is_solid, HatchPatternKind},
    cad::history::{record_entities_added, LegacyHistoryManager},
    document::{Document, Entity},
    geometry::Point,
    tool_parameters::ToolParametersState,
};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug)]
pub struct HatchBoundarySelection {
    pub source_entity_id: u64,
    pub boundary: Vec<Point>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HatchClickResult {
    BoundarySelected,
    Applied,
    NoBoundary,
    HiddenDenied,
    ActiveLayerLocked,
    GeometryFailed,
}

pub fn boundary_candidate(document: &Document, entity_id: u64) -> bool {
    if !document.entity_visible_in_active_layout(entity_id) {
        return false;
    }
    document
        .entities
        .iter()
        .find(|entity| entity.id() == entity_id)
        .and_then(hatch_boundary_from_entity)
        .is_some()
}

pub fn active_layer_allows_hatch(document: &Document) -> bool {
    !document.layer_locked(&document.active_layer_name)
}

pub fn build_hatch_entity(
    document: &Document,
    boundary: Vec<Point>,
    params: &ToolParametersState,
) -> Entity {
    let pattern = params.hatch_pattern.as_str().to_string();
    let solid = params.hatch_pattern == HatchPatternKind::Solid;
    Entity::Hatch {
        id: 0,
        layer: document.active_layer_name.clone(),
        boundary,
        pattern,
        scale: params.hatch_scale,
        angle: params.hatch_angle,
        solid,
    }
}

pub fn build_hatch_preview(
    document: &Document,
    selection: &HatchBoundarySelection,
    params: &ToolParametersState,
) -> Option<Entity> {
    if selection.boundary.len() < 3 {
        return None;
    }
    let mut hatch = build_hatch_entity(document, selection.boundary.clone(), params);
    if let Entity::Hatch { id, .. } = &mut hatch {
        *id = 0;
    }
    Some(hatch)
}

pub fn apply_hatch(
    document: &mut Document,
    history: &mut LegacyHistoryManager,
    boundary: Vec<Point>,
    params: &ToolParametersState,
) -> Option<u64> {
    if !active_layer_allows_hatch(document) || boundary.len() < 3 {
        return None;
    }
    let id = document.next_id();
    let mut entity = build_hatch_entity(document, boundary, params);
    if let Entity::Hatch { id: entity_id, .. } = &mut entity {
        *entity_id = id;
    }
    document.add_entity(entity);
    document.modified = true;
    record_entities_added(history, document, &[id]);
    Some(id)
}

pub fn handle_hatch_click(
    document: &mut Document,
    history: &Rc<RefCell<LegacyHistoryManager>>,
    point: Point,
    tolerance: f64,
    pending_boundary: &Rc<RefCell<Option<HatchBoundarySelection>>>,
    params: &ToolParametersState,
    hit_entity: impl FnOnce(&Document, Point, f64) -> Option<u64>,
) -> HatchClickResult {
    if !active_layer_allows_hatch(document) {
        return HatchClickResult::ActiveLayerLocked;
    }
    let pending = pending_boundary.borrow().clone();
    if let Some(selection) = pending {
        let _ = point;
        let applied = apply_hatch(
            document,
            &mut history.borrow_mut(),
            selection.boundary,
            params,
        );
        *pending_boundary.borrow_mut() = None;
        return if applied.is_some() {
            HatchClickResult::Applied
        } else {
            HatchClickResult::GeometryFailed
        };
    }
    let Some(entity_id) = hit_entity(document, point, tolerance) else {
        return HatchClickResult::NoBoundary;
    };
    if !boundary_candidate(document, entity_id) {
        return HatchClickResult::HiddenDenied;
    }
    let Some(entity) = document.entities.iter().find(|e| e.id() == entity_id) else {
        return HatchClickResult::GeometryFailed;
    };
    let Some(boundary) = hatch_boundary_from_entity(entity) else {
        return HatchClickResult::GeometryFailed;
    };
    *pending_boundary.borrow_mut() = Some(HatchBoundarySelection {
        source_entity_id: entity_id,
        boundary,
    });
    HatchClickResult::BoundarySelected
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HatchCommandOutcome {
    Activate,
}

pub fn try_execute_hatch_command(
    command: &str,
    params: &mut ToolParametersState,
) -> Option<HatchCommandOutcome> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    let Some(cmd) = parts.first().copied() else {
        return None;
    };
    if cmd != "hatch" && cmd != "h" && cmd != "bhatch" {
        return None;
    }
    if parts.len() >= 2 {
        match parts[1].to_ascii_lowercase().as_str() {
            "solid" => params.hatch_pattern = HatchPatternKind::Solid,
            "ansi31" => {
                params.hatch_pattern = HatchPatternKind::Ansi31;
                if let Some(scale) = parts.get(2).and_then(|v| v.parse::<f64>().ok()) {
                    if scale > 0.0 && scale.is_finite() {
                        params.hatch_scale = scale;
                    }
                }
                if let Some(angle) = parts.get(3).and_then(|v| v.parse::<f64>().ok()) {
                    if angle.is_finite() {
                        params.hatch_angle = angle;
                    }
                }
            }
            _ => {}
        }
    }
    Some(HatchCommandOutcome::Activate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::history::LegacyHistoryManager;

    fn closed_rect(doc: &mut Document) -> u64 {
        let id = doc.next_id();
        doc.add_entity(Entity::Polyline {
            id,
            layer: "Default".to_string(),
            points: vec![
                Point { x: 0.0, y: 0.0 },
                Point { x: 10.0, y: 0.0 },
                Point { x: 10.0, y: 5.0 },
                Point { x: 0.0, y: 5.0 },
            ],
            closed: true,
        });
        id
    }

    #[test]
    fn locked_boundary_allowed() {
        let mut doc = Document::new_empty();
        doc.create_layer("Locked");
        doc.set_layer_locked("Locked", true);
        let id = doc.next_id();
        doc.add_entity(Entity::Polyline {
            id,
            layer: "Locked".to_string(),
            points: vec![
                Point { x: 0.0, y: 0.0 },
                Point { x: 5.0, y: 0.0 },
                Point { x: 5.0, y: 5.0 },
            ],
            closed: true,
        });
        assert!(boundary_candidate(&doc, id));
    }

    #[test]
    fn hidden_boundary_not_candidate() {
        let mut doc = Document::new_empty();
        doc.create_layer("Hidden");
        doc.set_layer_visible("Hidden", false);
        let id = doc.next_id();
        doc.add_entity(Entity::Polyline {
            id,
            layer: "Hidden".to_string(),
            points: vec![
                Point { x: 0.0, y: 0.0 },
                Point { x: 5.0, y: 0.0 },
                Point { x: 5.0, y: 5.0 },
            ],
            closed: true,
        });
        assert!(!boundary_candidate(&doc, id));
    }

    #[test]
    fn active_layer_locked_blocks_hatch() {
        let mut doc = Document::new_empty();
        doc.set_layer_locked("Default", true);
        assert!(!active_layer_allows_hatch(&doc));
    }

    #[test]
    fn hatch_creation_uses_active_layer() {
        let mut doc = Document::new_empty();
        doc.create_layer("HatchLayer");
        doc.set_active_layer_name("HatchLayer");
        let mut history = LegacyHistoryManager::new();
        let params = ToolParametersState::default();
        let boundary = vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 1.0, y: 1.0 },
        ];
        let id = apply_hatch(&mut doc, &mut history, boundary, &params).unwrap();
        let entity = doc.entities.iter().find(|e| e.id() == id).unwrap();
        assert_eq!(entity.layer(), "HatchLayer");
    }

    #[test]
    fn hatch_undo_redo() {
        let mut doc = Document::new_empty();
        closed_rect(&mut doc);
        let mut history = LegacyHistoryManager::new();
        let params = ToolParametersState::default();
        let boundary = vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Point { x: 10.0, y: 5.0 },
            Point { x: 0.0, y: 5.0 },
        ];
        apply_hatch(&mut doc, &mut history, boundary, &params).unwrap();
        assert_eq!(doc.entities.len(), 2);
        assert!(history.undo(&mut doc));
        assert_eq!(doc.entities.len(), 1);
        assert!(history.redo(&mut doc));
        assert_eq!(doc.entities.len(), 2);
    }

    #[test]
    fn command_parser_hatch_variants() {
        let mut params = ToolParametersState::default();
        assert_eq!(
            try_execute_hatch_command("hatch", &mut params),
            Some(HatchCommandOutcome::Activate)
        );
        try_execute_hatch_command("hatch solid", &mut params);
        assert_eq!(params.hatch_pattern, HatchPatternKind::Solid);
        try_execute_hatch_command("hatch ansi31 2 30", &mut params);
        assert_eq!(params.hatch_pattern, HatchPatternKind::Ansi31);
        assert!((params.hatch_scale - 2.0).abs() < 1e-9);
        assert!((params.hatch_angle - 30.0).abs() < 1e-9);
    }

    #[test]
    fn preview_hatch_geometry() {
        let mut doc = Document::new_empty();
        let id = closed_rect(&mut doc);
        let entity = doc.entities.iter().find(|e| e.id() == id).unwrap();
        let boundary = hatch_boundary_from_entity(entity).unwrap();
        let preview = build_hatch_preview(
            &doc,
            &HatchBoundarySelection {
                source_entity_id: id,
                boundary,
            },
            &ToolParametersState::default(),
        )
        .unwrap();
        if let Entity::Hatch { boundary, .. } = preview {
            assert_eq!(boundary.len(), 4);
        } else {
            panic!("expected hatch preview");
        }
    }

    #[test]
    fn build_hatch_entity_solid_flag() {
        let doc = Document::new_empty();
        let mut params = ToolParametersState::default();
        params.hatch_pattern = HatchPatternKind::Solid;
        let entity = build_hatch_entity(
            &doc,
            vec![
                Point { x: 0.0, y: 0.0 },
                Point { x: 1.0, y: 0.0 },
                Point { x: 0.0, y: 1.0 },
            ],
            &params,
        );
        if let Entity::Hatch { pattern, solid, .. } = entity {
            assert!(hatch_is_solid(&pattern, solid));
        } else {
            panic!("expected hatch");
        }
    }
}
