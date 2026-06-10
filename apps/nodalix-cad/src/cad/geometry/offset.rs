//! Parallel / concentric offset helpers for lines, circles, and simple polylines.

use crate::{document::Entity, geometry::Point};

const EPS: f64 = 1e-9;

fn shoelace_area(points: &[Point]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut sum = 0.0;
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        sum += points[index].x * points[next].y - points[next].x * points[index].y;
    }
    sum.abs() * 0.5
}

fn signed_area(points: &[Point]) -> f64 {
    let mut sum = 0.0;
    for index in 0..points.len() {
        let next = (index + 1) % points.len();
        sum += points[index].x * points[next].y - points[next].x * points[index].y;
    }
    sum * 0.5
}

fn point_in_polygon(point: Point, polygon: &[Point]) -> bool {
    let mut inside = false;
    for index in 0..polygon.len() {
        let next = (index + 1) % polygon.len();
        let pi = polygon[index];
        let pj = polygon[next];
        let intersects = (pi.y > point.y) != (pj.y > point.y)
            && point.x < (pj.x - pi.x) * (point.y - pi.y) / (pj.y - pi.y + EPS) + pi.x;
        if intersects {
            inside = !inside;
        }
    }
    inside
}

fn offset_line_winding(
    start: Point,
    end: Point,
    distance: f64,
    ccw: bool,
    expand: f64,
) -> Option<(Point, Point)> {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < EPS {
        return None;
    }
    let mut nx = -dy / len;
    let mut ny = dx / len;
    if !ccw {
        nx = -nx;
        ny = -ny;
    }
    let shift_x = nx * distance * expand;
    let shift_y = ny * distance * expand;
    Some((
        Point {
            x: start.x + shift_x,
            y: start.y + shift_y,
        },
        Point {
            x: end.x + shift_x,
            y: end.y + shift_y,
        },
    ))
}

/// Signed offset side from `side_point` relative to directed segment `start` → `end`.
/// Positive = left of direction, negative = right.
pub fn line_offset_sign(start: Point, end: Point, side_point: Point) -> f64 {
    let cross =
        (end.x - start.x) * (side_point.y - start.y) - (end.y - start.y) * (side_point.x - start.x);
    if cross.abs() < EPS {
        1.0
    } else if cross > 0.0 {
        1.0
    } else {
        -1.0
    }
}

/// Offset a line by `distance` to the side indicated by `side_point`.
pub fn offset_line(
    start: Point,
    end: Point,
    distance: f64,
    side_point: Point,
) -> Option<(Point, Point)> {
    if distance <= 0.0 || !distance.is_finite() {
        return None;
    }
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < EPS {
        return None;
    }
    let sign = line_offset_sign(start, end, side_point);
    let nx = -dy / len * distance * sign;
    let ny = dx / len * distance * sign;
    Some((
        Point {
            x: start.x + nx,
            y: start.y + ny,
        },
        Point {
            x: end.x + nx,
            y: end.y + ny,
        },
    ))
}

/// Concentric offset: outside → larger radius, inside → smaller.
pub fn offset_circle(
    center: Point,
    radius: f64,
    distance: f64,
    side_point: Point,
) -> Option<(Point, f64)> {
    if distance <= 0.0 || !distance.is_finite() || radius <= 0.0 || !radius.is_finite() {
        return None;
    }
    let dist_to_side = center.distance_to(side_point);
    let new_radius = if dist_to_side > radius {
        radius + distance
    } else {
        radius - distance
    };
    if new_radius <= EPS || !new_radius.is_finite() {
        return None;
    }
    Some((center, new_radius))
}

fn intersect_rays(origin_a: Point, dir_a: Point, origin_b: Point, dir_b: Point) -> Option<Point> {
    let cross = dir_a.x * dir_b.y - dir_a.y * dir_b.x;
    if cross.abs() < EPS {
        return None;
    }
    let wx = origin_b.x - origin_a.x;
    let wy = origin_b.y - origin_a.y;
    let t = (wx * dir_b.y - wy * dir_b.x) / cross;
    Some(Point {
        x: origin_a.x + t * dir_a.x,
        y: origin_a.y + t * dir_a.y,
    })
}

struct OffsetSegment {
    start: Point,
    end: Point,
    direction: Point,
}

/// Offset an open or closed polyline; returns `None` on degenerate or failed joins.
pub fn offset_polyline(
    points: &[Point],
    closed: bool,
    distance: f64,
    side_point: Point,
) -> Option<Vec<Point>> {
    if points.len() < 2 || distance <= 0.0 || !distance.is_finite() {
        return None;
    }
    let segment_count = if closed {
        points.len()
    } else {
        points.len() - 1
    };
    if segment_count < 1 {
        return None;
    }

    let mut segments = Vec::with_capacity(segment_count);
    let closed_expand = if closed {
        if point_in_polygon(side_point, points) {
            -1.0
        } else {
            1.0
        }
    } else {
        1.0
    };
    let ccw = signed_area(points) >= 0.0;
    for index in 0..segment_count {
        let start = points[index];
        let end = points[(index + 1) % points.len()];
        let (o_start, o_end) = if closed {
            offset_line_winding(start, end, distance, ccw, closed_expand)?
        } else {
            offset_line(start, end, distance, side_point)?
        };
        let direction = Point {
            x: end.x - start.x,
            y: end.y - start.y,
        };
        segments.push(OffsetSegment {
            start: o_start,
            end: o_end,
            direction,
        });
    }

    if closed {
        let mut result = Vec::with_capacity(segment_count);
        for index in 0..segment_count {
            let prev = (index + segment_count - 1) % segment_count;
            let corner = intersect_rays(
                segments[prev].start,
                segments[prev].direction,
                segments[index].start,
                segments[index].direction,
            )?;
            result.push(corner);
        }
        return Some(result);
    }

    let mut result = Vec::with_capacity(points.len());
    result.push(segments[0].start);
    for index in 1..segment_count {
        let corner = intersect_rays(
            segments[index - 1].start,
            segments[index - 1].direction,
            segments[index].start,
            segments[index].direction,
        )?;
        result.push(corner);
    }
    result.push(segments[segment_count - 1].end);
    Some(result)
}

pub fn entity_supports_offset(entity: &Entity) -> bool {
    matches!(
        entity,
        Entity::Line { .. } | Entity::Circle { .. } | Entity::Polyline { .. }
    )
}

/// Build offset geometry (layer/id unset); returns `None` if unsupported or invalid.
pub fn offset_entity_geometry(entity: &Entity, distance: f64, side_point: Point) -> Option<Entity> {
    if distance <= 0.0 || !distance.is_finite() {
        return None;
    }
    match entity {
        Entity::Line {
            start, end, layer, ..
        } => {
            let (s, e) = offset_line(*start, *end, distance, side_point)?;
            Some(Entity::Line {
                id: 0,
                layer: layer.clone(),
                start: s,
                end: e,
            })
        }
        Entity::Circle {
            center,
            radius,
            layer,
            ..
        } => {
            let (c, r) = offset_circle(*center, *radius, distance, side_point)?;
            Some(Entity::Circle {
                id: 0,
                layer: layer.clone(),
                center: c,
                radius: r,
            })
        }
        Entity::Polyline {
            points,
            closed,
            layer,
            ..
        } => {
            let new_points = offset_polyline(points, *closed, distance, side_point)?;
            if new_points.len() < 2 {
                return None;
            }
            Some(Entity::Polyline {
                id: 0,
                layer: layer.clone(),
                points: new_points,
                closed: *closed,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    #[test]
    fn offset_line_horizontal_upward() {
        let (s, e) = offset_line(
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            2.0,
            Point { x: 5.0, y: 3.0 },
        )
        .unwrap();
        assert!((s.y - 2.0).abs() < 1e-6);
        assert!((e.y - 2.0).abs() < 1e-6);
        assert!((s.x - 0.0).abs() < 1e-6);
    }

    #[test]
    fn offset_line_horizontal_downward() {
        let (s, e) = offset_line(
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            2.0,
            Point { x: 5.0, y: -4.0 },
        )
        .unwrap();
        assert!((s.y + 2.0).abs() < 1e-6);
        assert!((e.y + 2.0).abs() < 1e-6);
    }

    #[test]
    fn offset_line_vertical() {
        let (s, e) = offset_line(
            Point { x: 0.0, y: 0.0 },
            Point { x: 0.0, y: 10.0 },
            3.0,
            Point { x: 5.0, y: 5.0 },
        )
        .unwrap();
        assert!((s.x - 3.0).abs() < 1e-6);
        assert!((e.x - 3.0).abs() < 1e-6);
    }

    #[test]
    fn offset_line_rejects_zero_length() {
        assert!(offset_line(
            Point { x: 1.0, y: 1.0 },
            Point { x: 1.0, y: 1.0 },
            5.0,
            Point { x: 2.0, y: 2.0 },
        )
        .is_none());
    }

    #[test]
    fn offset_circle_outside_increases_radius() {
        let (_, r) = offset_circle(
            Point { x: 0.0, y: 0.0 },
            5.0,
            2.0,
            Point { x: 10.0, y: 0.0 },
        )
        .unwrap();
        assert!((r - 7.0).abs() < 1e-6);
    }

    #[test]
    fn offset_circle_inside_decreases_radius() {
        let (_, r) = offset_circle(
            Point { x: 0.0, y: 0.0 },
            10.0,
            3.0,
            Point { x: 2.0, y: 0.0 },
        )
        .unwrap();
        assert!((r - 7.0).abs() < 1e-6);
    }

    #[test]
    fn offset_circle_rejects_non_positive_radius() {
        assert!(offset_circle(Point::default(), 2.0, 5.0, Point { x: 0.0, y: 0.0 },).is_none());
    }

    #[test]
    fn offset_polyline_open_simple() {
        let points = vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Point { x: 10.0, y: 10.0 },
        ];
        let out = offset_polyline(&points, false, 1.0, Point { x: 5.0, y: 2.0 }).unwrap();
        assert_eq!(out.len(), 3);
        assert!((out[0].y - 1.0).abs() < 1e-5);
    }

    #[test]
    fn offset_polyline_closed_rectangle() {
        let points = vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Point { x: 10.0, y: 5.0 },
            Point { x: 0.0, y: 5.0 },
        ];
        let out = offset_polyline(&points, true, 1.0, Point { x: 5.0, y: -2.0 }).unwrap();
        assert_eq!(out.len(), 4);
        for (original, offset) in points.iter().zip(out.iter()) {
            assert!(original.distance_to(*offset) > 0.5);
        }
        assert!(offset_polyline(&points, true, 1.0, Point { x: 5.0, y: 2.0 }).is_some());
    }

    #[test]
    fn locked_source_can_offset() {
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
        let entity = doc.entities.iter().find(|e| e.id() == id).unwrap();
        assert!(offset_entity_geometry(entity, 2.0, Point { x: 5.0, y: 3.0 }).is_some());
    }

    #[test]
    fn hidden_source_not_visible() {
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
