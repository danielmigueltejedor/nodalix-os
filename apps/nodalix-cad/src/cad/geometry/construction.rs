//! Pure geometric construction helpers (no GTK, no document).

use super::Point2;

const COLINEAR_EPSILON: f64 = 1e-10;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircleSpec {
    pub center: Point2,
    pub radius: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineSpec {
    pub start: Point2,
    pub end: Point2,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectangleSpec {
    pub min: Point2,
    pub max: Point2,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArcSpec {
    pub center: Point2,
    pub radius: f64,
    pub start_angle: f64,
    pub end_angle: f64,
    pub counter_clockwise: bool,
}

pub fn circle_from_center_radius(center: Point2, radius: f64) -> Option<CircleSpec> {
    if !radius.is_finite() || radius <= 0.0 {
        return None;
    }
    Some(CircleSpec { center, radius })
}

pub fn circle_from_center_diameter(center: Point2, diameter: f64) -> Option<CircleSpec> {
    if !diameter.is_finite() || diameter <= 0.0 {
        return None;
    }
    circle_from_center_radius(center, diameter / 2.0)
}

pub fn circle_from_two_diameter_points(p1: Point2, p2: Point2) -> Option<CircleSpec> {
    let diameter = p1.distance_to(p2);
    circle_from_center_radius(
        Point2 {
            x: (p1.x + p2.x) * 0.5,
            y: (p1.y + p2.y) * 0.5,
        },
        diameter / 2.0,
    )
}

/// Circumcircle through three non-colinear points.
pub fn circle_from_three_points(p1: Point2, p2: Point2, p3: Point2) -> Option<CircleSpec> {
    let ax = p1.x;
    let ay = p1.y;
    let bx = p2.x;
    let by = p2.y;
    let cx = p3.x;
    let cy = p3.y;

    let d = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
    if d.abs() < COLINEAR_EPSILON {
        return None;
    }

    let a2 = ax * ax + ay * ay;
    let b2 = bx * bx + by * by;
    let c2 = cx * cx + cy * cy;

    let ux = (a2 * (by - cy) + b2 * (cy - ay) + c2 * (ay - by)) / d;
    let uy = (a2 * (cx - bx) + b2 * (ax - cx) + c2 * (bx - ax)) / d;
    let center = Point2 { x: ux, y: uy };
    circle_from_center_radius(center, center.distance_to(p1))
}

pub fn line_from_two_points(start: Point2, end: Point2) -> LineSpec {
    LineSpec { start, end }
}

pub fn line_from_point_length_angle(
    start: Point2,
    length: f64,
    angle_degrees: f64,
) -> Option<LineSpec> {
    if !length.is_finite() || length <= 0.0 {
        return None;
    }
    let radians = angle_degrees.to_radians();
    Some(LineSpec {
        start,
        end: Point2 {
            x: start.x + length * radians.cos(),
            y: start.y + length * radians.sin(),
        },
    })
}

pub fn rectangle_from_two_corners(a: Point2, b: Point2) -> RectangleSpec {
    RectangleSpec {
        min: Point2 {
            x: a.x.min(b.x),
            y: a.y.min(b.y),
        },
        max: Point2 {
            x: a.x.max(b.x),
            y: a.y.max(b.y),
        },
    }
}

pub fn rectangle_from_corner_size(
    corner: Point2,
    width: f64,
    height: f64,
) -> Option<RectangleSpec> {
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return None;
    }
    Some(rectangle_from_two_corners(
        corner,
        Point2 {
            x: corner.x + width,
            y: corner.y + height,
        },
    ))
}

pub fn rectangle_from_center_size(
    center: Point2,
    width: f64,
    height: f64,
) -> Option<RectangleSpec> {
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return None;
    }
    let half_w = width * 0.5;
    let half_h = height * 0.5;
    Some(rectangle_from_two_corners(
        Point2 {
            x: center.x - half_w,
            y: center.y - half_h,
        },
        Point2 {
            x: center.x + half_w,
            y: center.y + half_h,
        },
    ))
}

pub fn rectangle_spec_to_corners(spec: RectangleSpec) -> [Point2; 4] {
    let RectangleSpec { min, max } = spec;
    [
        Point2 { x: min.x, y: min.y },
        Point2 { x: max.x, y: min.y },
        Point2 { x: max.x, y: max.y },
        Point2 { x: min.x, y: max.y },
    ]
}

fn angle_of(center: Point2, point: Point2) -> f64 {
    (point.y - center.y).atan2(point.x - center.x)
}

fn point_on_circle(center: Point2, radius: f64, angle: f64) -> Point2 {
    Point2 {
        x: center.x + radius * angle.cos(),
        y: center.y + radius * angle.sin(),
    }
}

fn normalize_angle_positive(angle: f64) -> f64 {
    let two_pi = std::f64::consts::TAU;
    let mut a = angle % two_pi;
    if a < 0.0 {
        a += two_pi;
    }
    a
}

fn arc_sweep_ccw(start: f64, mid: f64, end: f64) -> bool {
    let s = normalize_angle_positive(start);
    let m = normalize_angle_positive(mid);
    let e = normalize_angle_positive(end);
    if s <= e {
        s <= m && m <= e
    } else {
        m >= s || m <= e
    }
}

/// Arc through three points (start = p1, end = p3, passes through p2).
pub fn arc_from_three_points(p1: Point2, p2: Point2, p3: Point2) -> Option<ArcSpec> {
    let circle = circle_from_three_points(p1, p2, p3)?;
    let start_angle = angle_of(circle.center, p1);
    let mid_angle = angle_of(circle.center, p2);
    let end_angle = angle_of(circle.center, p3);
    let counter_clockwise = arc_sweep_ccw(start_angle, mid_angle, end_angle);
    Some(ArcSpec {
        center: circle.center,
        radius: circle.radius,
        start_angle,
        end_angle,
        counter_clockwise,
    })
}

pub fn arc_to_polyline_points(arc: &ArcSpec, segments: usize) -> Vec<Point2> {
    let segments = segments.max(2);
    let mut points = Vec::with_capacity(segments + 1);
    let start = normalize_angle_positive(arc.start_angle);
    let end = normalize_angle_positive(arc.end_angle);
    let sweep = if arc.counter_clockwise {
        if end >= start {
            end - start
        } else {
            end + std::f64::consts::TAU - start
        }
    } else if start >= end {
        -(start - end)
    } else {
        -(start + std::f64::consts::TAU - end)
    };
    for i in 0..=segments {
        let t = i as f64 / segments as f64;
        let angle = start + sweep * t;
        points.push(point_on_circle(arc.center, arc.radius, angle));
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circle_from_center_radius_valid() {
        let c = circle_from_center_radius(Point2::new(0.0, 0.0), 25.0).unwrap();
        assert!((c.radius - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn circle_from_center_diameter_works() {
        let c = circle_from_center_diameter(Point2::new(1.0, 2.0), 50.0).unwrap();
        assert!((c.radius - 25.0).abs() < f64::EPSILON);
        assert!((c.center.x - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn circle_from_two_diameter_points_works() {
        let c = circle_from_two_diameter_points(Point2::new(0.0, 0.0), Point2::new(100.0, 0.0))
            .unwrap();
        assert!((c.center.x - 50.0).abs() < f64::EPSILON);
        assert!((c.radius - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn circle_from_three_points_valid() {
        let c = circle_from_three_points(
            Point2::new(0.0, 0.0),
            Point2::new(100.0, 0.0),
            Point2::new(50.0, 50.0),
        )
        .unwrap();
        assert!((c.center.x - 50.0).abs() < 1e-6);
        assert!((c.center.y - 0.0).abs() < 1e-6);
        assert!((c.radius - 50.0).abs() < 1e-6);
    }

    #[test]
    fn circle_from_three_points_colinear_fails() {
        assert!(circle_from_three_points(
            Point2::new(0.0, 0.0),
            Point2::new(50.0, 0.0),
            Point2::new(100.0, 0.0),
        )
        .is_none());
    }

    #[test]
    fn rectangle_from_corner_size_works() {
        let r = rectangle_from_corner_size(Point2::new(0.0, 0.0), 100.0, 50.0).unwrap();
        assert!((r.max.x - 100.0).abs() < f64::EPSILON);
        assert!((r.max.y - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn rectangle_from_center_size_works() {
        let r = rectangle_from_center_size(Point2::new(0.0, 0.0), 100.0, 50.0).unwrap();
        assert!((r.min.x - (-50.0)).abs() < f64::EPSILON);
        assert!((r.max.x - 50.0).abs() < f64::EPSILON);
        assert!((r.max.y - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn line_from_point_length_angle_works() {
        let line = line_from_point_length_angle(Point2::new(0.0, 0.0), 100.0, 45.0).unwrap();
        assert!((line.end.x - 100.0 * 45.0f64.to_radians().cos()).abs() < 1e-6);
        assert!((line.end.y - 100.0 * 45.0f64.to_radians().sin()).abs() < 1e-6);
    }

    #[test]
    fn arc_from_three_points_produces_sweep() {
        let arc = arc_from_three_points(
            Point2::new(0.0, 0.0),
            Point2::new(50.0, 50.0),
            Point2::new(100.0, 0.0),
        )
        .unwrap();
        let pts = arc_to_polyline_points(&arc, 8);
        assert!(pts.len() >= 3);
        assert!((pts[0].x - 0.0).abs() < 1e-3);
    }

    #[test]
    fn command_style_circle_diameter_equivalent() {
        let c = circle_from_center_diameter(Point2::new(0.0, 0.0), 50.0).unwrap();
        assert!((c.radius - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn command_style_circle_2p() {
        let c = circle_from_two_diameter_points(Point2::new(0.0, 0.0), Point2::new(100.0, 0.0))
            .unwrap();
        assert!((c.radius - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn command_style_circle_3p() {
        let c = circle_from_three_points(
            Point2::new(0.0, 0.0),
            Point2::new(100.0, 0.0),
            Point2::new(50.0, 50.0),
        );
        assert!(c.is_some());
    }

    #[test]
    fn command_style_rectangle_size() {
        assert!(rectangle_from_corner_size(Point2::new(0.0, 0.0), 100.0, 50.0).is_some());
    }

    #[test]
    fn command_style_rectangle_center() {
        assert!(rectangle_from_center_size(Point2::new(0.0, 0.0), 100.0, 50.0).is_some());
    }

    #[test]
    fn command_style_line_length() {
        assert!(line_from_point_length_angle(Point2::new(0.0, 0.0), 100.0, 45.0).is_some());
    }
}
