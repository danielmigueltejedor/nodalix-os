//! Hatch boundary helpers and ANSI31 line generation.

use crate::geometry::Point;

const EPS: f64 = 1e-9;
const CIRCLE_SEGMENTS: usize = 48;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HatchPatternKind {
    Solid,
    Ansi31,
}

impl HatchPatternKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Solid => "SOLID",
            Self::Ansi31 => "ANSI31",
        }
    }

    pub fn parse(value: &str) -> Self {
        if value.eq_ignore_ascii_case("SOLID") {
            Self::Solid
        } else {
            Self::Ansi31
        }
    }
}

pub fn hatch_is_solid(pattern: &str, solid: bool) -> bool {
    solid || pattern.eq_ignore_ascii_case("SOLID")
}

pub fn polyline_is_closed(points: &[Point], closed: bool) -> bool {
    if closed {
        return points.len() >= 3;
    }
    if points.len() < 3 {
        return false;
    }
    points
        .first()
        .zip(points.last())
        .is_some_and(|(a, b)| a.distance_to(*b) <= EPS.max(1e-6))
}

pub fn hatch_boundary_from_polyline(points: &[Point], closed: bool) -> Option<Vec<Point>> {
    if !polyline_is_closed(points, closed) {
        return None;
    }
    let mut boundary = points.to_vec();
    if !closed && boundary.len() >= 2 {
        let last = *boundary.last()?;
        let first = boundary[0];
        if last.distance_to(first) > EPS {
            boundary.push(first);
        }
    }
    (boundary.len() >= 3).then_some(boundary)
}

pub fn hatch_boundary_from_circle(
    center: Point,
    radius: f64,
    segments: usize,
) -> Option<Vec<Point>> {
    if !radius.is_finite() || radius <= EPS {
        return None;
    }
    let segments = segments.max(12);
    let mut points = Vec::with_capacity(segments);
    for index in 0..segments {
        let angle = std::f64::consts::TAU * index as f64 / segments as f64;
        points.push(Point {
            x: center.x + radius * angle.cos(),
            y: center.y + radius * angle.sin(),
        });
    }
    Some(points)
}

pub fn hatch_boundary_from_entity(entity: &crate::document::Entity) -> Option<Vec<Point>> {
    match entity {
        crate::document::Entity::Polyline { points, closed, .. } => {
            hatch_boundary_from_polyline(points, *closed)
        }
        crate::document::Entity::Circle { center, radius, .. } => {
            hatch_boundary_from_circle(*center, *radius, CIRCLE_SEGMENTS)
        }
        _ => None,
    }
}

pub fn polygon_area(points: &[Point]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for index in 0..points.len() {
        let a = points[index];
        let b = points[(index + 1) % points.len()];
        area += a.x * b.y - b.x * a.y;
    }
    area.abs() * 0.5
}

pub fn point_in_polygon(point: Point, polygon: &[Point]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = polygon.len() - 1;
    for i in 0..polygon.len() {
        let pi = polygon[i];
        let pj = polygon[j];
        let intersects = ((pi.y > point.y) != (pj.y > point.y))
            && (point.x < (pj.x - pi.x) * (point.y - pi.y) / (pj.y - pi.y + EPS) + pi.x);
        if intersects {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn polygon_bbox(points: &[Point]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for point in points {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    (min_x, min_y, max_x, max_y)
}

fn segment_segment_parameter(a1: Point, a2: Point, b1: Point, b2: Point) -> Option<f64> {
    let x1 = a1.x;
    let y1 = a1.y;
    let x2 = a2.x;
    let y2 = a2.y;
    let x3 = b1.x;
    let y3 = b1.y;
    let x4 = b2.x;
    let y4 = b2.y;
    let denom = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);
    if denom.abs() < EPS {
        return None;
    }
    let t = ((x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4)) / denom;
    let u = ((x1 - x3) * (y1 - y2) - (y1 - y3) * (x1 - x2)) / denom;
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some(t)
    } else {
        None
    }
}

fn clip_line_to_polygon(start: Point, end: Point, polygon: &[Point]) -> Vec<(Point, Point)> {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq < EPS {
        return Vec::new();
    }
    let mut parameters = vec![0.0, 1.0];
    let count = polygon.len();
    for index in 0..count {
        let a = polygon[index];
        let b = polygon[(index + 1) % count];
        if let Some(t) = segment_segment_parameter(start, end, a, b) {
            if t > EPS && t < 1.0 - EPS {
                parameters.push(t);
            }
        }
    }
    parameters.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    parameters.dedup_by(|a, b| (*a - *b).abs() < EPS);

    let mut segments = Vec::new();
    for window in parameters.windows(2) {
        let t0 = window[0];
        let t1 = window[1];
        let mid = Point {
            x: start.x + dx * (t0 + t1) * 0.5,
            y: start.y + dy * (t0 + t1) * 0.5,
        };
        if point_in_polygon(mid, polygon) {
            let p0 = Point {
                x: start.x + dx * t0,
                y: start.y + dy * t0,
            };
            let p1 = Point {
                x: start.x + dx * t1,
                y: start.y + dy * t1,
            };
            if p0.distance_to(p1) > EPS {
                segments.push((p0, p1));
            }
        }
    }
    segments
}

/// Generate diagonal hatch segments clipped to `boundary`.
pub fn generate_ansi31_lines(
    boundary: &[Point],
    scale: f64,
    angle_deg: f64,
) -> Vec<(Point, Point)> {
    if boundary.len() < 3 || !scale.is_finite() || scale <= EPS {
        return Vec::new();
    }
    let area = polygon_area(boundary);
    if area < EPS {
        return Vec::new();
    }
    let (min_x, min_y, max_x, max_y) = polygon_bbox(boundary);
    let cx = (min_x + max_x) * 0.5;
    let cy = (min_y + max_y) * 0.5;
    let diag = ((max_x - min_x).powi(2) + (max_y - min_y).powi(2))
        .sqrt()
        .max(scale);
    let ang = angle_deg.to_radians();
    let nx = ang.cos();
    let ny = ang.sin();
    let ux = -ny;
    let uy = nx;
    let spacing = scale;
    let extent = diag * 1.5;
    let count = ((extent / spacing).ceil() as i32) + 2;
    let mut lines = Vec::new();
    for index in -count..=count {
        let offset = index as f64 * spacing;
        let ox = cx + nx * offset;
        let oy = cy + ny * offset;
        let start = Point {
            x: ox - ux * extent,
            y: oy - uy * extent,
        };
        let end = Point {
            x: ox + ux * extent,
            y: oy + uy * extent,
        };
        lines.extend(clip_line_to_polygon(start, end, boundary));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polyline_is_closed_true_for_flag() {
        let points = vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Point { x: 10.0, y: 5.0 },
        ];
        assert!(polyline_is_closed(&points, true));
    }

    #[test]
    fn polyline_is_closed_false_for_open() {
        let points = vec![Point { x: 0.0, y: 0.0 }, Point { x: 10.0, y: 0.0 }];
        assert!(!polyline_is_closed(&points, false));
    }

    #[test]
    fn boundary_from_closed_polyline() {
        let points = vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Point { x: 10.0, y: 5.0 },
            Point { x: 0.0, y: 5.0 },
        ];
        let boundary = hatch_boundary_from_polyline(&points, true).unwrap();
        assert_eq!(boundary.len(), 4);
    }

    #[test]
    fn boundary_rejects_open_polyline() {
        let points = vec![Point { x: 0.0, y: 0.0 }, Point { x: 10.0, y: 0.0 }];
        assert!(hatch_boundary_from_polyline(&points, false).is_none());
    }

    #[test]
    fn boundary_from_circle_segments() {
        let boundary = hatch_boundary_from_circle(Point { x: 1.0, y: 2.0 }, 5.0, 32).unwrap();
        assert_eq!(boundary.len(), 32);
    }

    #[test]
    fn polygon_area_rectangle() {
        let rect = vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Point { x: 10.0, y: 5.0 },
            Point { x: 0.0, y: 5.0 },
        ];
        assert!((polygon_area(&rect) - 50.0).abs() < 1e-6);
    }

    #[test]
    fn point_in_polygon_inside_outside() {
        let rect = vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Point { x: 10.0, y: 5.0 },
            Point { x: 0.0, y: 5.0 },
        ];
        assert!(point_in_polygon(Point { x: 2.0, y: 2.0 }, &rect));
        assert!(!point_in_polygon(Point { x: 20.0, y: 2.0 }, &rect));
    }

    #[test]
    fn generate_ansi31_lines_non_empty() {
        let rect = vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Point { x: 10.0, y: 5.0 },
            Point { x: 0.0, y: 5.0 },
        ];
        let lines = generate_ansi31_lines(&rect, 2.0, 45.0);
        assert!(!lines.is_empty());
    }

    #[test]
    fn invalid_polygon_hatch_returns_empty() {
        assert!(generate_ansi31_lines(&[], 1.0, 45.0).is_empty());
    }

    #[test]
    fn render_helpers_solid_and_ansi31() {
        assert!(hatch_is_solid("SOLID", false));
        assert!(!hatch_is_solid("ANSI31", false));
        assert!(hatch_is_solid("ANSI31", true));
    }
}
