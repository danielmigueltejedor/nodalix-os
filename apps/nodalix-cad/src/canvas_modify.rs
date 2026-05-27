//! Rotate / Scale / Mirror tool logic (preview + commit).

use crate::{
    cad::geometry::{
        angle_degrees_from_points, clone_entities_offset, mirror_entity, rotate_entity,
        scale_entity, scale_factor_from_points,
    },
    cad::history::{
        record_entity_move_if_nonzero, record_entity_transform, LegacyAddEntitiesAction,
        LegacyHistoryManager,
    },
    document::{Document, Entity},
    geometry::Point,
    tools::Tool,
};
use std::{cell::RefCell, rc::Rc};

pub fn editable_selection_ids(document: &Document, selected: &[u64]) -> Vec<u64> {
    selected
        .iter()
        .copied()
        .filter(|id| {
            document.entity_visible_in_active_layout(*id) && !document.entity_layer_locked(*id)
        })
        .collect()
}

/// Visible entities eligible for copy (locked layers allowed; original is not modified).
pub fn copyable_selection_ids(document: &Document, selected: &[u64]) -> Vec<u64> {
    selected
        .iter()
        .copied()
        .filter(|id| document.entity_visible_in_active_layout(*id))
        .collect()
}

pub fn move_delta(base: Point, target: Point) -> (f64, f64) {
    (target.x - base.x, target.y - base.y)
}

pub fn build_move_copy_preview(
    document: &Document,
    selected_ids: &[u64],
    base: Point,
    hover: Point,
    copy: bool,
) -> Vec<Entity> {
    let (dx, dy) = move_delta(base, hover);
    if copy {
        let ids = copyable_selection_ids(document, selected_ids);
        let sources: Vec<Entity> = document
            .entities
            .iter()
            .filter(|entity| ids.contains(&entity.id()))
            .cloned()
            .collect();
        return clone_entities_offset(&sources, dx, dy);
    }
    let ids = editable_selection_ids(document, selected_ids);
    let mut preview: Vec<Entity> = document
        .entities
        .iter()
        .filter(|entity| ids.contains(&entity.id()))
        .cloned()
        .collect();
    for entity in &mut preview {
        entity.translate(dx, dy);
    }
    preview
}

pub fn apply_move_selection(
    document: &mut Document,
    history: &mut LegacyHistoryManager,
    selected: &[u64],
    base: Point,
    target: Point,
) -> bool {
    let ids = editable_selection_ids(document, selected);
    if ids.is_empty() {
        return false;
    }
    let (dx, dy) = move_delta(base, target);
    for id in &ids {
        document.translate_entity(*id, dx, dy);
    }
    record_entity_move_if_nonzero(history, ids, dx, dy);
    true
}

pub fn apply_copy_selection(
    document: &mut Document,
    history: &mut LegacyHistoryManager,
    selected: &[u64],
    base: Point,
    target: Point,
) -> Vec<u64> {
    let ids = copyable_selection_ids(document, selected);
    if ids.is_empty() {
        return Vec::new();
    }
    let (dx, dy) = move_delta(base, target);
    let sources = document.entities_by_ids(&ids);
    let pasted = document.paste_entities_translated(&sources, dx, dy);
    if pasted.is_empty() {
        return pasted;
    }
    let added = crate::cad::history::capture_entity_snapshots(document, &pasted);
    if !added.is_empty() {
        history.record(Box::new(LegacyAddEntitiesAction { added }));
    }
    pasted
}

pub fn handle_move_copy_tool_click(
    document: &mut Document,
    history: &Rc<RefCell<LegacyHistoryManager>>,
    tool: Tool,
    point: Point,
    selected: &[u64],
    pending: &Rc<RefCell<Option<Point>>>,
) -> bool {
    match tool {
        Tool::Move | Tool::Copy => {}
        _ => return false,
    }
    if pending.borrow().is_none() {
        *pending.borrow_mut() = Some(point);
        return true;
    }
    let Some(base) = *pending.borrow() else {
        return false;
    };
    let changed = match tool {
        Tool::Move => {
            apply_move_selection(document, &mut history.borrow_mut(), selected, base, point)
        }
        Tool::Copy => {
            !apply_copy_selection(document, &mut history.borrow_mut(), selected, base, point)
                .is_empty()
        }
        _ => false,
    };
    *pending.borrow_mut() = None;
    changed
}

pub fn build_modify_preview(
    document: &Document,
    selected_ids: &[u64],
    tool: Tool,
    base: Point,
    hover: Point,
    axis_start: Option<Point>,
    scale_reference: Option<Point>,
) -> Vec<Entity> {
    let ids = editable_selection_ids(document, selected_ids);
    if ids.is_empty() {
        return Vec::new();
    }
    let mut preview: Vec<Entity> = document
        .entities
        .iter()
        .filter(|e| ids.contains(&e.id()))
        .cloned()
        .collect();
    match tool {
        Tool::Rotate => {
            let angle = angle_degrees_from_points(base, hover);
            for entity in &mut preview {
                rotate_entity(entity, base, angle);
            }
        }
        Tool::Scale => {
            let reference = scale_reference.unwrap_or(Point {
                x: base.x + 1.0,
                y: base.y,
            });
            if let Some(factor) = scale_factor_from_points(base, reference, hover) {
                for entity in &mut preview {
                    scale_entity(entity, base, factor);
                }
            }
        }
        Tool::Mirror => {
            if let Some(start) = axis_start {
                for entity in &mut preview {
                    mirror_entity(entity, start, hover);
                }
            }
        }
        _ => {}
    }
    preview
}

pub fn handle_modify_tool_click(
    document: &mut Document,
    history: &Rc<RefCell<LegacyHistoryManager>>,
    tool: Tool,
    point: Point,
    selected: &[u64],
    pending: &Rc<RefCell<Option<Point>>>,
    pending_points: &Rc<RefCell<Vec<Point>>>,
) -> bool {
    let ids = editable_selection_ids(document, selected);
    if ids.is_empty() {
        return false;
    }
    match tool {
        Tool::Rotate => {
            if pending.borrow().is_none() {
                *pending.borrow_mut() = Some(point);
                return true;
            }
            let Some(base) = *pending.borrow() else {
                return false;
            };
            let angle = angle_degrees_from_points(base, point);
            record_entity_transform(&mut history.borrow_mut(), document, &ids, |entity| {
                rotate_entity(entity, base, angle);
            });
            *pending.borrow_mut() = None;
            true
        }
        Tool::Scale => {
            if pending.borrow().is_none() {
                *pending.borrow_mut() = Some(point);
                pending_points.borrow_mut().clear();
                pending_points.borrow_mut().push(Point {
                    x: point.x + 1.0,
                    y: point.y,
                });
                return true;
            }
            let Some(base) = *pending.borrow() else {
                return false;
            };
            let reference = pending_points.borrow().first().copied().unwrap_or(Point {
                x: base.x + 1.0,
                y: base.y,
            });
            let Some(factor) = scale_factor_from_points(base, reference, point) else {
                return false;
            };
            record_entity_transform(&mut history.borrow_mut(), document, &ids, |entity| {
                scale_entity(entity, base, factor);
            });
            *pending.borrow_mut() = None;
            pending_points.borrow_mut().clear();
            true
        }
        Tool::Mirror => {
            if pending.borrow().is_none() {
                *pending.borrow_mut() = Some(point);
                return true;
            }
            let Some(axis_start) = *pending.borrow() else {
                return false;
            };
            record_entity_transform(&mut history.borrow_mut(), document, &ids, |entity| {
                mirror_entity(entity, axis_start, point);
            });
            *pending.borrow_mut() = None;
            true
        }
        _ => false,
    }
}

pub fn apply_modify_command_rotate(
    document: &mut Document,
    history: &Rc<RefCell<LegacyHistoryManager>>,
    selected: &[u64],
    base: Point,
    angle_degrees: f64,
) -> bool {
    let ids = editable_selection_ids(document, selected);
    if ids.is_empty() {
        return false;
    }
    record_entity_transform(&mut history.borrow_mut(), document, &ids, |entity| {
        rotate_entity(entity, base, angle_degrees);
    });
    true
}

pub fn apply_modify_command_scale(
    document: &mut Document,
    history: &Rc<RefCell<LegacyHistoryManager>>,
    selected: &[u64],
    base: Point,
    factor: f64,
) -> bool {
    if factor <= 0.0 || !factor.is_finite() {
        return false;
    }
    let ids = editable_selection_ids(document, selected);
    if ids.is_empty() {
        return false;
    }
    record_entity_transform(&mut history.borrow_mut(), document, &ids, |entity| {
        scale_entity(entity, base, factor);
    });
    true
}

fn parse_command_point(value: &str) -> Option<Point> {
    let (x, y) = value.split_once(',')?;
    Some(Point {
        x: x.trim().parse().ok()?,
        y: y.trim().parse().ok()?,
    })
}

/// Command bar: `rotate 0,0 45`, `scale 0,0 2`, `mirror 0,0 100,0`.
pub fn try_execute_modify_command(
    command: &str,
    document: &mut Document,
    history: &Rc<RefCell<LegacyHistoryManager>>,
    selected: &[u64],
    status: Option<&gtk::Label>,
) -> bool {
    let set_status = |text: &str| {
        if let Some(label) = status {
            label.set_text(text);
        }
    };
    let parts: Vec<&str> = command.split_whitespace().collect();
    let Some(cmd) = parts.first().copied() else {
        return false;
    };
    match cmd {
        "rotate" | "ro" if parts.len() >= 3 => {
            let Some(base) = parse_command_point(parts[1]) else {
                set_status("ROTATE: invalid base point");
                return true;
            };
            let Ok(angle) = parts[2].parse::<f64>() else {
                set_status("ROTATE: invalid angle");
                return true;
            };
            if apply_modify_command_rotate(document, history, selected, base, angle) {
                set_status(&format!("ROTATE: {angle:.2}°"));
            } else {
                set_status("ROTATE: no editable selection");
            }
            true
        }
        "scale" | "sc" if parts.len() >= 3 => {
            let Some(base) = parse_command_point(parts[1]) else {
                set_status("SCALE: invalid base point");
                return true;
            };
            let Ok(factor) = parts[2].parse::<f64>() else {
                set_status("SCALE: invalid factor");
                return true;
            };
            if apply_modify_command_scale(document, history, selected, base, factor) {
                set_status(&format!("SCALE: factor {factor:.4}"));
            } else {
                set_status("SCALE: no editable selection");
            }
            true
        }
        "mirror" | "mi" if parts.len() >= 3 => {
            let Some(start) = parse_command_point(parts[1]) else {
                set_status("MIRROR: invalid axis start");
                return true;
            };
            let Some(end) = parse_command_point(parts[2]) else {
                set_status("MIRROR: invalid axis end");
                return true;
            };
            if apply_modify_command_mirror(document, history, selected, start, end) {
                set_status("MIRROR: applied");
            } else {
                set_status("MIRROR: no editable selection");
            }
            true
        }
        "move" | "m" if parts.len() >= 3 => {
            let Some(base) = parse_command_point(parts[1]) else {
                set_status("MOVE: invalid base point");
                return true;
            };
            let Some(target) = parse_command_point(parts[2]) else {
                set_status("MOVE: invalid target point");
                return true;
            };
            if apply_move_selection(document, &mut history.borrow_mut(), selected, base, target) {
                set_status("MOVE: selection moved");
            } else {
                set_status("MOVE: no movable selection");
            }
            true
        }
        "copy" | "co" | "cp" if parts.len() >= 3 => {
            let Some(base) = parse_command_point(parts[1]) else {
                set_status("COPY: invalid base point");
                return true;
            };
            let Some(target) = parse_command_point(parts[2]) else {
                set_status("COPY: invalid target point");
                return true;
            };
            let pasted =
                apply_copy_selection(document, &mut history.borrow_mut(), selected, base, target);
            if pasted.is_empty() {
                set_status("COPY: nothing to copy");
            } else {
                set_status(&format!("COPY: {} entities", pasted.len()));
            }
            true
        }
        _ => false,
    }
}

pub fn apply_modify_command_mirror(
    document: &mut Document,
    history: &Rc<RefCell<LegacyHistoryManager>>,
    selected: &[u64],
    axis_start: Point,
    axis_end: Point,
) -> bool {
    let ids = editable_selection_ids(document, selected);
    if ids.is_empty() {
        return false;
    }
    record_entity_transform(&mut history.borrow_mut(), document, &ids, |entity| {
        mirror_entity(entity, axis_start, axis_end);
    });
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::history::LegacyHistoryManager;
    use crate::document::Entity;

    #[test]
    fn locked_layer_entity_not_in_editable_selection() {
        let mut doc = Document::new_empty();
        doc.create_layer("Locked");
        doc.add_entity(Entity::Line {
            id: 1,
            layer: "Locked".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 1.0, y: 0.0 },
        });
        doc.set_layer_locked("Locked", true);
        let ids = editable_selection_ids(&doc, &[1]);
        assert!(ids.is_empty());
    }

    #[test]
    fn preview_rotate_produces_expected_geometry() {
        let mut doc = Document::new_empty();
        doc.add_entity(Entity::Line {
            id: 1,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        });
        let preview = build_modify_preview(
            &doc,
            &[1],
            Tool::Rotate,
            Point { x: 0.0, y: 0.0 },
            Point { x: 0.0, y: 10.0 },
            None,
            None,
        );
        assert_eq!(preview.len(), 1);
        let Entity::Line { end, .. } = &preview[0] else {
            panic!("line");
        };
        assert!((end.y - 10.0).abs() < 1e-6);
    }

    #[test]
    fn hidden_entity_not_editable() {
        let mut doc = Document::new_empty();
        doc.create_layer("Hidden");
        doc.set_layer_visible("Hidden", false);
        doc.add_entity(Entity::Line {
            id: 2,
            layer: "Hidden".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 1.0, y: 0.0 },
        });
        let ids = editable_selection_ids(&doc, &[2]);
        assert!(ids.is_empty());
    }

    #[test]
    fn command_parser_rotate() {
        let mut doc = Document::new_empty();
        doc.add_entity(Entity::Line {
            id: 1,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        });
        let history = Rc::new(RefCell::new(LegacyHistoryManager::new()));
        assert!(try_execute_modify_command(
            "rotate 0,0 90",
            &mut doc,
            &history,
            &[1],
            None,
        ));
        let Entity::Line { end, .. } = &doc.entities[0] else {
            panic!("line");
        };
        assert!((end.y - 10.0).abs() < 1e-6);
    }

    #[test]
    fn command_parser_scale_rejects_zero() {
        let mut doc = Document::new_empty();
        doc.add_entity(Entity::Line {
            id: 1,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        });
        let history = Rc::new(RefCell::new(LegacyHistoryManager::new()));
        try_execute_modify_command("scale 0,0 0", &mut doc, &history, &[1], None);
        let Entity::Line { end, .. } = &doc.entities[0] else {
            panic!("line");
        };
        assert!((end.x - 10.0).abs() < 1e-6);
    }

    #[test]
    fn command_parser_mirror() {
        let mut doc = Document::new_empty();
        doc.add_entity(Entity::Line {
            id: 1,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 2.0 },
            end: Point { x: 10.0, y: 2.0 },
        });
        let history = Rc::new(RefCell::new(LegacyHistoryManager::new()));
        try_execute_modify_command("mirror 0,0 100,0", &mut doc, &history, &[1], None);
        let Entity::Line { start, .. } = &doc.entities[0] else {
            panic!("line");
        };
        assert!((start.y + 2.0).abs() < 1e-6);
    }

    #[test]
    fn transform_action_undo_redo() {
        let mut doc = Document::new_empty();
        doc.add_entity(Entity::Line {
            id: 1,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        });
        let history = Rc::new(RefCell::new(LegacyHistoryManager::new()));
        apply_modify_command_rotate(&mut doc, &history, &[1], Point { x: 0.0, y: 0.0 }, 90.0);
        let Entity::Line { end, .. } = &doc.entities[0] else {
            panic!("line");
        };
        assert!((end.y - 10.0).abs() < 1e-6);
        assert!(history.borrow_mut().undo(&mut doc));
        let Entity::Line { end, .. } = &doc.entities[0] else {
            panic!("line");
        };
        assert!((end.y).abs() < 1e-6);
        assert!(history.borrow_mut().redo(&mut doc));
        let Entity::Line { end, .. } = &doc.entities[0] else {
            panic!("line");
        };
        assert!((end.y - 10.0).abs() < 1e-6);
    }

    #[test]
    fn move_action_undo_redo() {
        let mut doc = Document::new_empty();
        doc.add_entity(Entity::Line {
            id: 1,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        });
        let history = Rc::new(RefCell::new(LegacyHistoryManager::new()));
        assert!(apply_move_selection(
            &mut doc,
            &mut history.borrow_mut(),
            &[1],
            Point { x: 0.0, y: 0.0 },
            Point { x: 5.0, y: 0.0 },
        ));
        let Entity::Line { start, .. } = &doc.entities[0] else {
            panic!("line");
        };
        assert!((start.x - 5.0).abs() < 1e-6);
        assert!(history.borrow_mut().undo(&mut doc));
        let Entity::Line { start, .. } = &doc.entities[0] else {
            panic!("line");
        };
        assert!((start.x).abs() < 1e-6);
        assert!(history.borrow_mut().redo(&mut doc));
    }

    #[test]
    fn copy_action_undo_redo() {
        let mut doc = Document::new_empty();
        doc.add_entity(Entity::Line {
            id: 1,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        });
        let history = Rc::new(RefCell::new(LegacyHistoryManager::new()));
        let pasted = apply_copy_selection(
            &mut doc,
            &mut history.borrow_mut(),
            &[1],
            Point { x: 0.0, y: 0.0 },
            Point { x: 3.0, y: 0.0 },
        );
        assert_eq!(pasted.len(), 1);
        assert_eq!(doc.entities.len(), 2);
        assert!(history.borrow_mut().undo(&mut doc));
        assert_eq!(doc.entities.len(), 1);
        assert!(history.borrow_mut().redo(&mut doc));
        assert_eq!(doc.entities.len(), 2);
    }
}
