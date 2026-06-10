//! Pure 2D modify transforms for legacy `Entity` geometry.

use crate::{document::Entity, geometry::Point};

pub fn rotate_point(point: Point, base: Point, angle_degrees: f64) -> Point {
    let rad = angle_degrees.to_radians();
    let cos = rad.cos();
    let sin = rad.sin();
    let dx = point.x - base.x;
    let dy = point.y - base.y;
    Point {
        x: base.x + dx * cos - dy * sin,
        y: base.y + dy * cos + dx * sin,
    }
}

pub fn scale_point(point: Point, base: Point, factor: f64) -> Point {
    Point {
        x: base.x + (point.x - base.x) * factor,
        y: base.y + (point.y - base.y) * factor,
    }
}

pub fn mirror_point(point: Point, axis_start: Point, axis_end: Point) -> Point {
    let dx = axis_end.x - axis_start.x;
    let dy = axis_end.y - axis_start.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq <= f64::EPSILON {
        return point;
    }
    let vx = point.x - axis_start.x;
    let vy = point.y - axis_start.y;
    let t = (vx * dx + vy * dy) / len_sq;
    let proj_x = axis_start.x + t * dx;
    let proj_y = axis_start.y + t * dy;
    Point {
        x: 2.0 * proj_x - point.x,
        y: 2.0 * proj_y - point.y,
    }
}

pub fn angle_degrees_from_points(base: Point, target: Point) -> f64 {
    let dx = target.x - base.x;
    let dy = target.y - base.y;
    let mut angle = dy.atan2(dx).to_degrees();
    if angle < 0.0 {
        angle += 360.0;
    }
    angle
}

pub fn scale_factor_from_points(base: Point, reference: Point, cursor: Point) -> Option<f64> {
    let ref_dist = base.distance_to(reference);
    if ref_dist <= 1e-9 {
        return None;
    }
    let factor = base.distance_to(cursor) / ref_dist;
    if factor.is_finite() && factor > 0.0 {
        Some(factor)
    } else {
        None
    }
}

pub fn rotate_entity(entity: &mut Entity, base: Point, angle_degrees: f64) {
    let rad = angle_degrees.to_radians();
    match entity {
        Entity::Point { point, .. } => *point = rotate_point(*point, base, angle_degrees),
        Entity::Line { start, end, .. }
        | Entity::Dimension { start, end, .. }
        | Entity::Guideline { start, end, .. } => {
            *start = rotate_point(*start, base, angle_degrees);
            *end = rotate_point(*end, base, angle_degrees);
        }
        Entity::Polyline { points, .. }
        | Entity::Spline {
            control_points: points,
            ..
        }
        | Entity::Hatch {
            boundary: points, ..
        } => {
            for p in points {
                *p = rotate_point(*p, base, angle_degrees);
            }
        }
        Entity::Circle { center, .. } => *center = rotate_point(*center, base, angle_degrees),
        Entity::Text {
            origin, rotation, ..
        } => {
            *origin = rotate_point(*origin, base, angle_degrees);
            *rotation += angle_degrees;
        }
        Entity::Table { origin, .. } => *origin = rotate_point(*origin, base, angle_degrees),
        Entity::BlockReference {
            insertion,
            rotation,
            ..
        } => {
            *insertion = rotate_point(*insertion, base, angle_degrees);
            *rotation += rad;
        }
    }
}

pub fn scale_entity(entity: &mut Entity, base: Point, factor: f64) {
    if factor <= 0.0 || !factor.is_finite() {
        return;
    }
    match entity {
        Entity::Point { point, .. } => *point = scale_point(*point, base, factor),
        Entity::Line { start, end, .. }
        | Entity::Dimension { start, end, .. }
        | Entity::Guideline { start, end, .. } => {
            *start = scale_point(*start, base, factor);
            *end = scale_point(*end, base, factor);
        }
        Entity::Polyline { points, .. }
        | Entity::Spline {
            control_points: points,
            ..
        } => {
            for p in points {
                *p = scale_point(*p, base, factor);
            }
        }
        Entity::Hatch {
            boundary: points,
            scale,
            ..
        } => {
            for p in points {
                *p = scale_point(*p, base, factor);
            }
            *scale *= factor;
        }
        Entity::Circle { center, radius, .. } => {
            *center = scale_point(*center, base, factor);
            *radius *= factor;
        }
        Entity::Text { origin, height, .. } => {
            *origin = scale_point(*origin, base, factor);
            *height *= factor;
        }
        Entity::Table {
            origin,
            cell_width,
            cell_height,
            ..
        } => {
            *origin = scale_point(*origin, base, factor);
            *cell_width *= factor;
            *cell_height *= factor;
        }
        Entity::BlockReference {
            insertion,
            scale,
            scale_y,
            ..
        } => {
            *insertion = scale_point(*insertion, base, factor);
            *scale *= factor;
            if let Some(scale_y) = scale_y {
                *scale_y *= factor;
            }
        }
    }
}

pub fn mirror_entity(entity: &mut Entity, axis_start: Point, axis_end: Point) {
    let axis_angle = angle_degrees_from_points(axis_start, axis_end);
    match entity {
        Entity::Point { point, .. } => *point = mirror_point(*point, axis_start, axis_end),
        Entity::Line { start, end, .. }
        | Entity::Dimension { start, end, .. }
        | Entity::Guideline { start, end, .. } => {
            *start = mirror_point(*start, axis_start, axis_end);
            *end = mirror_point(*end, axis_start, axis_end);
        }
        Entity::Polyline { points, .. }
        | Entity::Spline {
            control_points: points,
            ..
        }
        | Entity::Hatch {
            boundary: points, ..
        } => {
            for p in points {
                *p = mirror_point(*p, axis_start, axis_end);
            }
        }
        Entity::Circle { center, .. } => *center = mirror_point(*center, axis_start, axis_end),
        Entity::Text {
            origin, rotation, ..
        } => {
            *origin = mirror_point(*origin, axis_start, axis_end);
            *rotation = 2.0 * axis_angle - *rotation;
        }
        Entity::Table { origin, .. } => *origin = mirror_point(*origin, axis_start, axis_end),
        Entity::BlockReference {
            insertion,
            rotation,
            ..
        } => {
            *insertion = mirror_point(*insertion, axis_start, axis_end);
            let rot_deg = rotation.to_degrees();
            *rotation = (2.0 * axis_angle - rot_deg).to_radians();
        }
    }
}

pub fn transform_entities_rotate(entities: &mut [Entity], base: Point, angle_degrees: f64) {
    for entity in entities {
        rotate_entity(entity, base, angle_degrees);
    }
}

pub fn transform_entities_scale(entities: &mut [Entity], base: Point, factor: f64) {
    for entity in entities {
        scale_entity(entity, base, factor);
    }
}

pub fn transform_entities_mirror(entities: &mut [Entity], axis_start: Point, axis_end: Point) {
    for entity in entities {
        mirror_entity(entity, axis_start, axis_end);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotate_point_90_degrees() {
        let base = Point { x: 0.0, y: 0.0 };
        let p = rotate_point(Point { x: 10.0, y: 0.0 }, base, 90.0);
        assert!((p.x - 0.0).abs() < 1e-6);
        assert!((p.y - 10.0).abs() < 1e-6);
    }

    #[test]
    fn rotate_line_endpoints() {
        let mut entity = Entity::Line {
            id: 1,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        };
        rotate_entity(&mut entity, Point { x: 0.0, y: 0.0 }, 90.0);
        let Entity::Line { end, .. } = entity else {
            panic!("line");
        };
        assert!((end.x - 0.0).abs() < 1e-6);
        assert!((end.y - 10.0).abs() < 1e-6);
    }

    #[test]
    fn rotate_circle_moves_center() {
        let mut entity = Entity::Circle {
            id: 1,
            layer: "Default".to_string(),
            center: Point { x: 10.0, y: 0.0 },
            radius: 5.0,
        };
        rotate_entity(&mut entity, Point { x: 0.0, y: 0.0 }, 90.0);
        let Entity::Circle { center, radius, .. } = entity else {
            panic!("circle");
        };
        assert!((center.x - 0.0).abs() < 1e-6);
        assert!((center.y - 10.0).abs() < 1e-6);
        assert!((radius - 5.0).abs() < 1e-6);
    }

    #[test]
    fn scale_point_doubles_distance() {
        let base = Point { x: 0.0, y: 0.0 };
        let p = scale_point(Point { x: 5.0, y: 0.0 }, base, 2.0);
        assert!((p.x - 10.0).abs() < 1e-6);
    }

    #[test]
    fn scale_line() {
        let mut entity = Entity::Line {
            id: 1,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 5.0, y: 0.0 },
        };
        scale_entity(&mut entity, Point { x: 0.0, y: 0.0 }, 2.0);
        let Entity::Line { end, .. } = entity else {
            panic!("line");
        };
        assert!((end.x - 10.0).abs() < 1e-6);
    }

    #[test]
    fn scale_circle_radius() {
        let mut entity = Entity::Circle {
            id: 1,
            layer: "Default".to_string(),
            center: Point { x: 0.0, y: 0.0 },
            radius: 3.0,
        };
        scale_entity(&mut entity, Point { x: 0.0, y: 0.0 }, 2.0);
        let Entity::Circle { radius, .. } = entity else {
            panic!("circle");
        };
        assert!((radius - 6.0).abs() < 1e-6);
    }

    #[test]
    fn scale_rejects_non_positive_factor() {
        let mut entity = Entity::Line {
            id: 1,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        };
        scale_entity(&mut entity, Point { x: 0.0, y: 0.0 }, 0.0);
        let Entity::Line { end, .. } = entity else {
            panic!("line");
        };
        assert!((end.x - 10.0).abs() < 1e-6);
    }

    #[test]
    fn mirror_point_horizontal_axis() {
        let p = mirror_point(
            Point { x: 5.0, y: 3.0 },
            Point { x: 0.0, y: 0.0 },
            Point { x: 100.0, y: 0.0 },
        );
        assert!((p.x - 5.0).abs() < 1e-6);
        assert!((p.y + 3.0).abs() < 1e-6);
    }

    #[test]
    fn mirror_line() {
        let mut entity = Entity::Line {
            id: 1,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 2.0 },
            end: Point { x: 10.0, y: 2.0 },
        };
        mirror_entity(
            &mut entity,
            Point { x: 0.0, y: 0.0 },
            Point { x: 100.0, y: 0.0 },
        );
        let Entity::Line { start, .. } = entity else {
            panic!("line");
        };
        assert!((start.y + 2.0).abs() < 1e-6);
    }

    #[test]
    fn mirror_circle() {
        let mut entity = Entity::Circle {
            id: 1,
            layer: "Default".to_string(),
            center: Point { x: 5.0, y: 4.0 },
            radius: 2.0,
        };
        mirror_entity(
            &mut entity,
            Point { x: 0.0, y: 0.0 },
            Point { x: 100.0, y: 0.0 },
        );
        let Entity::Circle { center, radius, .. } = entity else {
            panic!("circle");
        };
        assert!((center.x - 5.0).abs() < 1e-6);
        assert!((center.y + 4.0).abs() < 1e-6);
        assert!((radius - 2.0).abs() < 1e-6);
    }
}
