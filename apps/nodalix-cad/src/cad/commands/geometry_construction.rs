//! Command-bar geometry construction (alternative creation methods).
//!
// TODO(phase-4): move interactive tool routing to `cad/tools/draw`.

use crate::{
    cad::geometry::{
        arc_from_three_points, arc_to_polyline_points, circle_from_center_diameter,
        circle_from_center_radius, circle_from_three_points, circle_from_two_diameter_points,
        line_from_point_length_angle, line_from_two_points, rectangle_from_center_size,
        rectangle_from_corner_size, rectangle_from_two_corners, rectangle_spec_to_corners, Point2,
    },
    document::Entity,
    geometry::Point,
};

#[derive(Debug, Clone)]
pub enum GeometryBuildResult {
    NotHandled,
    Success {
        entity: Entity,
        message: &'static str,
    },
    Error {
        message: &'static str,
    },
}

pub fn build_geometry_from_command_parts(parts: &[&str], next_id: u64) -> GeometryBuildResult {
    let Some(name) = parts.first().copied() else {
        return GeometryBuildResult::NotHandled;
    };

    match name {
        "line" | "l" | "li" => parse_line(parts, next_id),
        "circle" | "c" | "ci" => parse_circle(parts, next_id),
        "rect" | "rectangle" | "rec" | "r" => parse_rectangle(parts, next_id),
        "arc" | "a" => parse_arc(parts, next_id),
        "pline" | "polyline" | "pl" | "pol" => parse_polyline(parts, next_id),
        "point" | "pt" => parse_point(parts, next_id),
        _ => GeometryBuildResult::NotHandled,
    }
}

fn parse_line(parts: &[&str], id: u64) -> GeometryBuildResult {
    match parts.len() {
        3 => {
            let Some(start) = parse_command_point(parts[1]) else {
                return GeometryBuildResult::Error {
                    message: "LINE: invalid first point, use x,y",
                };
            };
            let Some(end) = parse_command_point(parts[2]) else {
                return GeometryBuildResult::Error {
                    message: "LINE: invalid second point, use x,y",
                };
            };
            let spec = line_from_two_points(to_point2(start), to_point2(end));
            GeometryBuildResult::Success {
                entity: Entity::Line {
                    id,
                    layer: default_layer(),
                    start: to_legacy(spec.start),
                    end: to_legacy(spec.end),
                },
                message: "LINE: created from two points",
            }
        }
        5 if parts[1] == "length" || parts[1] == "l" => {
            let Some(start) = parse_command_point(parts[2]) else {
                return GeometryBuildResult::Error {
                    message: "LINE: invalid start point, use x,y",
                };
            };
            let Ok(length) = parts[3].parse::<f64>() else {
                return GeometryBuildResult::Error {
                    message: "LINE: invalid length",
                };
            };
            let Ok(angle) = parts[4].parse::<f64>() else {
                return GeometryBuildResult::Error {
                    message: "LINE: invalid angle in degrees",
                };
            };
            let Some(spec) = line_from_point_length_angle(to_point2(start), length, angle) else {
                return GeometryBuildResult::Error {
                    message: "LINE: invalid length or angle",
                };
            };
            GeometryBuildResult::Success {
                entity: Entity::Line {
                    id,
                    layer: default_layer(),
                    start: to_legacy(spec.start),
                    end: to_legacy(spec.end),
                },
                message: "LINE: created from length, point and angle",
            }
        }
        4 => {
            // Legacy: line length start angle  (e.g. line 100 0,0 45)
            let Ok(length) = parts[1].parse::<f64>() else {
                return GeometryBuildResult::NotHandled;
            };
            let Some(start) = parse_command_point(parts[2]) else {
                return GeometryBuildResult::Error {
                    message: "LINE: invalid start point, use x,y",
                };
            };
            let Ok(angle) = parts[3].parse::<f64>() else {
                return GeometryBuildResult::Error {
                    message: "LINE: invalid angle in degrees",
                };
            };
            let Some(spec) = line_from_point_length_angle(to_point2(start), length, angle) else {
                return GeometryBuildResult::Error {
                    message: "LINE: invalid length",
                };
            };
            GeometryBuildResult::Success {
                entity: Entity::Line {
                    id,
                    layer: default_layer(),
                    start: to_legacy(spec.start),
                    end: to_legacy(spec.end),
                },
                message: "LINE: created from length, point and angle",
            }
        }
        _ => GeometryBuildResult::NotHandled,
    }
}

fn parse_circle(parts: &[&str], id: u64) -> GeometryBuildResult {
    if parts.len() == 3 {
        let Some(center) = parse_command_point(parts[1]) else {
            return GeometryBuildResult::Error {
                message: "CIRCLE: invalid center point",
            };
        };
        let Ok(radius) = parts[2].parse::<f64>() else {
            return GeometryBuildResult::Error {
                message: "CIRCLE: invalid radius",
            };
        };
        let Some(spec) = circle_from_center_radius(to_point2(center), radius) else {
            return GeometryBuildResult::Error {
                message: "CIRCLE: radius must be positive",
            };
        };
        return circle_entity(id, spec, "CIRCLE: created from center and radius");
    }

    if parts.len() == 4 && (parts[1] == "diameter" || parts[1] == "d" || parts[1] == "dia") {
        let Some(center) = parse_command_point(parts[2]) else {
            return GeometryBuildResult::Error {
                message: "CIRCLE: invalid center point",
            };
        };
        let Ok(diameter) = parts[3].parse::<f64>() else {
            return GeometryBuildResult::Error {
                message: "CIRCLE: invalid diameter",
            };
        };
        let Some(spec) = circle_from_center_diameter(to_point2(center), diameter) else {
            return GeometryBuildResult::Error {
                message: "CIRCLE: diameter must be positive",
            };
        };
        return circle_entity(id, spec, "CIRCLE: created from center and diameter");
    }

    if parts.len() == 4 && (parts[1] == "2p" || parts[1] == "2point") {
        let Some(p1) = parse_command_point(parts[2]) else {
            return GeometryBuildResult::Error {
                message: "CIRCLE 2P: invalid first point",
            };
        };
        let Some(p2) = parse_command_point(parts[3]) else {
            return GeometryBuildResult::Error {
                message: "CIRCLE 2P: invalid second point",
            };
        };
        let Some(spec) = circle_from_two_diameter_points(to_point2(p1), to_point2(p2)) else {
            return GeometryBuildResult::Error {
                message: "CIRCLE 2P: points must differ",
            };
        };
        return circle_entity(id, spec, "CIRCLE: created from two diameter points");
    }

    if parts.len() == 5 && (parts[1] == "3p" || parts[1] == "3point") {
        let Some(p1) = parse_command_point(parts[2]) else {
            return GeometryBuildResult::Error {
                message: "CIRCLE 3P: invalid first point",
            };
        };
        let Some(p2) = parse_command_point(parts[3]) else {
            return GeometryBuildResult::Error {
                message: "CIRCLE 3P: invalid second point",
            };
        };
        let Some(p3) = parse_command_point(parts[4]) else {
            return GeometryBuildResult::Error {
                message: "CIRCLE 3P: invalid third point",
            };
        };
        let Some(spec) = circle_from_three_points(to_point2(p1), to_point2(p2), to_point2(p3))
        else {
            return GeometryBuildResult::Error {
                message: "CIRCLE 3P: points are colinear",
            };
        };
        return circle_entity(id, spec, "CIRCLE: created from three points");
    }

    GeometryBuildResult::NotHandled
}

fn parse_rectangle(parts: &[&str], id: u64) -> GeometryBuildResult {
    if parts.len() == 3 {
        let Some(a) = parse_command_point(parts[1]) else {
            return GeometryBuildResult::Error {
                message: "RECTANGLE: invalid first corner",
            };
        };
        let Some(b) = parse_command_point(parts[2]) else {
            return GeometryBuildResult::Error {
                message: "RECTANGLE: invalid second corner",
            };
        };
        let spec = rectangle_from_two_corners(to_point2(a), to_point2(b));
        return rectangle_entity(id, spec, "RECTANGLE: created from two corners");
    }

    if parts.len() == 5 && (parts[1] == "size" || parts[1] == "s") {
        let Some(corner) = parse_command_point(parts[2]) else {
            return GeometryBuildResult::Error {
                message: "RECTANGLE: invalid corner point",
            };
        };
        let Ok(width) = parts[3].parse::<f64>() else {
            return GeometryBuildResult::Error {
                message: "RECTANGLE: invalid width",
            };
        };
        let Ok(height) = parts[4].parse::<f64>() else {
            return GeometryBuildResult::Error {
                message: "RECTANGLE: invalid height",
            };
        };
        let Some(spec) = rectangle_from_corner_size(to_point2(corner), width, height) else {
            return GeometryBuildResult::Error {
                message: "RECTANGLE: width and height must be positive",
            };
        };
        return rectangle_entity(id, spec, "RECTANGLE: created from corner and size");
    }

    if parts.len() == 5 && (parts[1] == "center" || parts[1] == "c") {
        let Some(center) = parse_command_point(parts[2]) else {
            return GeometryBuildResult::Error {
                message: "RECTANGLE: invalid center point",
            };
        };
        let Ok(width) = parts[3].parse::<f64>() else {
            return GeometryBuildResult::Error {
                message: "RECTANGLE: invalid width",
            };
        };
        let Ok(height) = parts[4].parse::<f64>() else {
            return GeometryBuildResult::Error {
                message: "RECTANGLE: invalid height",
            };
        };
        let Some(spec) = rectangle_from_center_size(to_point2(center), width, height) else {
            return GeometryBuildResult::Error {
                message: "RECTANGLE: width and height must be positive",
            };
        };
        return rectangle_entity(id, spec, "RECTANGLE: created from center and size");
    }

    GeometryBuildResult::NotHandled
}

fn parse_arc(parts: &[&str], id: u64) -> GeometryBuildResult {
    if parts.len() == 5 && (parts[1] == "3p" || parts[1] == "3point") {
        let Some(p1) = parse_command_point(parts[2]) else {
            return GeometryBuildResult::Error {
                message: "ARC 3P: invalid first point",
            };
        };
        let Some(p2) = parse_command_point(parts[3]) else {
            return GeometryBuildResult::Error {
                message: "ARC 3P: invalid second point",
            };
        };
        let Some(p3) = parse_command_point(parts[4]) else {
            return GeometryBuildResult::Error {
                message: "ARC 3P: invalid third point",
            };
        };
        let Some(arc) = arc_from_three_points(to_point2(p1), to_point2(p2), to_point2(p3)) else {
            return GeometryBuildResult::Error {
                message: "ARC 3P: points are colinear",
            };
        };
        let points: Vec<Point> = arc_to_polyline_points(&arc, 24)
            .into_iter()
            .map(to_legacy)
            .collect();
        return GeometryBuildResult::Success {
            entity: Entity::Polyline {
                id,
                layer: default_layer(),
                points,
                closed: false,
            },
            message: "ARC: created from three points",
        };
    }

    if parts.len() >= 2 && (parts[1] == "center" || parts[1] == "c") {
        return GeometryBuildResult::Error {
            message: "ARC center: not supported yet (TODO: dedicated arc entity)",
        };
    }

    GeometryBuildResult::NotHandled
}

fn parse_polyline(parts: &[&str], id: u64) -> GeometryBuildResult {
    if parts.len() < 3 {
        return GeometryBuildResult::NotHandled;
    }
    let points = parts[1..]
        .iter()
        .filter_map(|part| parse_command_point(part))
        .collect::<Vec<_>>();
    if points.len() != parts.len() - 1 {
        return GeometryBuildResult::Error {
            message: "PLINE: use x,y x,y ...",
        };
    }
    GeometryBuildResult::Success {
        entity: Entity::Polyline {
            id,
            layer: default_layer(),
            points,
            closed: false,
        },
        message: "PLINE: created from command points",
    }
}

fn parse_point(parts: &[&str], id: u64) -> GeometryBuildResult {
    if parts.len() != 2 {
        return GeometryBuildResult::NotHandled;
    }
    let Some(point) = parse_command_point(parts[1]) else {
        return GeometryBuildResult::Error {
            message: "POINT: invalid point",
        };
    };
    GeometryBuildResult::Success {
        entity: Entity::Point {
            id,
            layer: default_layer(),
            point,
        },
        message: "POINT: created",
    }
}

fn circle_entity(
    id: u64,
    spec: crate::cad::geometry::CircleSpec,
    message: &'static str,
) -> GeometryBuildResult {
    GeometryBuildResult::Success {
        entity: Entity::Circle {
            id,
            layer: default_layer(),
            center: to_legacy(spec.center),
            radius: spec.radius,
        },
        message,
    }
}

fn rectangle_entity(
    id: u64,
    spec: crate::cad::geometry::RectangleSpec,
    message: &'static str,
) -> GeometryBuildResult {
    let corners = rectangle_spec_to_corners(spec);
    GeometryBuildResult::Success {
        entity: Entity::Polyline {
            id,
            layer: default_layer(),
            points: corners.map(to_legacy).to_vec(),
            closed: true,
        },
        message,
    }
}

fn default_layer() -> String {
    "Default".to_string()
}

fn parse_command_point(value: &str) -> Option<Point> {
    let (x, y) = value.split_once(',')?;
    Some(Point {
        x: x.parse().ok()?,
        y: y.parse().ok()?,
    })
}

fn to_point2(p: Point) -> Point2 {
    Point2::new(p.x, p.y)
}

fn to_legacy(p: Point2) -> Point {
    Point { x: p.x, y: p.y }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn circle_id(parts: &[&str]) -> GeometryBuildResult {
        build_geometry_from_command_parts(parts, 1)
    }

    #[test]
    fn parse_circle_center_radius() {
        match circle_id(&["circle", "0,0", "25"]) {
            GeometryBuildResult::Success { entity, .. } => {
                let Entity::Circle { radius, .. } = entity else {
                    panic!("expected circle");
                };
                assert!((radius - 25.0).abs() < f64::EPSILON);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn parse_circle_diameter() {
        match circle_id(&["circle", "d", "0,0", "50"]) {
            GeometryBuildResult::Success { entity, .. } => {
                let Entity::Circle { radius, .. } = entity else {
                    panic!("expected circle");
                };
                assert!((radius - 25.0).abs() < f64::EPSILON);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn parse_circle_two_point() {
        assert!(matches!(
            circle_id(&["circle", "2p", "0,0", "100,0"]),
            GeometryBuildResult::Success { .. }
        ));
    }

    #[test]
    fn parse_circle_three_point() {
        assert!(matches!(
            circle_id(&["circle", "3p", "0,0", "100,0", "50,50"]),
            GeometryBuildResult::Success { .. }
        ));
    }

    #[test]
    fn parse_rectangle_size() {
        assert!(matches!(
            build_geometry_from_command_parts(&["rectangle", "size", "0,0", "100", "50"], 2),
            GeometryBuildResult::Success { .. }
        ));
    }

    #[test]
    fn parse_rectangle_center() {
        assert!(matches!(
            build_geometry_from_command_parts(&["rectangle", "center", "0,0", "100", "50"], 3),
            GeometryBuildResult::Success { .. }
        ));
    }

    #[test]
    fn parse_line_length_syntax() {
        assert!(matches!(
            build_geometry_from_command_parts(&["line", "length", "0,0", "100", "45"], 4),
            GeometryBuildResult::Success { .. }
        ));
    }
}
