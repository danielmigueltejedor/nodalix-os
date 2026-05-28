//! FILLET / CHAMFER tools: two-line corner, preview, modify+add history.

use crate::{
    cad::geometry::{chamfer_line_line, fillet_line_line, ChamferResult, FilletResult},
    cad::history::{
        capture_entity_snapshots, LegacyHistoryManager, LegacyModifyAndAddEntitiesAction,
    },
    canvas_trim_extend::{pick_edge_at, EdgeRef},
    document::{Document, Entity},
    geometry::Point,
    tool_parameters::ToolParametersState,
};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy, Debug)]
pub struct CornerSelection {
    pub edge: EdgeRef,
    pub pick: Point,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilletChamferClickResult {
    FirstLineSelected,
    Applied,
    NoEdge,
    LockedDenied,
    GeometryFailed,
    SameLine,
}

pub fn line_candidate(document: &Document, entity_id: u64) -> bool {
    document.entity_visible_in_active_layout(entity_id)
        && !document.entity_layer_locked(entity_id)
        && document
            .entities
            .iter()
            .find(|e| e.id() == entity_id)
            .is_some_and(|e| matches!(e, Entity::Line { .. } | Entity::Polyline { .. }))
}

fn apply_edge_endpoints(
    entity: &mut Entity,
    edge: EdgeRef,
    new_start: Point,
    new_end: Point,
) -> bool {
    match entity {
        Entity::Line { start, end, id, .. } if *id == edge.entity_id => {
            *start = new_start;
            *end = new_end;
            true
        }
        Entity::Polyline {
            id, points, closed, ..
        } if *id == edge.entity_id => {
            let index = edge.segment_index;
            if index >= points.len().saturating_sub(1) && !*closed {
                return false;
            }
            let point_count = points.len();
            points[index] = new_start;
            points[(index + 1) % point_count] = new_end;
            true
        }
        _ => false,
    }
}

fn compute_chamfer(
    edge_a: EdgeRef,
    edge_b: EdgeRef,
    pick_a: Point,
    pick_b: Point,
    distance_a: f64,
    distance_b: f64,
) -> Option<ChamferResult> {
    chamfer_line_line(
        edge_a.start,
        edge_a.end,
        edge_b.start,
        edge_b.end,
        distance_a,
        distance_b,
        pick_a,
        pick_b,
    )
}

fn compute_fillet(
    edge_a: EdgeRef,
    edge_b: EdgeRef,
    pick_a: Point,
    pick_b: Point,
    radius: f64,
) -> Option<FilletResult> {
    fillet_line_line(
        edge_a.start,
        edge_a.end,
        edge_b.start,
        edge_b.end,
        radius,
        pick_a,
        pick_b,
    )
}

pub fn build_chamfer_preview(
    document: &Document,
    first: EdgeRef,
    second: EdgeRef,
    pick_a: Point,
    pick_b: Point,
    distance_a: f64,
    distance_b: f64,
) -> Option<Vec<Entity>> {
    if !line_candidate(document, first.entity_id) || !line_candidate(document, second.entity_id) {
        return None;
    }
    let result = compute_chamfer(first, second, pick_a, pick_b, distance_a, distance_b)?;
    preview_from_chamfer(document, first, second, &result)
}

pub fn build_fillet_preview(
    document: &Document,
    first: EdgeRef,
    second: EdgeRef,
    pick_a: Point,
    pick_b: Point,
    radius: f64,
) -> Option<Vec<Entity>> {
    if !line_candidate(document, first.entity_id) || !line_candidate(document, second.entity_id) {
        return None;
    }
    let result = compute_fillet(first, second, pick_a, pick_b, radius)?;
    preview_from_fillet(document, first, second, &result)
}

fn preview_from_chamfer(
    document: &Document,
    first: EdgeRef,
    second: EdgeRef,
    result: &ChamferResult,
) -> Option<Vec<Entity>> {
    let mut out = Vec::new();
    let e1 = document
        .entities
        .iter()
        .find(|e| e.id() == first.entity_id)?;
    let e2 = document
        .entities
        .iter()
        .find(|e| e.id() == second.entity_id)?;
    let mut copy1 = e1.clone();
    let mut copy2 = e2.clone();
    apply_edge_endpoints(&mut copy1, first, result.new_a_start, result.new_a_end);
    apply_edge_endpoints(&mut copy2, second, result.new_b_start, result.new_b_end);
    out.push(copy1);
    out.push(copy2);
    out.push(Entity::Line {
        id: 0,
        layer: document.active_layer_name.clone(),
        start: result.chamfer_line_start,
        end: result.chamfer_line_end,
    });
    Some(out)
}

fn preview_from_fillet(
    document: &Document,
    first: EdgeRef,
    second: EdgeRef,
    result: &FilletResult,
) -> Option<Vec<Entity>> {
    let mut out = Vec::new();
    let e1 = document
        .entities
        .iter()
        .find(|e| e.id() == first.entity_id)?;
    let e2 = document
        .entities
        .iter()
        .find(|e| e.id() == second.entity_id)?;
    let mut copy1 = e1.clone();
    let mut copy2 = e2.clone();
    apply_edge_endpoints(&mut copy1, first, result.new_a_start, result.new_a_end);
    apply_edge_endpoints(&mut copy2, second, result.new_b_start, result.new_b_end);
    out.push(copy1);
    out.push(copy2);
    out.push(Entity::Polyline {
        id: 0,
        layer: document.active_layer_name.clone(),
        points: result.arc_points.clone(),
        closed: false,
    });
    Some(out)
}

pub fn apply_chamfer(
    document: &mut Document,
    history: &mut LegacyHistoryManager,
    first: EdgeRef,
    second: EdgeRef,
    pick_a: Point,
    pick_b: Point,
    distance_a: f64,
    distance_b: f64,
) -> bool {
    if !line_candidate(document, first.entity_id) || !line_candidate(document, second.entity_id) {
        return false;
    }
    let Some(result) = compute_chamfer(first, second, pick_a, pick_b, distance_a, distance_b)
    else {
        return false;
    };
    commit_corner(
        document,
        history,
        first,
        second,
        &result.new_a_start,
        &result.new_a_end,
        &result.new_b_start,
        &result.new_b_end,
        Entity::Line {
            id: 0,
            layer: document.active_layer_name.clone(),
            start: result.chamfer_line_start,
            end: result.chamfer_line_end,
        },
    )
}

pub fn apply_fillet(
    document: &mut Document,
    history: &mut LegacyHistoryManager,
    first: EdgeRef,
    second: EdgeRef,
    pick_a: Point,
    pick_b: Point,
    radius: f64,
) -> bool {
    if !line_candidate(document, first.entity_id) || !line_candidate(document, second.entity_id) {
        return false;
    }
    let Some(result) = compute_fillet(first, second, pick_a, pick_b, radius) else {
        return false;
    };
    commit_corner(
        document,
        history,
        first,
        second,
        &result.new_a_start,
        &result.new_a_end,
        &result.new_b_start,
        &result.new_b_end,
        Entity::Polyline {
            id: 0,
            layer: document.active_layer_name.clone(),
            points: result.arc_points,
            closed: false,
        },
    )
}

fn commit_corner(
    document: &mut Document,
    history: &mut LegacyHistoryManager,
    first: EdgeRef,
    second: EdgeRef,
    new_a_start: &Point,
    new_a_end: &Point,
    new_b_start: &Point,
    new_b_end: &Point,
    mut new_entity: Entity,
) -> bool {
    let ids = [first.entity_id, second.entity_id];
    let before = capture_entity_snapshots(document, &ids);
    {
        let Some(entity) = document
            .entities
            .iter_mut()
            .find(|e| e.id() == first.entity_id)
        else {
            return false;
        };
        if !apply_edge_endpoints(entity, first, *new_a_start, *new_a_end) {
            return false;
        }
    }
    {
        let Some(entity) = document
            .entities
            .iter_mut()
            .find(|e| e.id() == second.entity_id)
        else {
            return false;
        };
        if !apply_edge_endpoints(entity, second, *new_b_start, *new_b_end) {
            return false;
        }
    }
    for id in &ids {
        document.invalidate_entity_bounds(*id);
    }
    let after_modified = capture_entity_snapshots(document, &ids);
    let new_id = document.next_id();
    match &mut new_entity {
        Entity::Line { id, .. } | Entity::Polyline { id, .. } => *id = new_id,
        _ => return false,
    }
    document.add_entity(new_entity);
    document.modified = true;
    let added = capture_entity_snapshots(document, &[new_id]);
    if before.is_empty() || after_modified.is_empty() || added.is_empty() {
        return false;
    }
    history.record(Box::new(LegacyModifyAndAddEntitiesAction {
        before,
        after: after_modified,
        added,
    }));
    true
}

pub fn handle_fillet_chamfer_click(
    document: &mut Document,
    history: &Rc<RefCell<LegacyHistoryManager>>,
    point: Point,
    tolerance: f64,
    first_selection: &Rc<RefCell<Option<CornerSelection>>>,
    chamfer_mode: bool,
    distance_a: f64,
    distance_b: f64,
    fillet_radius: f64,
) -> FilletChamferClickResult {
    let pending_first = *first_selection.borrow();
    if let Some(first) = pending_first {
        let Some(second) = pick_edge_at(document, point, tolerance) else {
            return FilletChamferClickResult::NoEdge;
        };
        if first.edge.entity_id == second.entity_id
            && first.edge.segment_index == second.segment_index
        {
            return FilletChamferClickResult::SameLine;
        }
        if !line_candidate(document, first.edge.entity_id)
            || !line_candidate(document, second.entity_id)
        {
            return FilletChamferClickResult::LockedDenied;
        }
        let ok = if chamfer_mode {
            apply_chamfer(
                document,
                &mut history.borrow_mut(),
                first.edge,
                second,
                first.pick,
                point,
                distance_a,
                distance_b,
            )
        } else {
            apply_fillet(
                document,
                &mut history.borrow_mut(),
                first.edge,
                second,
                first.pick,
                point,
                fillet_radius,
            )
        };
        *first_selection.borrow_mut() = None;
        return if ok {
            FilletChamferClickResult::Applied
        } else {
            FilletChamferClickResult::GeometryFailed
        };
    }
    let Some(edge) = pick_edge_at(document, point, tolerance) else {
        return FilletChamferClickResult::NoEdge;
    };
    if !line_candidate(document, edge.entity_id) {
        return FilletChamferClickResult::LockedDenied;
    }
    *first_selection.borrow_mut() = Some(CornerSelection { edge, pick: point });
    FilletChamferClickResult::FirstLineSelected
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilletChamferCommandOutcome {
    ActivateFillet,
    ActivateChamfer,
}

pub fn try_execute_fillet_chamfer_command(
    command: &str,
    params: &mut ToolParametersState,
) -> Option<FilletChamferCommandOutcome> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    let Some(cmd) = parts.first().copied() else {
        return None;
    };
    match cmd {
        "fillet" | "f" => {
            if let Some(value) = parts.get(1).and_then(|s| s.parse::<f64>().ok()) {
                if value > 0.0 && value.is_finite() {
                    params.fillet_radius = value;
                }
            }
            Some(FilletChamferCommandOutcome::ActivateFillet)
        }
        "chamfer" | "cha" => {
            if let Some(d1) = parts.get(1).and_then(|s| s.parse::<f64>().ok()) {
                if d1 >= 0.0 && d1.is_finite() {
                    params.chamfer_distance_1 = d1;
                    params.chamfer_distance_2 = parts
                        .get(2)
                        .and_then(|s| s.parse::<f64>().ok())
                        .filter(|d| *d >= 0.0 && d.is_finite())
                        .unwrap_or(d1);
                }
            }
            Some(FilletChamferCommandOutcome::ActivateChamfer)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::history::LegacyHistoryManager;

    fn perpendicular_lines(doc: &mut Document) -> (u64, u64) {
        let a = doc.next_id();
        doc.add_entity(Entity::Line {
            id: a,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        });
        let b = doc.next_id();
        doc.add_entity(Entity::Line {
            id: b,
            layer: "Default".to_string(),
            start: Point { x: 10.0, y: 0.0 },
            end: Point { x: 10.0, y: 10.0 },
        });
        (a, b)
    }

    fn edge(id: u64, start: Point, end: Point) -> EdgeRef {
        EdgeRef {
            entity_id: id,
            segment_index: 0,
            start,
            end,
        }
    }

    #[test]
    fn locked_line_blocks_fillet() {
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
        assert!(!line_candidate(&doc, id));
    }

    #[test]
    fn locked_line_blocks_chamfer() {
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
        assert!(!line_candidate(&doc, id));
    }

    #[test]
    fn hidden_line_not_candidate() {
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
        assert!(!line_candidate(&doc, id));
    }

    #[test]
    fn chamfer_undo_redo() {
        let mut doc = Document::new_empty();
        let (a, b) = perpendicular_lines(&mut doc);
        let first = edge(a, Point { x: 0.0, y: 0.0 }, Point { x: 10.0, y: 0.0 });
        let second = edge(b, Point { x: 10.0, y: 0.0 }, Point { x: 10.0, y: 10.0 });
        let mut history = LegacyHistoryManager::new();
        assert!(apply_chamfer(
            &mut doc,
            &mut history,
            first,
            second,
            Point { x: 5.0, y: 0.0 },
            Point { x: 10.0, y: 5.0 },
            2.0,
            2.0,
        ));
        assert_eq!(doc.entities.len(), 3);
        assert!(history.undo(&mut doc));
        assert_eq!(doc.entities.len(), 2);
        assert!(history.redo(&mut doc));
        assert_eq!(doc.entities.len(), 3);
    }

    #[test]
    fn fillet_undo_redo() {
        let mut doc = Document::new_empty();
        let (a, b) = perpendicular_lines(&mut doc);
        let first = edge(a, Point { x: 0.0, y: 0.0 }, Point { x: 10.0, y: 0.0 });
        let second = edge(b, Point { x: 10.0, y: 0.0 }, Point { x: 10.0, y: 10.0 });
        let mut history = LegacyHistoryManager::new();
        assert!(apply_fillet(
            &mut doc,
            &mut history,
            first,
            second,
            Point { x: 5.0, y: 0.0 },
            Point { x: 10.0, y: 5.0 },
            2.0,
        ));
        assert!(doc
            .entities
            .iter()
            .any(|e| matches!(e, Entity::Polyline { .. })));
        assert!(history.undo(&mut doc));
        assert!(!doc
            .entities
            .iter()
            .any(|e| matches!(e, Entity::Polyline { .. })));
    }

    #[test]
    fn command_parser_fillet_and_chamfer() {
        let mut params = ToolParametersState::default();
        assert_eq!(
            try_execute_fillet_chamfer_command("fillet 10", &mut params),
            Some(FilletChamferCommandOutcome::ActivateFillet)
        );
        assert!((params.fillet_radius - 10.0).abs() < 1e-9);
        assert_eq!(
            try_execute_fillet_chamfer_command("f", &mut params),
            Some(FilletChamferCommandOutcome::ActivateFillet)
        );
        assert_eq!(
            try_execute_fillet_chamfer_command("chamfer 5 8", &mut params),
            Some(FilletChamferCommandOutcome::ActivateChamfer)
        );
        assert!((params.chamfer_distance_1 - 5.0).abs() < 1e-9);
        assert!((params.chamfer_distance_2 - 8.0).abs() < 1e-9);
    }

    #[test]
    fn preview_chamfer_geometry() {
        let mut doc = Document::new_empty();
        let (a, b) = perpendicular_lines(&mut doc);
        let preview = build_chamfer_preview(
            &doc,
            edge(a, Point { x: 0.0, y: 0.0 }, Point { x: 10.0, y: 0.0 }),
            edge(b, Point { x: 10.0, y: 0.0 }, Point { x: 10.0, y: 10.0 }),
            Point { x: 5.0, y: 0.0 },
            Point { x: 10.0, y: 5.0 },
            2.0,
            2.0,
        )
        .unwrap();
        assert_eq!(preview.len(), 3);
    }

    #[test]
    fn preview_fillet_geometry() {
        let mut doc = Document::new_empty();
        let (a, b) = perpendicular_lines(&mut doc);
        let preview = build_fillet_preview(
            &doc,
            edge(a, Point { x: 0.0, y: 0.0 }, Point { x: 10.0, y: 0.0 }),
            edge(b, Point { x: 10.0, y: 0.0 }, Point { x: 10.0, y: 10.0 }),
            Point { x: 5.0, y: 0.0 },
            Point { x: 10.0, y: 5.0 },
            2.0,
        )
        .unwrap();
        assert_eq!(preview.len(), 3);
        assert!(preview.iter().any(|e| matches!(e, Entity::Polyline { .. })));
    }
}
