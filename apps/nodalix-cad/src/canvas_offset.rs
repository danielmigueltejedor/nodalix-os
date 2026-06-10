//! OFFSET tool: pick source entity, side point, preview, commit via history.

use crate::{
    cad::geometry::{entity_supports_offset, offset_entity_geometry},
    cad::history::{record_entities_added, LegacyHistoryManager},
    document::{Document, Entity},
    geometry::Point,
    tool_parameters::ToolParametersState,
};
use std::{cell::RefCell, rc::Rc};

pub fn offset_source_candidate(document: &Document, entity_id: u64) -> bool {
    document.entity_visible_in_active_layout(entity_id)
        && document
            .entities
            .iter()
            .find(|entity| entity.id() == entity_id)
            .is_some_and(entity_supports_offset)
}

pub fn build_offset_preview(
    document: &Document,
    source_id: u64,
    distance: f64,
    side_point: Point,
) -> Option<Entity> {
    if !offset_source_candidate(document, source_id) {
        return None;
    }
    let source = document.entities.iter().find(|e| e.id() == source_id)?;
    offset_entity_geometry(source, distance, side_point)
}

/// Pure commit payload built under a read-only `Document` borrow.
#[derive(Clone, Debug)]
pub struct OffsetCommitPlan {
    pub source_entity_id: u64,
    pub entity: Entity,
    pub new_entity_id: u64,
    pub inherited_color: Option<String>,
    pub inherited_line_weight: Option<f64>,
    pub inherited_line_type: Option<String>,
    pub layout_name: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OffsetCommitError {
    HistoryBusy,
}

pub fn build_offset_commit_plan(
    document: &Document,
    source_id: u64,
    distance: f64,
    side_point: Point,
) -> Option<OffsetCommitPlan> {
    if !offset_source_candidate(document, source_id) {
        return None;
    }
    let source = document.entities.iter().find(|e| e.id() == source_id)?;
    let mut entity = offset_entity_geometry(source, distance, side_point)?;
    let new_entity_id = document.next_id();
    entity.set_layer(&document.active_layer_name);
    match &mut entity {
        Entity::Line { id, .. } | Entity::Circle { id, .. } | Entity::Polyline { id, .. } => {
            *id = new_entity_id
        }
        _ => return None,
    }
    let layout_name = (document.active_layout != "Model").then(|| document.active_layout.clone());
    Some(OffsetCommitPlan {
        source_entity_id: source_id,
        entity,
        new_entity_id,
        inherited_color: document.entity_colors.get(&source_id).cloned(),
        inherited_line_weight: document.entity_line_weights.get(&source_id).copied(),
        inherited_line_type: document.entity_line_types.get(&source_id).cloned(),
        layout_name,
    })
}

pub fn apply_offset_commit_plan(
    document: &mut Document,
    history: &mut LegacyHistoryManager,
    plan: OffsetCommitPlan,
) -> u64 {
    let new_id = plan.new_entity_id;
    document.add_entity(plan.entity);
    document.invalidate_entity_bounds(new_id);
    if let Some(color) = plan.inherited_color {
        document.entity_colors.insert(new_id, color);
    }
    if let Some(weight) = plan.inherited_line_weight {
        document.entity_line_weights.insert(new_id, weight);
    }
    if let Some(line_type) = plan.inherited_line_type {
        document.entity_line_types.insert(new_id, line_type);
    }
    if let Some(layout) = plan.layout_name {
        document.entity_layouts.insert(new_id, layout);
    }
    document.modified = true;
    record_entities_added(history, document, &[new_id]);
    new_id
}

pub fn try_apply_offset_commit_plan(
    document: &mut Document,
    history: &Rc<RefCell<LegacyHistoryManager>>,
    plan: OffsetCommitPlan,
) -> Result<u64, OffsetCommitError> {
    let mut history = history
        .try_borrow_mut()
        .map_err(|_| OffsetCommitError::HistoryBusy)?;
    Ok(apply_offset_commit_plan(document, &mut history, plan))
}

pub fn commit_offset_entity(
    document: &mut Document,
    history: &mut LegacyHistoryManager,
    source_id: u64,
    distance: f64,
    side_point: Point,
) -> Option<u64> {
    let plan = build_offset_commit_plan(document, source_id, distance, side_point)?;
    Some(apply_offset_commit_plan(document, history, plan))
}

pub fn handle_offset_tool_click(
    document: &mut Document,
    history: &Rc<RefCell<LegacyHistoryManager>>,
    point: Point,
    hit_tolerance: f64,
    distance: f64,
    offset_source: &Rc<RefCell<Option<u64>>>,
    hit_entity: impl FnOnce(&Document, Point, f64) -> Option<u64>,
) -> OffsetClickResult {
    if distance <= 0.0 || !distance.is_finite() {
        return OffsetClickResult::InvalidDistance;
    }
    // Copy out so the immutable `offset_source` borrow does not live across commit.
    let pending_source = *offset_source.borrow();
    if let Some(source_id) = pending_source {
        let plan = build_offset_commit_plan(document, source_id, distance, point);
        let committed =
            plan.and_then(|plan| try_apply_offset_commit_plan(document, history, plan).ok());
        *offset_source.borrow_mut() = None;
        return if committed.is_some() {
            OffsetClickResult::Committed
        } else {
            OffsetClickResult::CommitFailed
        };
    }
    let Some(entity_id) = hit_entity(document, point, hit_tolerance) else {
        return OffsetClickResult::NoEntity;
    };
    if !offset_source_candidate(document, entity_id) {
        return OffsetClickResult::UnsupportedEntity;
    }
    *offset_source.borrow_mut() = Some(entity_id);
    OffsetClickResult::SourceSelected
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OffsetClickResult {
    SourceSelected,
    Committed,
    NoEntity,
    UnsupportedEntity,
    CommitFailed,
    InvalidDistance,
}

fn parse_command_point(value: &str) -> Option<Point> {
    let (x, y) = value.split_once(',')?;
    Some(Point {
        x: x.trim().parse().ok()?,
        y: y.trim().parse().ok()?,
    })
}

/// `offset 10`, `o 10`, or `offset 10 100,50` (single selection + side point).
pub fn try_execute_offset_command(
    command: &str,
    document: &mut Document,
    history: &Rc<RefCell<LegacyHistoryManager>>,
    selected: &[u64],
    tool_parameters: &mut ToolParametersState,
    status: Option<&gtk::Label>,
) -> OffsetCommandResult {
    let set_status = |text: &str| {
        if let Some(label) = status {
            label.set_text(text);
        }
    };
    let parts: Vec<&str> = command.split_whitespace().collect();
    let Some(cmd) = parts.first().copied() else {
        return OffsetCommandResult::NotHandled;
    };
    if cmd != "offset" && cmd != "o" {
        return OffsetCommandResult::NotHandled;
    }
    if parts.len() < 2 {
        set_status("OFFSET: specify distance, e.g. offset 10");
        return OffsetCommandResult::Handled;
    }
    let Ok(distance) = parts[1].parse::<f64>() else {
        set_status("OFFSET: invalid distance");
        return OffsetCommandResult::Handled;
    };
    if distance <= 0.0 || !distance.is_finite() {
        set_status("OFFSET: distance must be positive");
        return OffsetCommandResult::Handled;
    }
    tool_parameters.offset_distance = distance;

    if parts.len() >= 3 {
        let Some(side) = parse_command_point(parts[2]) else {
            set_status("OFFSET: invalid side point");
            return OffsetCommandResult::Handled;
        };
        if selected.len() != 1 {
            set_status("OFFSET: select exactly one entity for offset 10 x,y");
            return OffsetCommandResult::Handled;
        }
        let source_id = selected[0];
        if !offset_source_candidate(document, source_id) {
            set_status("OFFSET: entity not valid for offset");
            return OffsetCommandResult::Handled;
        }
        let Some(plan) = build_offset_commit_plan(document, source_id, distance, side) else {
            set_status("OFFSET: geometry failed");
            return OffsetCommandResult::Handled;
        };
        if try_apply_offset_commit_plan(document, history, plan).is_ok() {
            set_status(&format!("OFFSET: created at distance {distance:.4}"));
            return OffsetCommandResult::Applied;
        }
        set_status("OFFSET: geometry failed");
        return OffsetCommandResult::Handled;
    }

    set_status(&format!("OFFSET: distance set to {distance:.4}"));
    OffsetCommandResult::SetDistance
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OffsetCommandResult {
    NotHandled,
    Handled,
    SetDistance,
    Applied,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::history::LegacyHistoryManager;

    #[test]
    fn offset_plan_built_without_mutation() {
        let mut doc = Document::new_empty();
        let id = doc.next_id();
        doc.add_entity(Entity::Line {
            id,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        });
        let plan = build_offset_commit_plan(&doc, id, 2.0, Point { x: 5.0, y: 3.0 }).unwrap();
        assert_eq!(doc.entities.len(), 1);
        assert_eq!(plan.new_entity_id, 2);
    }

    #[test]
    fn offset_commit_registers_history() {
        let mut doc = Document::new_empty();
        let id = doc.next_id();
        doc.add_entity(Entity::Line {
            id,
            layer: "Default".to_string(),
            start: Point::default(),
            end: Point { x: 10.0, y: 0.0 },
        });
        let plan = build_offset_commit_plan(&doc, id, 2.0, Point { x: 5.0, y: 2.0 }).unwrap();
        let mut history = LegacyHistoryManager::new();
        let new_id = apply_offset_commit_plan(&mut doc, &mut history, plan);
        assert_eq!(doc.entities.len(), 2);
        assert_eq!(new_id, 2);
    }

    #[test]
    fn offset_commit_returns_history_busy_when_history_borrowed() {
        let mut doc = Document::new_empty();
        let id = doc.next_id();
        doc.add_entity(Entity::Line {
            id,
            layer: "Default".to_string(),
            start: Point::default(),
            end: Point { x: 1.0, y: 0.0 },
        });
        let history = Rc::new(RefCell::new(LegacyHistoryManager::new()));
        let plan = build_offset_commit_plan(&doc, id, 1.0, Point { x: 0.0, y: 1.0 }).unwrap();
        let _hold = history.borrow_mut();
        assert_eq!(
            try_apply_offset_commit_plan(&mut doc, &history, plan),
            Err(OffsetCommitError::HistoryBusy)
        );
    }

    #[test]
    fn offset_click_second_click_does_not_panic_offset_source() {
        let mut doc = Document::new_empty();
        let id = doc.next_id();
        doc.add_entity(Entity::Line {
            id,
            layer: "Default".to_string(),
            start: Point::default(),
            end: Point { x: 10.0, y: 0.0 },
        });
        let history = Rc::new(RefCell::new(LegacyHistoryManager::new()));
        let offset_source = Rc::new(RefCell::new(None));
        let hit = |doc: &Document, _: Point, _: f64| doc.entities.first().map(|entity| entity.id());
        assert_eq!(
            handle_offset_tool_click(
                &mut doc,
                &history,
                Point::default(),
                1.0,
                2.0,
                &offset_source,
                hit,
            ),
            OffsetClickResult::SourceSelected
        );
        assert_eq!(
            handle_offset_tool_click(
                &mut doc,
                &history,
                Point { x: 5.0, y: 3.0 },
                1.0,
                2.0,
                &offset_source,
                hit,
            ),
            OffsetClickResult::Committed
        );
        assert!(offset_source.borrow().is_none());
        assert_eq!(doc.entities.len(), 2);
    }

    #[test]
    fn hidden_source_does_not_build_plan() {
        let mut doc = Document::new_empty();
        doc.create_layer("Hidden");
        doc.set_layer_visible("Hidden", false);
        let id = doc.next_id();
        doc.add_entity(Entity::Line {
            id,
            layer: "Hidden".to_string(),
            start: Point::default(),
            end: Point { x: 1.0, y: 0.0 },
        });
        assert!(build_offset_commit_plan(&doc, id, 1.0, Point { x: 0.0, y: 1.0 }).is_none());
    }

    #[test]
    fn locked_source_can_build_plan() {
        let mut doc = Document::new_empty();
        doc.create_layer("Locked");
        doc.set_layer_locked("Locked", true);
        let id = doc.next_id();
        doc.add_entity(Entity::Line {
            id,
            layer: "Locked".to_string(),
            start: Point::default(),
            end: Point { x: 10.0, y: 0.0 },
        });
        assert!(build_offset_commit_plan(&doc, id, 2.0, Point { x: 5.0, y: 3.0 }).is_some());
    }

    #[test]
    fn offset_command_parser_distance() {
        let mut doc = Document::new_empty();
        let history = Rc::new(RefCell::new(LegacyHistoryManager::new()));
        let mut params = ToolParametersState::default();
        let result =
            try_execute_offset_command("offset 10", &mut doc, &history, &[], &mut params, None);
        assert_eq!(result, OffsetCommandResult::SetDistance);
        assert!((params.offset_distance - 10.0).abs() < 1e-9);
    }

    #[test]
    fn offset_selected_line_creates_new_entity() {
        let mut doc = Document::new_empty();
        doc.set_active_layer_name("Default");
        let id = doc.next_id();
        doc.add_entity(Entity::Line {
            id,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        });
        let history = Rc::new(RefCell::new(LegacyHistoryManager::new()));
        let new_id = commit_offset_entity(
            &mut doc,
            &mut history.borrow_mut(),
            id,
            2.0,
            Point { x: 5.0, y: 3.0 },
        )
        .unwrap();
        assert_ne!(new_id, id);
        assert_eq!(doc.entities.len(), 2);
    }

    #[test]
    fn offset_undo_redo() {
        let mut doc = Document::new_empty();
        let id = doc.next_id();
        doc.add_entity(Entity::Line {
            id,
            layer: "Default".to_string(),
            start: Point::default(),
            end: Point { x: 10.0, y: 0.0 },
        });
        let history = Rc::new(RefCell::new(LegacyHistoryManager::new()));
        commit_offset_entity(
            &mut doc,
            &mut history.borrow_mut(),
            id,
            2.0,
            Point { x: 5.0, y: 2.0 },
        );
        assert_eq!(doc.entities.len(), 2);
        assert!(history.borrow_mut().undo(&mut doc));
        assert_eq!(doc.entities.len(), 1);
        assert!(history.borrow_mut().redo(&mut doc));
        assert_eq!(doc.entities.len(), 2);
    }
}
