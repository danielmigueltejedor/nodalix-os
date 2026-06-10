//! Fillet / chamfer helpers for two lines (and polyline segments as lines).

use crate::{
    cad::geometry::{arc_to_polyline_points, line_line_intersection_infinite, ArcSpec, Point2},
    geometry::Point,
};

const EPS: f64 = 1e-9;
const FILLET_ARC_SEGMENTS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChamferResult {
    pub new_a_start: Point,
    pub new_a_end: Point,
    pub new_b_start: Point,
    pub new_b_end: Point,
    pub chamfer_line_start: Point,
    pub chamfer_line_end: Point,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FilletResult {
    pub new_a_start: Point,
    pub new_a_end: Point,
    pub new_b_start: Point,
    pub new_b_end: Point,
    pub arc_points: Vec<Point>,
    pub center: Point,
    pub radius: f64,
}

fn normalize(v: Point) -> Option<Point> {
    let len = (v.x * v.x + v.y * v.y).sqrt();
    if len < EPS {
        return None;
    }
    Some(Point {
        x: v.x / len,
        y: v.y / len,
    })
}

/// Unit direction from intersection into the kept portion of the line (away from the corner).
fn direction_into_line(inter: Point, start: Point, end: Point, pick: Point) -> Option<Point> {
    let corner_is_start = start.distance_to(inter) <= end.distance_to(inter);
    let mut keep = if corner_is_start { end } else { start };
    if (pick.distance_to(start) - pick.distance_to(end)).abs() > EPS {
        keep = if pick.distance_to(start) < pick.distance_to(end) {
            start
        } else {
            end
        };
    }
    normalize(Point {
        x: keep.x - inter.x,
        y: keep.y - inter.y,
    })
}

fn interior_angle(u: Point, v: Point) -> f64 {
    let dot = (u.x * v.x + u.y * v.y).clamp(-1.0, 1.0);
    dot.acos()
}

fn replace_corner_endpoint(start: &mut Point, end: &mut Point, inter: Point, new_corner: Point) {
    if start.distance_to(inter) <= end.distance_to(inter) {
        *start = new_corner;
    } else {
        *end = new_corner;
    }
}

fn segment_length_from_corner(inter: Point, start: Point, end: Point) -> f64 {
    start.distance_to(inter).max(end.distance_to(inter))
}

/// Chamfer between infinite lines through `a1–a2` and `b1–b2`.
pub fn chamfer_line_line(
    a1: Point,
    a2: Point,
    b1: Point,
    b2: Point,
    distance_a: f64,
    distance_b: f64,
    pick_a: Point,
    pick_b: Point,
) -> Option<ChamferResult> {
    if distance_a < 0.0 || distance_b < 0.0 || !distance_a.is_finite() || !distance_b.is_finite() {
        return None;
    }
    if distance_a < EPS && distance_b < EPS {
        return None;
    }
    let inter = line_line_intersection_infinite(a1, a2, b1, b2)?;
    let dir_a = direction_into_line(inter, a1, a2, pick_a)?;
    let dir_b = direction_into_line(inter, b1, b2, pick_b)?;
    if distance_a > segment_length_from_corner(inter, a1, a2) + EPS
        || distance_b > segment_length_from_corner(inter, b1, b2) + EPS
    {
        return None;
    }
    let chamfer_a = Point {
        x: inter.x + dir_a.x * distance_a,
        y: inter.y + dir_a.y * distance_a,
    };
    let chamfer_b = Point {
        x: inter.x + dir_b.x * distance_b,
        y: inter.y + dir_b.y * distance_b,
    };
    let mut new_a_start = a1;
    let mut new_a_end = a2;
    let mut new_b_start = b1;
    let mut new_b_end = b2;
    replace_corner_endpoint(&mut new_a_start, &mut new_a_end, inter, chamfer_a);
    replace_corner_endpoint(&mut new_b_start, &mut new_b_end, inter, chamfer_b);
    Some(ChamferResult {
        new_a_start,
        new_a_end,
        new_b_start,
        new_b_end,
        chamfer_line_start: chamfer_a,
        chamfer_line_end: chamfer_b,
    })
}

fn fillet_arc_points(
    center: Point,
    radius: f64,
    start: Point,
    end: Point,
    ccw: bool,
) -> Vec<Point> {
    let start_angle = (start.y - center.y).atan2(start.x - center.x);
    let end_angle = (end.y - center.y).atan2(end.x - center.x);
    let arc = ArcSpec {
        center: Point2::new(center.x, center.y),
        radius,
        start_angle,
        end_angle,
        counter_clockwise: ccw,
    };
    arc_to_polyline_points(&arc, FILLET_ARC_SEGMENTS)
        .into_iter()
        .map(|p| Point { x: p.x, y: p.y })
        .collect()
}

/// Fillet between two lines; arc is approximated as an open polyline (no `Entity::Arc` in document).
pub fn fillet_line_line(
    a1: Point,
    a2: Point,
    b1: Point,
    b2: Point,
    radius: f64,
    pick_a: Point,
    pick_b: Point,
) -> Option<FilletResult> {
    if !radius.is_finite() || radius <= EPS {
        return None;
    }
    let inter = line_line_intersection_infinite(a1, a2, b1, b2)?;
    let dir_a = direction_into_line(inter, a1, a2, pick_a)?;
    let dir_b = direction_into_line(inter, b1, b2, pick_b)?;
    let alpha = interior_angle(dir_a, dir_b);
    if alpha < EPS || (std::f64::consts::PI - alpha).abs() < EPS {
        return None;
    }
    let half = alpha / 2.0;
    let tan_half = half.tan();
    if tan_half.abs() < EPS {
        return None;
    }
    let tangent_dist = radius / tan_half;
    if tangent_dist > segment_length_from_corner(inter, a1, a2) + EPS
        || tangent_dist > segment_length_from_corner(inter, b1, b2) + EPS
    {
        return None;
    }
    let tangent_a = Point {
        x: inter.x + dir_a.x * tangent_dist,
        y: inter.y + dir_a.y * tangent_dist,
    };
    let tangent_b = Point {
        x: inter.x + dir_b.x * tangent_dist,
        y: inter.y + dir_b.y * tangent_dist,
    };
    let sin_half = half.sin();
    if sin_half.abs() < EPS {
        return None;
    }
    let center_dist = radius / sin_half;
    let bisector = normalize(Point {
        x: dir_a.x + dir_b.x,
        y: dir_a.y + dir_b.y,
    })?;
    let center = Point {
        x: inter.x + bisector.x * center_dist,
        y: inter.y + bisector.y * center_dist,
    };
    let cross = dir_a.x * dir_b.y - dir_a.y * dir_b.x;
    let ccw = cross > 0.0;
    let arc_points = fillet_arc_points(center, radius, tangent_a, tangent_b, ccw);
    if arc_points.len() < 2 {
        return None;
    }
    let mut new_a_start = a1;
    let mut new_a_end = a2;
    let mut new_b_start = b1;
    let mut new_b_end = b2;
    replace_corner_endpoint(&mut new_a_start, &mut new_a_end, inter, tangent_a);
    replace_corner_endpoint(&mut new_b_start, &mut new_b_end, inter, tangent_b);
    Some(FilletResult {
        new_a_start,
        new_a_end,
        new_b_start,
        new_b_end,
        arc_points,
        center,
        radius,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perpendicular_l() -> (Point, Point, Point, Point) {
        (
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Point { x: 10.0, y: 10.0 },
        )
    }

    #[test]
    fn chamfer_perpendicular_lines_equal_distance() {
        let (a1, a2, b1, b2) = perpendicular_l();
        let r = chamfer_line_line(
            a1,
            a2,
            b1,
            b2,
            2.0,
            2.0,
            Point { x: 5.0, y: 0.0 },
            Point { x: 10.0, y: 5.0 },
        )
        .unwrap();
        assert!((r.new_a_end.x - 8.0).abs() < 1e-6);
        assert!((r.new_b_start.y - 2.0).abs() < 1e-6);
        assert!((r.chamfer_line_start.x - 8.0).abs() < 1e-6);
        assert!((r.chamfer_line_end.y - 2.0).abs() < 1e-6);
    }

    #[test]
    fn chamfer_rejects_parallel_lines() {
        assert!(chamfer_line_line(
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Point { x: 0.0, y: 1.0 },
            Point { x: 10.0, y: 1.0 },
            1.0,
            1.0,
            Point { x: 5.0, y: 0.0 },
            Point { x: 5.0, y: 1.0 },
        )
        .is_none());
    }

    #[test]
    fn chamfer_rejects_invalid_distance() {
        let (a1, a2, b1, b2) = perpendicular_l();
        assert!(chamfer_line_line(a1, a2, b1, b2, -1.0, 2.0, a1, b2).is_none());
        assert!(chamfer_line_line(a1, a2, b1, b2, 100.0, 2.0, a1, b2).is_none());
    }

    #[test]
    fn chamfer_produces_chamfer_line() {
        let (a1, a2, b1, b2) = perpendicular_l();
        let r = chamfer_line_line(a1, a2, b1, b2, 3.0, 3.0, a1, b2).unwrap();
        let len = r.chamfer_line_start.distance_to(r.chamfer_line_end);
        assert!(len > 1.0);
    }

    #[test]
    fn fillet_perpendicular_lines() {
        let (a1, a2, b1, b2) = perpendicular_l();
        let r = fillet_line_line(
            a1,
            a2,
            b1,
            b2,
            2.0,
            Point { x: 5.0, y: 0.0 },
            Point { x: 10.0, y: 5.0 },
        )
        .unwrap();
        assert!((r.new_a_end.x - 8.0).abs() < 1e-6);
        assert!((r.new_b_start.y - 2.0).abs() < 1e-6);
        assert!((r.radius - 2.0).abs() < 1e-6);
    }

    #[test]
    fn fillet_rejects_parallel_lines() {
        assert!(fillet_line_line(
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Point { x: 0.0, y: 1.0 },
            Point { x: 10.0, y: 1.0 },
            2.0,
            Point { x: 5.0, y: 0.0 },
            Point { x: 5.0, y: 1.0 },
        )
        .is_none());
    }

    #[test]
    fn fillet_rejects_radius_non_positive() {
        let (a1, a2, b1, b2) = perpendicular_l();
        assert!(fillet_line_line(a1, a2, b1, b2, 0.0, a1, b2).is_none());
        assert!(fillet_line_line(a1, a2, b1, b2, -1.0, a1, b2).is_none());
    }

    #[test]
    fn fillet_arc_polyline_has_reasonable_points() {
        let (a1, a2, b1, b2) = perpendicular_l();
        let r = fillet_line_line(a1, a2, b1, b2, 2.0, a1, b2).unwrap();
        assert!(r.arc_points.len() >= 3);
        for p in &r.arc_points {
            let d = p.distance_to(r.center);
            assert!((d - r.radius).abs() < 0.05);
        }
    }
}
