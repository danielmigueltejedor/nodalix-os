use crate::{geometry::Point, units::Unit};

// NOTE:
// Imported DWG/DXF exploded dimensions remain plain geometry entities.
// Native dimensions authored in LixCAD use `Entity::Dimension`.
// Future work: dimension reconstruction from imported geometry.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DimensionKind {
    Linear,
    Aligned,
    Radius,
    Diameter,
}

pub fn format_distance_with_unit(value: f64, unit: Unit, precision: u8) -> String {
    let p = usize::from(precision);
    format!("{:.*} {}", p, value.max(0.0), unit.label())
}

pub fn format_radius_with_unit(value: f64, unit: Unit, precision: u8) -> String {
    let p = usize::from(precision);
    format!("R{:.*} {}", p, value.max(0.0), unit.label())
}

pub fn format_diameter_with_unit(value: f64, unit: Unit, precision: u8) -> String {
    let p = usize::from(precision);
    format!("Ø{:.*} {}", p, value.max(0.0), unit.label())
}

pub fn dimension_offset_from_point(start: Point, end: Point, offset_point: Point) -> Point {
    let mid = Point {
        x: (start.x + end.x) * 0.5,
        y: (start.y + end.y) * 0.5,
    };
    let vx = end.x - start.x;
    let vy = end.y - start.y;
    let len = (vx * vx + vy * vy).sqrt().max(1e-9);
    let nx = -vy / len;
    let ny = vx / len;
    let dx = offset_point.x - mid.x;
    let dy = offset_point.y - mid.y;
    let distance = dx * nx + dy * ny;
    Point {
        x: nx * distance,
        y: ny * distance,
    }
}

pub fn dimension_segments(start: Point, end: Point, offset: Point) -> (Point, Point, Point, Point) {
    let dim_start = Point {
        x: start.x + offset.x,
        y: start.y + offset.y,
    };
    let dim_end = Point {
        x: end.x + offset.x,
        y: end.y + offset.y,
    };
    (start, end, dim_start, dim_end)
}

pub fn parse_dimension_style(style: &str) -> (DimensionKind, Point) {
    let normalized = style.trim();
    let (kind, raw_offset) = normalized
        .split_once('@')
        .map(|(a, b)| (a, Some(b)))
        .unwrap_or((normalized, None));
    let kind = match kind.to_ascii_lowercase().as_str() {
        "aligned" => DimensionKind::Aligned,
        "radius" => DimensionKind::Radius,
        "diameter" => DimensionKind::Diameter,
        _ => DimensionKind::Linear,
    };
    let offset = raw_offset
        .and_then(|raw| raw.split_once(','))
        .and_then(|(x, y)| {
            Some(Point {
                x: x.parse().ok()?,
                y: y.parse().ok()?,
            })
        })
        .unwrap_or_default();
    (kind, offset)
}

pub fn linear_dimension_points(start: Point, end: Point) -> (Point, Point) {
    let dx = (end.x - start.x).abs();
    let dy = (end.y - start.y).abs();
    if dx >= dy {
        (
            start,
            Point {
                x: end.x,
                y: start.y,
            },
        )
    } else {
        (
            start,
            Point {
                x: start.x,
                y: end.y,
            },
        )
    }
}

pub fn aligned_dimension_points(start: Point, end: Point) -> (Point, Point) {
    (start, end)
}

pub fn radius_dimension_from_circle(
    center: Point,
    radius: f64,
    cursor: Point,
    unit: Unit,
    precision: u8,
) -> Option<(Point, Point, String)> {
    if !radius.is_finite() || radius <= 0.0 {
        return None;
    }
    let vx = cursor.x - center.x;
    let vy = cursor.y - center.y;
    let len = (vx * vx + vy * vy).sqrt();
    let (ux, uy) = if len <= 1e-9 {
        (1.0, 0.0)
    } else {
        (vx / len, vy / len)
    };
    let end = Point {
        x: center.x + ux * radius,
        y: center.y + uy * radius,
    };
    let label = format_radius_with_unit(radius, unit, precision);
    Some((center, end, label))
}

pub fn diameter_dimension_from_circle(
    center: Point,
    radius: f64,
    cursor: Point,
    unit: Unit,
    precision: u8,
) -> Option<(Point, Point, String)> {
    if !radius.is_finite() || radius <= 0.0 {
        return None;
    }
    let vx = cursor.x - center.x;
    let vy = cursor.y - center.y;
    let len = (vx * vx + vy * vy).sqrt();
    let (ux, uy) = if len <= 1e-9 {
        (1.0, 0.0)
    } else {
        (vx / len, vy / len)
    };
    let a = Point {
        x: center.x - ux * radius,
        y: center.y - uy * radius,
    };
    let b = Point {
        x: center.x + ux * radius,
        y: center.y + uy * radius,
    };
    let label = format_diameter_with_unit(radius * 2.0, unit, precision);
    Some((a, b, label))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_distance_uses_unit() {
        assert_eq!(
            format_distance_with_unit(100.0, Unit::Millimeters, 0),
            "100 mm"
        );
        assert_eq!(format_distance_with_unit(2.5, Unit::Meters, 2), "2.50 m");
    }

    #[test]
    fn format_radius_and_diameter_symbols() {
        assert_eq!(
            format_radius_with_unit(25.0, Unit::Millimeters, 0),
            "R25 mm"
        );
        assert_eq!(
            format_diameter_with_unit(50.0, Unit::Millimeters, 0),
            "Ø50 mm"
        );
    }

    #[test]
    fn offset_computation() {
        let start = Point { x: 0.0, y: 0.0 };
        let end = Point { x: 10.0, y: 0.0 };
        let offset = dimension_offset_from_point(start, end, Point { x: 5.0, y: 3.0 });
        assert!(offset.y > 2.9 && offset.y < 3.1);
    }

    #[test]
    fn linear_dimension_geometry() {
        let (a, b) = linear_dimension_points(Point { x: 0.0, y: 0.0 }, Point { x: 10.0, y: 3.0 });
        assert!((a.y - b.y).abs() < 1e-9);
    }

    #[test]
    fn aligned_dimension_geometry() {
        let (a, b) = aligned_dimension_points(Point { x: 0.0, y: 0.0 }, Point { x: 10.0, y: 3.0 });
        assert!((a.x - 0.0).abs() < 1e-9);
        assert!((b.y - 3.0).abs() < 1e-9);
    }

    #[test]
    fn radius_dimension_from_circle_builds_label() {
        let (_, _, label) = radius_dimension_from_circle(
            Point { x: 0.0, y: 0.0 },
            25.0,
            Point { x: 10.0, y: 0.0 },
            Unit::Millimeters,
            0,
        )
        .expect("radius dim");
        assert_eq!(label, "R25 mm");
    }

    #[test]
    fn diameter_dimension_from_circle_builds_label() {
        let (_, _, label) = diameter_dimension_from_circle(
            Point { x: 0.0, y: 0.0 },
            25.0,
            Point { x: 10.0, y: 0.0 },
            Unit::Millimeters,
            0,
        )
        .expect("diameter dim");
        assert_eq!(label, "Ø50 mm");
    }

    #[test]
    fn invalid_radius_target_fails() {
        assert!(radius_dimension_from_circle(
            Point { x: 0.0, y: 0.0 },
            0.0,
            Point { x: 1.0, y: 0.0 },
            Unit::Millimeters,
            0
        )
        .is_none());
    }
}
