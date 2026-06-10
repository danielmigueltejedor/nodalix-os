//! Bridge between legacy `Document` (u64 ids) and core hit-testing (String ids).
//!
// TODO(phase-2): cache `CADDocument` snapshot per frame to avoid full conversion on every pick.

use crate::cad::document::adapter::from_legacy_document;
use crate::cad::geometry::Point2;
use crate::cad::selection::hit_testing::hit_entity_at;
use crate::document::{Document, Entity};
use crate::geometry::Point;

/// Hit-test using the CAD core for adapter-supported entities, plus legacy geometry
/// for entity kinds not yet mapped in `CADEntity`.
pub fn legacy_hit_entity_at(document: &Document, point: Point, tolerance: f64) -> Option<u64> {
    let point2 = Point2::new(point.x, point.y);
    let mut best: Option<(u64, f64)> = None;

    let cad = from_legacy_document(document);
    if let Some(hit) = hit_entity_at(&cad, point2, tolerance) {
        if let Some(id) = parse_legacy_entity_id(&hit.entity_id) {
            if document.entity_visible_in_active_layout(id) {
                best = Some((id, hit.distance));
            }
        }
    }

    let query_bounds = (
        Point {
            x: point.x - tolerance,
            y: point.y - tolerance,
        },
        Point {
            x: point.x + tolerance,
            y: point.y + tolerance,
        },
    );

    for entity in document.entities.iter() {
        if !document.entity_visible_in_active_layout(entity.id()) {
            continue;
        }
        if matches!(
            entity,
            Entity::Line { .. } | Entity::Circle { .. } | Entity::Polyline { .. }
        ) {
            continue;
        }
        if document
            .cached_entity_bounds(entity)
            .map(|bounds| !bounds_intersect(bounds, query_bounds))
            .unwrap_or(false)
        {
            continue;
        }
        if let Some(distance) = legacy_hit_distance(document, entity, point) {
            if distance <= tolerance {
                let id = entity.id();
                if best.as_ref().map(|(_, d)| distance < *d).unwrap_or(true) {
                    best = Some((id, distance));
                }
            }
        }
    }

    best.map(|(id, _)| id)
}

fn parse_legacy_entity_id(entity_id: &str) -> Option<u64> {
    entity_id
        .strip_prefix("entity-")
        .and_then(|value| value.parse().ok())
}

fn legacy_hit_distance(document: &Document, entity: &Entity, point: Point) -> Option<f64> {
    match entity {
        Entity::Point { point: p, .. } => Some(point.distance_to(*p)),
        Entity::Dimension { start, end, .. } | Entity::Guideline { start, end, .. } => {
            Some(distance_to_segment(point, *start, *end))
        }
        Entity::Polyline { points, closed, .. } => polyline_distance(point, points, *closed),
        Entity::Spline {
            control_points,
            closed,
            ..
        } => polyline_distance(point, control_points, *closed),
        Entity::Text { origin, .. } | Entity::Table { origin, .. } => {
            Some(point.distance_to(*origin))
        }
        Entity::BlockReference {
            name,
            insertion,
            scale,
            scale_y,
            rotation,
            ..
        } => crate::cad::geometry::blocks::block_reference_hit_distance(
            document, name, *insertion, *scale, *scale_y, *rotation, point,
        ),
        Entity::Hatch { boundary, .. } => polyline_distance(point, boundary, true),
        Entity::Line { .. } | Entity::Circle { .. } => None,
    }
}

fn distance_to_segment(point: Point, start: Point, end: Point) -> f64 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_sq = dx * dx + dy * dy;
    if length_sq <= f64::EPSILON {
        return point.distance_to(start);
    }
    let t = ((point.x - start.x) * dx + (point.y - start.y) * dy) / length_sq;
    let t = t.clamp(0.0, 1.0);
    let closest = Point {
        x: start.x + t * dx,
        y: start.y + t * dy,
    };
    point.distance_to(closest)
}

fn polyline_distance(point: Point, points: &[Point], closed: bool) -> Option<f64> {
    if points.is_empty() {
        return None;
    }
    let mut best = f64::INFINITY;
    for pair in points.windows(2) {
        best = best.min(distance_to_segment(point, pair[0], pair[1]));
    }
    if closed && points.len() > 2 {
        let last = *points.last().unwrap();
        best = best.min(distance_to_segment(point, last, points[0]));
    }
    for vertex in points {
        best = best.min(point.distance_to(*vertex));
    }
    best.is_finite().then_some(best)
}

fn bounds_intersect(a: (Point, Point), b: (Point, Point)) -> bool {
    a.0.x <= b.1.x && a.1.x >= b.0.x && a.0.y <= b.1.y && a.1.y >= b.0.y
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Entity;

    #[test]
    fn delegates_line_hit_to_core() {
        let mut document = Document::new_empty();
        document.add_entity(Entity::Line {
            id: 42,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        });
        let hit = legacy_hit_entity_at(&document, Point { x: 5.0, y: 0.0 }, 0.5);
        assert_eq!(hit, Some(42));
    }

    #[test]
    fn falls_back_for_text_entity() {
        let mut document = Document::new_empty();
        document.add_entity(Entity::Text {
            id: 7,
            layer: "Default".to_string(),
            origin: Point { x: 2.0, y: 3.0 },
            text: "A".to_string(),
            height: 2.5,
            rotation: 0.0,
        });
        let hit = legacy_hit_entity_at(&document, Point { x: 2.0, y: 3.0 }, 0.5);
        assert_eq!(hit, Some(7));
    }
}
