use crate::cad::document::CADDocument;
use crate::cad::entities::CADEntity;
use crate::cad::geometry::{BoundingBox2, Point2, Vector2};

#[derive(Clone, Debug, PartialEq)]
pub struct HitResult {
    pub entity_id: String,
    pub distance: f64,
}

/// Returns the closest entity under the pointer within tolerance (world units).
pub fn hit_entity_at(document: &CADDocument, point: Point2, tolerance: f64) -> Option<HitResult> {
    document
        .entities_in_active_space()
        .iter()
        .filter_map(|entity| {
            hit_distance(entity, point).map(|distance| HitResult {
                entity_id: entity.id().to_string(),
                distance,
            })
        })
        .filter(|hit| hit.distance <= tolerance)
        .min_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// Window/crossing selection using axis-aligned world box (caller defines crossing rules).
pub fn hit_entities_in_box(document: &CADDocument, box_bounds: BoundingBox2) -> Vec<String> {
    document
        .entities_in_active_space()
        .iter()
        .filter(|entity| {
            entity
                .bounds()
                .map(|bounds| bounds.intersects(&box_bounds))
                .unwrap_or(false)
        })
        .map(|entity| entity.id().to_string())
        .collect()
}

fn hit_distance(entity: &CADEntity, point: Point2) -> Option<f64> {
    match entity {
        CADEntity::Line(e) => Some(distance_to_segment(point, e.start, e.end)),
        CADEntity::Circle(e) => Some((point.distance_to(e.center) - e.radius).abs()),
        CADEntity::Polyline(e) => polyline_distance(point, &e.points, e.closed),
        CADEntity::Text(e) => Some(point.distance_to(e.origin)),
        CADEntity::Dimension(e) => Some(distance_to_segment(point, e.start, e.end)),
        CADEntity::Hatch(e) => polyline_distance(point, &e.boundary, true),
        CADEntity::BlockReference(e) => Some(point.distance_to(e.insertion)),
    }
}

fn distance_to_segment(point: Point2, start: Point2, end: Point2) -> f64 {
    let segment = Vector2::from_points(start, end);
    let length_sq = segment.length_squared();
    if length_sq <= f64::EPSILON {
        return point.distance_to(start);
    }
    let t = (Vector2::from_points(start, point).dot(segment) / length_sq).clamp(0.0, 1.0);
    let closest = start + segment * t;
    point.distance_to(closest)
}

fn polyline_distance(point: Point2, points: &[Point2], closed: bool) -> Option<f64> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::document::CADDocument;
    use crate::cad::entities::{BaseEntity, CADEntity, LineEntity};

    #[test]
    fn hit_line_entity() {
        let mut doc = CADDocument::new_empty();
        doc.add_entity(CADEntity::Line(LineEntity::new(
            BaseEntity::new("entity-1", "layer-default"),
            Point2::new(0.0, 0.0),
            Point2::new(10.0, 0.0),
        )));
        let hit = hit_entity_at(&doc, Point2::new(5.0, 0.0), 0.5).unwrap();
        assert_eq!(hit.entity_id, "entity-1");
    }
}
