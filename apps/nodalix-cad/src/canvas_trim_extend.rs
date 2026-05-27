//! TRIM / EXTEND tools: boundary pick, target modify, preview, history.

use crate::{
    cad::geometry::{extend_line_to_boundary, trim_line_to_boundary},
    cad::history::{capture_entity_snapshots, LegacyHistoryManager, LegacyTransformEntitiesAction},
    document::{Document, Entity},
    geometry::Point,
};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeRef {
    pub entity_id: u64,
    pub segment_index: usize,
    pub start: Point,
    pub end: Point,
}

#[derive(Clone, Copy, Debug)]
pub struct TrimBoundary {
    pub start: Point,
    pub end: Point,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrimExtendClickResult {
    BoundarySelected,
    Applied,
    NoEdge,
    TargetDenied,
    GeometryFailed,
}

pub fn entity_supports_trim_extend(entity: &Entity) -> bool {
    matches!(entity, Entity::Line { .. } | Entity::Polyline { .. })
}

pub fn boundary_candidate(document: &Document, entity_id: u64) -> bool {
    document.entity_visible_in_active_layout(entity_id)
        && document
            .entities
            .iter()
            .find(|e| e.id() == entity_id)
            .is_some_and(entity_supports_trim_extend)
}

pub fn target_candidate(document: &Document, entity_id: u64) -> bool {
    boundary_candidate(document, entity_id) && !document.entity_layer_locked(entity_id)
}

pub fn pick_edge_at(document: &Document, point: Point, tolerance: f64) -> Option<EdgeRef> {
    let mut best: Option<(EdgeRef, f64)> = None;
    for entity in document.entities.iter() {
        if !document.entity_visible_in_active_layout(entity.id()) {
            continue;
        }
        let id = entity.id();
        match entity {
            Entity::Line { start, end, .. } => {
                let dist = distance_to_segment(point, *start, *end);
                if dist <= tolerance {
                    let edge = EdgeRef {
                        entity_id: id,
                        segment_index: 0,
                        start: *start,
                        end: *end,
                    };
                    if best.as_ref().map(|(_, d)| dist < *d).unwrap_or(true) {
                        best = Some((edge, dist));
                    }
                }
            }
            Entity::Polyline { points, closed, .. } => {
                if points.len() < 2 {
                    continue;
                }
                let segment_count = if *closed {
                    points.len()
                } else {
                    points.len() - 1
                };
                for index in 0..segment_count {
                    let start = points[index];
                    let end = points[(index + 1) % points.len()];
                    let dist = distance_to_segment(point, start, end);
                    if dist <= tolerance {
                        let edge = EdgeRef {
                            entity_id: id,
                            segment_index: index,
                            start,
                            end,
                        };
                        if best.as_ref().map(|(_, d)| dist < *d).unwrap_or(true) {
                            best = Some((edge, dist));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    best.map(|(edge, _)| edge)
}

fn distance_to_segment(point: Point, start: Point, end: Point) -> f64 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-12 {
        return point.distance_to(start);
    }
    let t = ((point.x - start.x) * dx + (point.y - start.y) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let proj = Point {
        x: start.x + t * dx,
        y: start.y + t * dy,
    };
    point.distance_to(proj)
}

pub fn build_trim_preview(
    document: &Document,
    target: EdgeRef,
    boundary: TrimBoundary,
    pick_point: Point,
) -> Option<Entity> {
    if !target_candidate(document, target.entity_id) {
        return None;
    }
    let entity = document
        .entities
        .iter()
        .find(|e| e.id() == target.entity_id)?;
    preview_trim_entity(entity, target, boundary, pick_point)
}

pub fn build_extend_preview(
    document: &Document,
    target: EdgeRef,
    boundary: TrimBoundary,
    pick_point: Point,
) -> Option<Entity> {
    if !target_candidate(document, target.entity_id) {
        return None;
    }
    let entity = document
        .entities
        .iter()
        .find(|e| e.id() == target.entity_id)?;
    preview_extend_entity(entity, target, boundary, pick_point)
}

fn preview_trim_entity(
    entity: &Entity,
    target: EdgeRef,
    boundary: TrimBoundary,
    pick_point: Point,
) -> Option<Entity> {
    let mut copy = entity.clone();
    if apply_trim_to_entity(&mut copy, target, boundary, pick_point) {
        Some(copy)
    } else {
        None
    }
}

fn preview_extend_entity(
    entity: &Entity,
    target: EdgeRef,
    boundary: TrimBoundary,
    pick_point: Point,
) -> Option<Entity> {
    let mut copy = entity.clone();
    if apply_extend_to_entity(&mut copy, target, boundary, pick_point) {
        Some(copy)
    } else {
        None
    }
}

fn apply_trim_to_entity(
    entity: &mut Entity,
    target: EdgeRef,
    boundary: TrimBoundary,
    pick_point: Point,
) -> bool {
    match entity {
        Entity::Line { start, end, id, .. } if *id == target.entity_id => {
            if let Some((s, e)) =
                trim_line_to_boundary(*start, *end, boundary.start, boundary.end, pick_point)
            {
                *start = s;
                *end = e;
                return true;
            }
            false
        }
        Entity::Polyline {
            id, points, closed, ..
        } if *id == target.entity_id => {
            let index = target.segment_index;
            if index >= points.len().saturating_sub(1) && !*closed {
                return false;
            }
            let seg_start = points[index];
            let seg_end = points[(index + 1) % points.len()];
            let Some((new_start, new_end)) =
                trim_line_to_boundary(seg_start, seg_end, boundary.start, boundary.end, pick_point)
            else {
                return false;
            };
            let point_count = points.len();
            points[index] = new_start;
            points[(index + 1) % point_count] = new_end;
            true
        }
        _ => false,
    }
}

fn apply_extend_to_entity(
    entity: &mut Entity,
    target: EdgeRef,
    boundary: TrimBoundary,
    pick_point: Point,
) -> bool {
    match entity {
        Entity::Line { start, end, id, .. } if *id == target.entity_id => {
            if let Some((s, e)) =
                extend_line_to_boundary(*start, *end, boundary.start, boundary.end, pick_point)
            {
                *start = s;
                *end = e;
                return true;
            }
            false
        }
        Entity::Polyline {
            id, points, closed, ..
        } if *id == target.entity_id => {
            let index = target.segment_index;
            if index >= points.len().saturating_sub(1) && !*closed {
                return false;
            }
            let seg_start = points[index];
            let seg_end = points[(index + 1) % points.len()];
            let Some((new_start, new_end)) = extend_line_to_boundary(
                seg_start,
                seg_end,
                boundary.start,
                boundary.end,
                pick_point,
            ) else {
                return false;
            };
            let point_count = points.len();
            points[index] = new_start;
            points[(index + 1) % point_count] = new_end;
            true
        }
        _ => false,
    }
}

pub fn apply_trim(
    document: &mut Document,
    history: &mut LegacyHistoryManager,
    target: EdgeRef,
    boundary: TrimBoundary,
    pick_point: Point,
) -> bool {
    if !target_candidate(document, target.entity_id) {
        return false;
    }
    let id = target.entity_id;
    let before = capture_entity_snapshots(document, &[id]);
    let Some(entity) = document.entities.iter_mut().find(|e| e.id() == id) else {
        return false;
    };
    if !apply_trim_to_entity(entity, target, boundary, pick_point) {
        return false;
    }
    document.invalidate_entity_bounds(id);
    document.modified = true;
    let after = capture_entity_snapshots(document, &[id]);
    if !before.is_empty() && !after.is_empty() {
        history.record(Box::new(LegacyTransformEntitiesAction { before, after }));
    }
    true
}

pub fn apply_extend(
    document: &mut Document,
    history: &mut LegacyHistoryManager,
    target: EdgeRef,
    boundary: TrimBoundary,
    pick_point: Point,
) -> bool {
    if !target_candidate(document, target.entity_id) {
        return false;
    }
    let id = target.entity_id;
    let before = capture_entity_snapshots(document, &[id]);
    let Some(entity) = document.entities.iter_mut().find(|e| e.id() == id) else {
        return false;
    };
    if !apply_extend_to_entity(entity, target, boundary, pick_point) {
        return false;
    }
    document.invalidate_entity_bounds(id);
    document.modified = true;
    let after = capture_entity_snapshots(document, &[id]);
    if !before.is_empty() && !after.is_empty() {
        history.record(Box::new(LegacyTransformEntitiesAction { before, after }));
    }
    true
}

pub fn handle_trim_extend_click(
    document: &mut Document,
    history: &Rc<RefCell<LegacyHistoryManager>>,
    point: Point,
    tolerance: f64,
    boundary: &Rc<RefCell<Option<TrimBoundary>>>,
    trim_mode: bool,
    pick_edge: impl FnOnce(&Document, Point, f64) -> Option<EdgeRef>,
) -> TrimExtendClickResult {
    let pending_boundary = *boundary.borrow();
    if let Some(boundary_geom) = pending_boundary {
        let Some(target) = pick_edge(document, point, tolerance) else {
            return TrimExtendClickResult::NoEdge;
        };
        if !target_candidate(document, target.entity_id) {
            return TrimExtendClickResult::TargetDenied;
        }
        let ok = if trim_mode {
            apply_trim(
                document,
                &mut history.borrow_mut(),
                target,
                boundary_geom,
                point,
            )
        } else {
            apply_extend(
                document,
                &mut history.borrow_mut(),
                target,
                boundary_geom,
                point,
            )
        };
        *boundary.borrow_mut() = None;
        return if ok {
            TrimExtendClickResult::Applied
        } else {
            TrimExtendClickResult::GeometryFailed
        };
    }
    let Some(edge) = pick_edge(document, point, tolerance) else {
        return TrimExtendClickResult::NoEdge;
    };
    if !boundary_candidate(document, edge.entity_id) {
        return TrimExtendClickResult::NoEdge;
    }
    *boundary.borrow_mut() = Some(TrimBoundary {
        start: edge.start,
        end: edge.end,
    });
    TrimExtendClickResult::BoundarySelected
}

pub fn try_execute_trim_extend_command(command: &str) -> Option<ToolActivation> {
    match command.trim().to_ascii_lowercase().as_str() {
        "trim" | "tr" => Some(ToolActivation::Trim),
        "extend" | "ex" => Some(ToolActivation::Extend),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolActivation {
    Trim,
    Extend,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::history::LegacyHistoryManager;

    fn sample_crossing_lines(doc: &mut Document) -> (u64, u64) {
        let h = doc.next_id();
        doc.add_entity(Entity::Line {
            id: h,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        });
        let v = doc.next_id();
        doc.add_entity(Entity::Line {
            id: v,
            layer: "Default".to_string(),
            start: Point { x: 5.0, y: -5.0 },
            end: Point { x: 5.0, y: 5.0 },
        });
        (h, v)
    }

    #[test]
    fn locked_boundary_allowed() {
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
        assert!(boundary_candidate(&doc, id));
        assert!(!target_candidate(&doc, id));
    }

    #[test]
    fn hidden_not_candidate() {
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
        assert!(!boundary_candidate(&doc, id));
    }

    #[test]
    fn trim_undo_redo() {
        let mut doc = Document::new_empty();
        let (h, v) = sample_crossing_lines(&mut doc);
        let boundary = TrimBoundary {
            start: Point { x: 5.0, y: -5.0 },
            end: Point { x: 5.0, y: 5.0 },
        };
        let target = EdgeRef {
            entity_id: h,
            segment_index: 0,
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        };
        let mut history = LegacyHistoryManager::new();
        assert!(apply_trim(
            &mut doc,
            &mut history,
            target,
            boundary,
            Point { x: 2.0, y: 0.0 },
        ));
        let line = doc.entities.iter().find(|e| e.id() == h).unwrap();
        if let Entity::Line { end, .. } = line {
            assert!((end.x - 5.0).abs() < 1e-6);
        } else {
            panic!("expected line");
        }
        assert!(history.undo(&mut doc));
        if let Entity::Line { end, .. } = doc.entities.iter().find(|e| e.id() == h).unwrap() {
            assert!((end.x - 10.0).abs() < 1e-6);
        }
        assert!(history.redo(&mut doc));
        if let Entity::Line { end, .. } = doc.entities.iter().find(|e| e.id() == h).unwrap() {
            assert!((end.x - 5.0).abs() < 1e-6);
        }
        let _ = v;
    }

    #[test]
    fn extend_undo_redo() {
        let mut doc = Document::new_empty();
        let h = doc.next_id();
        doc.add_entity(Entity::Line {
            id: h,
            layer: "Default".to_string(),
            start: Point { x: 2.0, y: 0.0 },
            end: Point { x: 6.0, y: 0.0 },
        });
        let _v = doc.next_id();
        doc.add_entity(Entity::Line {
            id: _v,
            layer: "Default".to_string(),
            start: Point { x: 10.0, y: -5.0 },
            end: Point { x: 10.0, y: 5.0 },
        });
        let boundary = TrimBoundary {
            start: Point { x: 10.0, y: -5.0 },
            end: Point { x: 10.0, y: 5.0 },
        };
        let target = EdgeRef {
            entity_id: h,
            segment_index: 0,
            start: Point { x: 2.0, y: 0.0 },
            end: Point { x: 6.0, y: 0.0 },
        };
        let mut history = LegacyHistoryManager::new();
        assert!(apply_extend(
            &mut doc,
            &mut history,
            target,
            boundary,
            Point { x: 6.0, y: 0.0 },
        ));
        if let Entity::Line { end, .. } = doc.entities.iter().find(|e| e.id() == h).unwrap() {
            assert!((end.x - 10.0).abs() < 1e-6);
        }
        assert!(history.undo(&mut doc));
        assert!(history.redo(&mut doc));
        if let Entity::Line { end, .. } = doc.entities.iter().find(|e| e.id() == h).unwrap() {
            assert!((end.x - 10.0).abs() < 1e-6);
        }
    }

    #[test]
    fn command_parser_trim_and_extend() {
        assert_eq!(
            try_execute_trim_extend_command("trim"),
            Some(ToolActivation::Trim)
        );
        assert_eq!(
            try_execute_trim_extend_command("TR"),
            Some(ToolActivation::Trim)
        );
        assert_eq!(
            try_execute_trim_extend_command("extend"),
            Some(ToolActivation::Extend)
        );
        assert_eq!(
            try_execute_trim_extend_command("ex"),
            Some(ToolActivation::Extend)
        );
    }

    #[test]
    fn preview_extend_geometry() {
        let mut doc = Document::new_empty();
        let h = doc.next_id();
        doc.add_entity(Entity::Line {
            id: h,
            layer: "Default".to_string(),
            start: Point { x: 2.0, y: 0.0 },
            end: Point { x: 6.0, y: 0.0 },
        });
        let target = EdgeRef {
            entity_id: h,
            segment_index: 0,
            start: Point { x: 2.0, y: 0.0 },
            end: Point { x: 6.0, y: 0.0 },
        };
        let preview = build_extend_preview(
            &doc,
            target,
            TrimBoundary {
                start: Point { x: 10.0, y: -5.0 },
                end: Point { x: 10.0, y: 5.0 },
            },
            Point { x: 6.0, y: 0.0 },
        )
        .unwrap();
        if let Entity::Line { end, .. } = preview {
            assert!((end.x - 10.0).abs() < 1e-6);
        } else {
            panic!("expected line preview");
        }
    }

    #[test]
    fn preview_trim_geometry() {
        let mut doc = Document::new_empty();
        let (h, _) = sample_crossing_lines(&mut doc);
        let target = EdgeRef {
            entity_id: h,
            segment_index: 0,
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        };
        let preview = build_trim_preview(
            &doc,
            target,
            TrimBoundary {
                start: Point { x: 5.0, y: -5.0 },
                end: Point { x: 5.0, y: 5.0 },
            },
            Point { x: 2.0, y: 0.0 },
        )
        .unwrap();
        if let Entity::Line { end, .. } = preview {
            assert!((end.x - 5.0).abs() < 1e-6);
        } else {
            panic!("expected line preview");
        }
    }

    #[test]
    fn click_flow_no_refcell_panic() {
        let mut doc = Document::new_empty();
        sample_crossing_lines(&mut doc);
        let history = Rc::new(RefCell::new(LegacyHistoryManager::new()));
        let boundary = Rc::new(RefCell::new(None));
        let pick = |d: &Document, p: Point, t: f64| pick_edge_at(d, p, t);
        assert_eq!(
            handle_trim_extend_click(
                &mut doc,
                &history,
                Point { x: 5.0, y: 3.0 },
                1.0,
                &boundary,
                true,
                pick,
            ),
            TrimExtendClickResult::BoundarySelected
        );
        assert_eq!(
            handle_trim_extend_click(
                &mut doc,
                &history,
                Point { x: 2.0, y: 0.0 },
                1.0,
                &boundary,
                true,
                pick,
            ),
            TrimExtendClickResult::Applied
        );
        assert!(boundary.borrow().is_none());
    }
}
