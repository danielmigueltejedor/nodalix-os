use crate::{geometry::Point, units::Unit};

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
}
