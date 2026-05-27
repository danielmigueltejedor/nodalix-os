//! Trim / extend helpers for lines and polyline segments.

use crate::geometry::Point;

const EPS: f64 = 1e-9;

/// Intersection of infinite lines through `a1–a2` and `b1–b2`.
pub fn line_line_intersection_infinite(
    a1: Point,
    a2: Point,
    b1: Point,
    b2: Point,
) -> Option<Point> {
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
    Some(Point {
        x: x1 + t * (x2 - x1),
        y: y1 + t * (y2 - y1),
    })
}

/// Intersection of two finite segments, if any.
pub fn line_segment_intersection(a1: Point, a2: Point, b1: Point, b2: Point) -> Option<Point> {
    let inter = line_line_intersection_infinite(a1, a2, b1, b2)?;
    if point_on_segment(inter, a1, a2, EPS) && point_on_segment(inter, b1, b2, EPS) {
        Some(inter)
    } else {
        None
    }
}

fn point_on_segment(point: Point, start: Point, end: Point, eps: f64) -> bool {
    let min_x = start.x.min(end.x) - eps;
    let max_x = start.x.max(end.x) + eps;
    let min_y = start.y.min(end.y) - eps;
    let max_y = start.y.max(end.y) + eps;
    if point.x < min_x || point.x > max_x || point.y < min_y || point.y > max_y {
        return false;
    }
    let cross = (end.x - start.x) * (point.y - start.y) - (end.y - start.y) * (point.x - start.x);
    cross.abs() < eps
}

fn segment_length_squared(start: Point, end: Point) -> f64 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    dx * dx + dy * dy
}

fn line_parameter(point: Point, start: Point, end: Point) -> f64 {
    let len_sq = segment_length_squared(start, end);
    if len_sq < EPS {
        return 0.0;
    }
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    ((point.x - start.x) * dx + (point.y - start.y) * dy) / len_sq
}

/// Trim `line_start–line_end` to the cutting line through `boundary_*`; keep side containing `pick_point`.
pub fn trim_line_to_boundary(
    line_start: Point,
    line_end: Point,
    boundary_start: Point,
    boundary_end: Point,
    pick_point: Point,
) -> Option<(Point, Point)> {
    if segment_length_squared(line_start, line_end) < EPS * EPS {
        return None;
    }
    let inter =
        line_line_intersection_infinite(line_start, line_end, boundary_start, boundary_end)?;
    if !point_on_segment(inter, line_start, line_end, EPS) {
        return None;
    }
    let t_pick = line_parameter(pick_point, line_start, line_end);
    let t_inter = line_parameter(inter, line_start, line_end);
    if (t_pick - t_inter).abs() < EPS {
        return None;
    }
    if t_pick < t_inter {
        if segment_length_squared(line_start, inter) < EPS * EPS {
            return None;
        }
        Some((line_start, inter))
    } else {
        if segment_length_squared(inter, line_end) < EPS * EPS {
            return None;
        }
        Some((inter, line_end))
    }
}

/// Extend the endpoint nearer `pick_point` until the infinite boundary line.
pub fn extend_line_to_boundary(
    line_start: Point,
    line_end: Point,
    boundary_start: Point,
    boundary_end: Point,
    pick_point: Point,
) -> Option<(Point, Point)> {
    if segment_length_squared(line_start, line_end) < EPS * EPS {
        return None;
    }
    let inter =
        line_line_intersection_infinite(line_start, line_end, boundary_start, boundary_end)?;
    let extend_start = pick_point.distance_to(line_start) <= pick_point.distance_to(line_end);
    if extend_start {
        let outward_x = line_start.x - line_end.x;
        let outward_y = line_start.y - line_end.y;
        let to_inter_x = inter.x - line_start.x;
        let to_inter_y = inter.y - line_start.y;
        if outward_x * to_inter_x + outward_y * to_inter_y <= EPS {
            return None;
        }
        if segment_length_squared(inter, line_end) < EPS * EPS {
            return None;
        }
        Some((inter, line_end))
    } else {
        let outward_x = line_end.x - line_start.x;
        let outward_y = line_end.y - line_start.y;
        let to_inter_x = inter.x - line_end.x;
        let to_inter_y = inter.y - line_end.y;
        if outward_x * to_inter_x + outward_y * to_inter_y <= EPS {
            return None;
        }
        if segment_length_squared(line_start, inter) < EPS * EPS {
            return None;
        }
        Some((line_start, inter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_line_intersection_infinite_crosses() {
        let p = line_line_intersection_infinite(
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Point { x: 5.0, y: -5.0 },
            Point { x: 5.0, y: 5.0 },
        )
        .unwrap();
        assert!((p.x - 5.0).abs() < 1e-6);
        assert!(p.y.abs() < 1e-6);
    }

    #[test]
    fn line_segment_intersection_finds_crossing() {
        let p = line_segment_intersection(
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Point { x: 5.0, y: -2.0 },
            Point { x: 5.0, y: 2.0 },
        )
        .unwrap();
        assert!((p.x - 5.0).abs() < 1e-6);
    }

    #[test]
    fn trim_horizontal_line_pick_left() {
        let (s, e) = trim_line_to_boundary(
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Point { x: 5.0, y: -5.0 },
            Point { x: 5.0, y: 5.0 },
            Point { x: 2.0, y: 0.0 },
        )
        .unwrap();
        assert!((s.x - 0.0).abs() < 1e-6);
        assert!((e.x - 5.0).abs() < 1e-6);
    }

    #[test]
    fn trim_horizontal_line_pick_right() {
        let (s, e) = trim_line_to_boundary(
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Point { x: 5.0, y: -5.0 },
            Point { x: 5.0, y: 5.0 },
            Point { x: 8.0, y: 0.0 },
        )
        .unwrap();
        assert!((s.x - 5.0).abs() < 1e-6);
        assert!((e.x - 10.0).abs() < 1e-6);
    }

    #[test]
    fn trim_rejects_parallel_boundary() {
        assert!(trim_line_to_boundary(
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Point { x: 0.0, y: 1.0 },
            Point { x: 10.0, y: 1.0 },
            Point { x: 2.0, y: 0.0 },
        )
        .is_none());
    }

    #[test]
    fn extend_line_to_vertical_boundary() {
        let (s, e) = extend_line_to_boundary(
            Point { x: 2.0, y: 0.0 },
            Point { x: 6.0, y: 0.0 },
            Point { x: 10.0, y: -5.0 },
            Point { x: 10.0, y: 5.0 },
            Point { x: 6.0, y: 0.0 },
        )
        .unwrap();
        assert!((s.x - 2.0).abs() < 1e-6);
        assert!((e.x - 10.0).abs() < 1e-6);
    }

    #[test]
    fn extend_rejects_parallel_boundary() {
        assert!(extend_line_to_boundary(
            Point { x: 0.0, y: 0.0 },
            Point { x: 5.0, y: 0.0 },
            Point { x: 0.0, y: 1.0 },
            Point { x: 10.0, y: 1.0 },
            Point { x: 5.0, y: 0.0 },
        )
        .is_none());
    }

    #[test]
    fn extend_no_op_if_intersection_wrong_side() {
        assert!(extend_line_to_boundary(
            Point { x: 0.0, y: 0.0 },
            Point { x: 5.0, y: 0.0 },
            Point { x: 10.0, y: -5.0 },
            Point { x: 10.0, y: 5.0 },
            Point { x: 0.0, y: 0.0 },
        )
        .is_none());
    }
}
