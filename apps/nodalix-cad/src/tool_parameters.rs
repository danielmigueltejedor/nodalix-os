use crate::{
    cad::commands::{build_geometry_from_command_parts, GeometryBuildResult},
    cad::geometry::{
        arc_from_three_points, arc_to_polyline_points, circle_from_three_points,
        circle_from_two_diameter_points, line_from_two_points, rectangle_from_center_size,
        rectangle_from_corner_size, rectangle_from_two_corners, CircleCreationMode,
        LineCreationMode, Point2, RectangleCreationMode,
    },
    document::Entity,
    geometry::Point,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArcUiMode {
    ThreePoint,
    CenterStartEnd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DimensionCreationMode {
    Linear,
    Aligned,
    Radius,
    Diameter,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToolParametersState {
    pub circle_mode: CircleCreationMode,
    pub rectangle_mode: RectangleCreationMode,
    pub line_mode: LineCreationMode,
    pub arc_mode: ArcUiMode,
    pub dimension_mode: DimensionCreationMode,
    pub rectangle_width: f64,
    pub rectangle_height: f64,
    pub offset_distance: f64,
}

impl Default for ToolParametersState {
    fn default() -> Self {
        Self {
            circle_mode: CircleCreationMode::CenterRadius,
            rectangle_mode: RectangleCreationMode::TwoCorners,
            line_mode: LineCreationMode::TwoPoints,
            arc_mode: ArcUiMode::ThreePoint,
            dimension_mode: DimensionCreationMode::Linear,
            rectangle_width: 100.0,
            rectangle_height: 50.0,
            offset_distance: 10.0,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ToolPreview {
    None,
    Line { start: Point, end: Point },
    Circle { center: Point, radius: f64 },
    Rectangle { a: Point, b: Point },
    Polyline { points: Vec<Point>, closed: bool },
}

pub fn preview_line_two_points(start: Point, end: Point) -> ToolPreview {
    let spec = line_from_two_points(to_point2(start), to_point2(end));
    ToolPreview::Line {
        start: to_legacy(spec.start),
        end: to_legacy(spec.end),
    }
}

pub fn preview_rectangle_two_corners(a: Point, b: Point) -> ToolPreview {
    let spec = rectangle_from_two_corners(to_point2(a), to_point2(b));
    ToolPreview::Rectangle {
        a: to_legacy(spec.min),
        b: to_legacy(spec.max),
    }
}

pub fn preview_circle_center_radius(center: Point, cursor: Point) -> Option<ToolPreview> {
    let radius = center.distance_to(cursor);
    (radius > 0.0 && radius.is_finite()).then_some(ToolPreview::Circle { center, radius })
}

pub fn preview_circle_center_diameter(center: Point, cursor: Point) -> Option<ToolPreview> {
    let diameter = center.distance_to(cursor);
    let radius = diameter * 0.5;
    (radius > 0.0 && radius.is_finite()).then_some(ToolPreview::Circle { center, radius })
}

pub fn preview_circle_two_point_diameter(p1: Point, p2: Point) -> Option<ToolPreview> {
    let spec = circle_from_two_diameter_points(to_point2(p1), to_point2(p2))?;
    Some(ToolPreview::Circle {
        center: to_legacy(spec.center),
        radius: spec.radius,
    })
}

pub fn preview_circle_three_points(
    p1: Point,
    p2: Point,
    p3: Point,
) -> Result<ToolPreview, &'static str> {
    let spec = circle_from_three_points(to_point2(p1), to_point2(p2), to_point2(p3))
        .ok_or("CIRCLE 3P: points are colinear")?;
    Ok(ToolPreview::Circle {
        center: to_legacy(spec.center),
        radius: spec.radius,
    })
}

pub fn finalize_circle_entity(
    mode: CircleCreationMode,
    id: u64,
    points: &[Point],
) -> Option<Entity> {
    match mode {
        CircleCreationMode::CenterRadius if points.len() >= 2 => {
            let preview = preview_circle_center_radius(points[0], points[1])?;
            preview_to_circle_entity(id, preview)
        }
        CircleCreationMode::CenterDiameter if points.len() >= 2 => {
            let preview = preview_circle_center_diameter(points[0], points[1])?;
            preview_to_circle_entity(id, preview)
        }
        CircleCreationMode::TwoPointDiameter if points.len() >= 2 => {
            let preview = preview_circle_two_point_diameter(points[0], points[1])?;
            preview_to_circle_entity(id, preview)
        }
        CircleCreationMode::ThreePoint if points.len() >= 3 => {
            let preview = preview_circle_three_points(points[0], points[1], points[2]).ok()?;
            preview_to_circle_entity(id, preview)
        }
        _ => None,
    }
}

pub fn preview_rectangle_corner_size(
    corner: Point,
    width: f64,
    height: f64,
) -> Option<ToolPreview> {
    let spec = rectangle_from_corner_size(to_point2(corner), width, height)?;
    Some(ToolPreview::Rectangle {
        a: to_legacy(spec.min),
        b: to_legacy(spec.max),
    })
}

pub fn preview_rectangle_center_size(
    center: Point,
    width: f64,
    height: f64,
) -> Option<ToolPreview> {
    let spec = rectangle_from_center_size(to_point2(center), width, height)?;
    Some(ToolPreview::Rectangle {
        a: to_legacy(spec.min),
        b: to_legacy(spec.max),
    })
}

pub fn finalize_rectangle_entity(
    mode: RectangleCreationMode,
    id: u64,
    anchor: Point,
    width: f64,
    height: f64,
) -> Option<Entity> {
    let preview = match mode {
        RectangleCreationMode::CornerDimensions => {
            preview_rectangle_corner_size(anchor, width, height)?
        }
        RectangleCreationMode::CenterDimensions => {
            preview_rectangle_center_size(anchor, width, height)?
        }
        RectangleCreationMode::TwoCorners => return None,
    };
    let ToolPreview::Rectangle { a, b } = preview else {
        return None;
    };
    Some(Entity::Polyline {
        id,
        layer: "Default".to_string(),
        points: vec![
            Point { x: a.x, y: a.y },
            Point { x: b.x, y: a.y },
            Point { x: b.x, y: b.y },
            Point { x: a.x, y: b.y },
        ],
        closed: true,
    })
}

pub fn preview_arc_three_points(
    p1: Point,
    p2: Point,
    p3: Point,
) -> Result<ToolPreview, &'static str> {
    let arc = arc_from_three_points(to_point2(p1), to_point2(p2), to_point2(p3))
        .ok_or("ARC 3P: points are colinear")?;
    let points = arc_to_polyline_points(&arc, 24)
        .into_iter()
        .map(to_legacy)
        .collect::<Vec<_>>();
    Ok(ToolPreview::Polyline {
        points,
        closed: false,
    })
}

pub fn finalize_arc_entity(mode: ArcUiMode, id: u64, points: &[Point]) -> Option<Entity> {
    match mode {
        ArcUiMode::ThreePoint if points.len() >= 3 => {
            let ToolPreview::Polyline { points, .. } =
                preview_arc_three_points(points[0], points[1], points[2]).ok()?
            else {
                return None;
            };
            Some(Entity::Polyline {
                id,
                layer: "Default".to_string(),
                points,
                closed: false,
            })
        }
        ArcUiMode::CenterStartEnd => None,
        _ => None,
    }
}

fn preview_to_circle_entity(id: u64, preview: ToolPreview) -> Option<Entity> {
    let ToolPreview::Circle { center, radius } = preview else {
        return None;
    };
    Some(Entity::Circle {
        id,
        layer: "Default".to_string(),
        center,
        radius,
    })
}

fn to_point2(point: Point) -> Point2 {
    Point2::new(point.x, point.y)
}

fn to_legacy(point: Point2) -> Point {
    Point {
        x: point.x,
        y: point.y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_can_switch_circle_mode() {
        let mut state = ToolParametersState::default();
        assert_eq!(state.circle_mode, CircleCreationMode::CenterRadius);
        state.circle_mode = CircleCreationMode::TwoPointDiameter;
        assert_eq!(state.circle_mode, CircleCreationMode::TwoPointDiameter);
    }

    #[test]
    fn state_can_switch_dimension_mode() {
        let mut state = ToolParametersState::default();
        assert_eq!(state.dimension_mode, DimensionCreationMode::Linear);
        state.dimension_mode = DimensionCreationMode::Diameter;
        assert_eq!(state.dimension_mode, DimensionCreationMode::Diameter);
    }

    #[test]
    fn preview_center_radius() {
        let preview =
            preview_circle_center_radius(Point { x: 0.0, y: 0.0 }, Point { x: 3.0, y: 4.0 })
                .expect("preview");
        let ToolPreview::Circle { center, radius } = preview else {
            panic!("expected circle");
        };
        assert!(center.x.abs() < f64::EPSILON);
        assert!((radius - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn preview_two_point_circle() {
        let preview =
            preview_circle_two_point_diameter(Point { x: 0.0, y: 0.0 }, Point { x: 10.0, y: 0.0 })
                .expect("preview");
        let ToolPreview::Circle { center, radius } = preview else {
            panic!("expected circle");
        };
        assert!((center.x - 5.0).abs() < f64::EPSILON);
        assert!((radius - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn preview_three_point_circle_valid() {
        let preview = preview_circle_three_points(
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Point { x: 5.0, y: 5.0 },
        )
        .expect("preview");
        assert!(matches!(preview, ToolPreview::Circle { .. }));
    }

    #[test]
    fn preview_three_point_circle_colinear_fails() {
        assert!(preview_circle_three_points(
            Point { x: 0.0, y: 0.0 },
            Point { x: 5.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 }
        )
        .is_err());
    }

    #[test]
    fn rectangle_two_corners_preview() {
        let preview =
            preview_rectangle_two_corners(Point { x: 2.0, y: 3.0 }, Point { x: 8.0, y: 9.0 });
        let ToolPreview::Rectangle { a, b } = preview else {
            panic!("expected rectangle");
        };
        assert!((a.x - 2.0).abs() < f64::EPSILON);
        assert!((b.y - 9.0).abs() < f64::EPSILON);
    }

    #[test]
    fn line_two_points_preview() {
        let preview = preview_line_two_points(Point { x: 1.0, y: 2.0 }, Point { x: 3.0, y: 4.0 });
        assert!(matches!(preview, ToolPreview::Line { .. }));
    }

    #[test]
    fn rectangle_corner_size_preview() {
        let preview = preview_rectangle_corner_size(Point { x: 10.0, y: 20.0 }, 100.0, 50.0)
            .expect("preview");
        assert!(matches!(preview, ToolPreview::Rectangle { .. }));
    }

    #[test]
    fn rectangle_center_size_preview() {
        let preview =
            preview_rectangle_center_size(Point { x: 0.0, y: 0.0 }, 100.0, 50.0).expect("preview");
        let ToolPreview::Rectangle { a, b } = preview else {
            panic!("expected rectangle");
        };
        assert!((a.x + 50.0).abs() < 1e-9);
        assert!((b.x - 50.0).abs() < 1e-9);
    }

    #[test]
    fn arc_sidebar_helper_creates_polyline_not_circle() {
        let entity = finalize_arc_entity(
            ArcUiMode::ThreePoint,
            90,
            &[
                Point { x: 0.0, y: 0.0 },
                Point { x: 50.0, y: 50.0 },
                Point { x: 100.0, y: 0.0 },
            ],
        )
        .expect("arc");
        assert!(matches!(entity, Entity::Polyline { .. }));
    }

    #[test]
    fn preview_final_circle_matches_command_builder() {
        let from_preview = finalize_circle_entity(
            CircleCreationMode::TwoPointDiameter,
            77,
            &[Point { x: 0.0, y: 0.0 }, Point { x: 100.0, y: 0.0 }],
        )
        .expect("preview entity");
        let from_command =
            match build_geometry_from_command_parts(&["circle", "2p", "0,0", "100,0"], 77) {
                GeometryBuildResult::Success { entity, .. } => entity,
                other => panic!("unexpected {other:?}"),
            };
        let Entity::Circle {
            center: c1,
            radius: r1,
            ..
        } = from_preview
        else {
            panic!("expected circle");
        };
        let Entity::Circle {
            center: c2,
            radius: r2,
            ..
        } = from_command
        else {
            panic!("expected circle");
        };
        assert!((c1.x - c2.x).abs() < 1e-9);
        assert!((c1.y - c2.y).abs() < 1e-9);
        assert!((r1 - r2).abs() < 1e-9);
    }
}
