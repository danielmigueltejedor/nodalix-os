//! Selection grip points and pure geometry edits for grip dragging.

use crate::{document::Entity, geometry::Point};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GripKind {
    LineStart,
    LineEnd,
    LineMid,
    CircleCenter,
    CircleRadius,
    PolylineVertex(usize),
    TextInsertion,
    DimensionStart,
    DimensionEnd,
}

#[derive(Clone, Copy, Debug)]
pub struct Grip {
    pub entity_id: u64,
    pub kind: GripKind,
    pub position: Point,
}

pub fn collect_grips(entity: &Entity) -> Vec<Grip> {
    let id = entity.id();
    match entity {
        Entity::Line { start, end, .. } => {
            let mid = Point {
                x: (start.x + end.x) * 0.5,
                y: (start.y + end.y) * 0.5,
            };
            vec![
                Grip {
                    entity_id: id,
                    kind: GripKind::LineStart,
                    position: *start,
                },
                Grip {
                    entity_id: id,
                    kind: GripKind::LineMid,
                    position: mid,
                },
                Grip {
                    entity_id: id,
                    kind: GripKind::LineEnd,
                    position: *end,
                },
            ]
        }
        Entity::Circle { center, radius, .. } => {
            let radius_point = Point {
                x: center.x + radius,
                y: center.y,
            };
            vec![
                Grip {
                    entity_id: id,
                    kind: GripKind::CircleCenter,
                    position: *center,
                },
                Grip {
                    entity_id: id,
                    kind: GripKind::CircleRadius,
                    position: radius_point,
                },
            ]
        }
        Entity::Polyline { points, .. } => points
            .iter()
            .enumerate()
            .map(|(index, point)| Grip {
                entity_id: id,
                kind: GripKind::PolylineVertex(index),
                position: *point,
            })
            .collect(),
        Entity::Text { origin, .. } => vec![Grip {
            entity_id: id,
            kind: GripKind::TextInsertion,
            position: *origin,
        }],
        Entity::Dimension { start, end, .. } => vec![
            Grip {
                entity_id: id,
                kind: GripKind::DimensionStart,
                position: *start,
            },
            Grip {
                entity_id: id,
                kind: GripKind::DimensionEnd,
                position: *end,
            },
        ],
        _ => Vec::new(),
    }
}

pub fn grip_is_editable(kind: GripKind) -> bool {
    !matches!(
        kind,
        GripKind::PolylineVertex(_) | GripKind::DimensionStart | GripKind::DimensionEnd
    )
}

pub fn hit_test_grip(grips: &[Grip], point: Point, tolerance: f64) -> Option<Grip> {
    let mut best: Option<(Grip, f64)> = None;
    for grip in grips {
        let distance = grip.position.distance_to(point);
        if distance <= tolerance {
            if best.as_ref().map(|(_, d)| distance < *d).unwrap_or(true) {
                best = Some((*grip, distance));
            }
        }
    }
    best.map(|(grip, _)| grip)
}

pub fn apply_grip_edit(entity: &mut Entity, kind: GripKind, cursor: Point, anchor: Point) {
    match (entity, kind) {
        (Entity::Line { start, end, .. }, GripKind::LineStart) => *start = cursor,
        (Entity::Line { start, end, .. }, GripKind::LineEnd) => *end = cursor,
        (Entity::Line { start, end, .. }, GripKind::LineMid) => {
            let dx = cursor.x - anchor.x;
            let dy = cursor.y - anchor.y;
            start.x += dx;
            start.y += dy;
            end.x += dx;
            end.y += dy;
        }
        (Entity::Circle { center, radius, .. }, GripKind::CircleCenter) => {
            let dx = cursor.x - anchor.x;
            let dy = cursor.y - anchor.y;
            center.x += dx;
            center.y += dy;
        }
        (Entity::Circle { center, radius, .. }, GripKind::CircleRadius) => {
            let next = center.distance_to(cursor);
            if next.is_finite() && next > 0.0 {
                *radius = next;
            }
        }
        (Entity::Text { origin, .. }, GripKind::TextInsertion) => *origin = cursor,
        _ => {}
    }
}

pub fn translate_entities(entities: &mut [Entity], dx: f64, dy: f64) {
    for entity in entities {
        entity.translate(dx, dy);
    }
}

pub fn clone_entities_offset(entities: &[Entity], dx: f64, dy: f64) -> Vec<Entity> {
    entities
        .iter()
        .map(|entity| {
            let mut copy = entity.clone();
            copy.translate(dx, dy);
            copy
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cad::history::{capture_entity_snapshot, record_entity_transform, LegacyHistoryManager},
        document::Document,
    };

    fn sample_line(doc: &mut Document) -> u64 {
        let id = doc.next_id();
        doc.add_entity(Entity::Line {
            id,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        });
        id
    }

    #[test]
    fn line_has_three_grips() {
        let mut doc = Document::new_empty();
        let id = sample_line(&mut doc);
        let entity = doc.entities.iter().find(|e| e.id() == id).unwrap();
        assert_eq!(collect_grips(entity).len(), 3);
    }

    #[test]
    fn grip_line_start_edit() {
        let mut entity = Entity::Line {
            id: 1,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        };
        apply_grip_edit(
            &mut entity,
            GripKind::LineStart,
            Point { x: 2.0, y: 3.0 },
            Point::default(),
        );
        if let Entity::Line { start, .. } = entity {
            assert!((start.x - 2.0).abs() < 1e-9);
            assert!((start.y - 3.0).abs() < 1e-9);
        } else {
            panic!("expected line");
        }
    }

    #[test]
    fn grip_line_end_edit() {
        let mut entity = Entity::Line {
            id: 1,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        };
        apply_grip_edit(
            &mut entity,
            GripKind::LineEnd,
            Point { x: 5.0, y: 1.0 },
            Point::default(),
        );
        if let Entity::Line { end, .. } = entity {
            assert!((end.x - 5.0).abs() < 1e-9);
            assert!((end.y - 1.0).abs() < 1e-9);
        } else {
            panic!("expected line");
        }
    }

    #[test]
    fn grip_line_mid_moves_whole_line() {
        let mut entity = Entity::Line {
            id: 1,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        };
        apply_grip_edit(
            &mut entity,
            GripKind::LineMid,
            Point { x: 6.0, y: 4.0 },
            Point { x: 5.0, y: 4.0 },
        );
        if let Entity::Line { start, end, .. } = entity {
            assert!((start.x - 1.0).abs() < 1e-9);
            assert!((end.x - 11.0).abs() < 1e-9);
        } else {
            panic!("expected line");
        }
    }

    #[test]
    fn grip_circle_center_moves() {
        let mut entity = Entity::Circle {
            id: 1,
            layer: "Default".to_string(),
            center: Point { x: 0.0, y: 0.0 },
            radius: 5.0,
        };
        apply_grip_edit(
            &mut entity,
            GripKind::CircleCenter,
            Point { x: 3.0, y: 4.0 },
            Point { x: 0.0, y: 0.0 },
        );
        if let Entity::Circle { center, radius, .. } = entity {
            assert!((center.x - 3.0).abs() < 1e-9);
            assert!((radius - 5.0).abs() < 1e-9);
        } else {
            panic!("expected circle");
        }
    }

    #[test]
    fn grip_circle_radius_edit() {
        let mut entity = Entity::Circle {
            id: 1,
            layer: "Default".to_string(),
            center: Point { x: 0.0, y: 0.0 },
            radius: 5.0,
        };
        apply_grip_edit(
            &mut entity,
            GripKind::CircleRadius,
            Point { x: 8.0, y: 0.0 },
            Point::default(),
        );
        if let Entity::Circle { radius, .. } = entity {
            assert!((radius - 8.0).abs() < 1e-9);
        } else {
            panic!("expected circle");
        }
    }

    #[test]
    fn grip_text_insertion_moves() {
        let mut entity = Entity::Text {
            id: 1,
            layer: "Default".to_string(),
            origin: Point { x: 1.0, y: 2.0 },
            text: "A".to_string(),
            height: 2.5,
            rotation: 0.0,
        };
        apply_grip_edit(
            &mut entity,
            GripKind::TextInsertion,
            Point { x: 9.0, y: 8.0 },
            Point::default(),
        );
        if let Entity::Text { origin, .. } = entity {
            assert!((origin.x - 9.0).abs() < 1e-9);
        } else {
            panic!("expected text");
        }
    }

    #[test]
    fn grip_action_undo_redo() {
        let mut doc = Document::new_empty();
        let id = sample_line(&mut doc);
        let mut history = LegacyHistoryManager::new();
        let before = capture_entity_snapshot(&doc, id).unwrap();
        record_entity_transform(&mut history, &mut doc, &[id], |entity| {
            apply_grip_edit(
                entity,
                GripKind::LineEnd,
                Point { x: 20.0, y: 0.0 },
                Point::default(),
            );
        });
        if let Entity::Line { end, .. } = doc.entities.iter().find(|e| e.id() == id).unwrap() {
            assert!((end.x - 20.0).abs() < 1e-9);
        }
        assert!(history.undo(&mut doc));
        if let Entity::Line { end, .. } = doc.entities.iter().find(|e| e.id() == id).unwrap() {
            assert!((end.x - 10.0).abs() < 1e-9);
        }
        assert!(history.redo(&mut doc));
        if let Entity::Line { end, .. } = doc.entities.iter().find(|e| e.id() == id).unwrap() {
            assert!((end.x - 20.0).abs() < 1e-9);
        }
        let _ = before;
    }

    #[test]
    fn move_line_by_vector() {
        let mut doc = Document::new_empty();
        let id = sample_line(&mut doc);
        doc.translate_entity(id, 5.0, 2.0);
        if let Entity::Line { start, .. } = doc.entities.iter().find(|e| e.id() == id).unwrap() {
            assert!((start.x - 5.0).abs() < 1e-9);
            assert!((start.y - 2.0).abs() < 1e-9);
        }
    }

    #[test]
    fn copy_line_preserves_original() {
        let mut doc = Document::new_empty();
        let id = sample_line(&mut doc);
        let copies = doc.paste_entities_translated(&doc.entities_by_ids(&[id]), 3.0, 0.0);
        assert_eq!(doc.entities.len(), 2);
        assert_ne!(copies[0], id);
        if let Entity::Line { start, .. } = doc.entities.iter().find(|e| e.id() == id).unwrap() {
            assert!((start.x - 0.0).abs() < 1e-9);
        }
    }

    #[test]
    fn copy_preserves_layer_and_style_maps() {
        let mut doc = Document::new_empty();
        let id = sample_line(&mut doc);
        doc.entity_colors.insert(id, "#ff0000".to_string());
        doc.entity_line_weights.insert(id, 0.5);
        let copies = doc.paste_entities_translated(&doc.entities_by_ids(&[id]), 1.0, 1.0);
        let copy_id = copies[0];
        assert_eq!(
            doc.entity_colors.get(&copy_id),
            Some(&"#ff0000".to_string())
        );
        assert_eq!(doc.entity_line_weights.get(&copy_id), Some(&0.5));
    }

    #[test]
    fn locked_entity_cannot_translate() {
        let mut doc = Document::new_empty();
        doc.create_layer("Locked");
        doc.set_layer_locked("Locked", true);
        let id = doc.next_id();
        doc.add_entity(Entity::Line {
            id,
            layer: "Locked".to_string(),
            start: Point::default(),
            end: Point { x: 1.0, y: 0.0 },
        });
        doc.translate_entity(id, 5.0, 0.0);
        if let Entity::Line { start, .. } = doc.entities.iter().find(|e| e.id() == id).unwrap() {
            assert!((start.x - 0.0).abs() < 1e-9);
        }
    }

    #[test]
    fn locked_entity_can_still_be_copied() {
        let mut doc = Document::new_empty();
        doc.create_layer("Locked");
        doc.set_layer_locked("Locked", true);
        let id = doc.next_id();
        doc.add_entity(Entity::Line {
            id,
            layer: "Locked".to_string(),
            start: Point::default(),
            end: Point { x: 1.0, y: 0.0 },
        });
        let copies = doc.paste_entities_translated(&doc.entities_by_ids(&[id]), 2.0, 0.0);
        assert_eq!(copies.len(), 1);
        assert_eq!(doc.entities.len(), 2);
    }

    #[test]
    fn hidden_entity_has_no_grips_when_filtered() {
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
        assert!(!doc.entity_visible_in_active_layout(id));
    }
}
