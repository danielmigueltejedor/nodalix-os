//! Ortho mode, polar tracking, and dynamic distance/angle preview helpers.

use crate::{
    cad::dimensions::format_distance_with_unit, cad::snapping::SnapTarget, geometry::Point,
    units::Unit,
};

pub const POLAR_TOLERANCE_DEGREES: f64 = 5.0;

const DEFAULT_POLAR_ANGLES: &[f64] = &[
    0.0, 30.0, 45.0, 60.0, 90.0, 120.0, 135.0, 150.0, 180.0, 210.0, 225.0, 240.0, 270.0, 300.0,
    315.0, 330.0,
];

#[derive(Clone, Debug, PartialEq)]
pub struct PrecisionState {
    pub ortho_enabled: bool,
    pub polar_enabled: bool,
    pub polar_angles_degrees: Vec<f64>,
    pub dynamic_input_enabled: bool,
}

impl Default for PrecisionState {
    fn default() -> Self {
        Self {
            ortho_enabled: false,
            polar_enabled: true,
            polar_angles_degrees: DEFAULT_POLAR_ANGLES.to_vec(),
            dynamic_input_enabled: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PointSource {
    Raw,
    Osnap(crate::cad::snapping::SnapKind),
    Ortho,
    Polar { angle_degrees: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TrackingGuideKind {
    Ortho,
    Polar { angle_degrees: f64 },
}

#[derive(Clone, Copy, Debug)]
pub struct TrackingGuide {
    pub kind: TrackingGuideKind,
    pub base: Point,
    pub through: Point,
}

#[derive(Clone, Copy, Debug)]
pub struct ResolvedPoint {
    pub point: Point,
    pub source: PointSource,
    pub guide: Option<TrackingGuide>,
}

/// Anchor for ortho/polar while drawing: polyline tail, pending start, or last staged point.
pub fn precision_anchor(
    pending_start: Option<Point>,
    pending_points: &[Point],
    polyline_vertices: &[Point],
) -> Option<Point> {
    polyline_vertices
        .last()
        .copied()
        .or(pending_start)
        .or_else(|| pending_points.last().copied())
}

/// Final cursor point: OSNAP wins, then ortho (over polar), then polar, else raw.
pub fn resolve_precision_point(
    raw_cursor: Point,
    base_point: Option<Point>,
    osnap: Option<SnapTarget>,
    state: &PrecisionState,
) -> ResolvedPoint {
    if let Some(snap) = osnap {
        return ResolvedPoint {
            point: snap.point,
            source: PointSource::Osnap(snap.kind),
            guide: None,
        };
    }
    let Some(base) = base_point else {
        return ResolvedPoint {
            point: raw_cursor,
            source: PointSource::Raw,
            guide: None,
        };
    };
    if state.ortho_enabled {
        let point = apply_ortho(base, raw_cursor);
        return ResolvedPoint {
            point,
            source: PointSource::Ortho,
            guide: Some(TrackingGuide {
                kind: TrackingGuideKind::Ortho,
                base,
                through: point,
            }),
        };
    }
    if state.polar_enabled {
        if let Some((point, angle)) = apply_polar(
            base,
            raw_cursor,
            &state.polar_angles_degrees,
            POLAR_TOLERANCE_DEGREES,
        ) {
            return ResolvedPoint {
                point,
                source: PointSource::Polar {
                    angle_degrees: angle,
                },
                guide: Some(TrackingGuide {
                    kind: TrackingGuideKind::Polar {
                        angle_degrees: angle,
                    },
                    base,
                    through: point,
                }),
            };
        }
    }
    ResolvedPoint {
        point: raw_cursor,
        source: PointSource::Raw,
        guide: None,
    }
}

pub fn apply_ortho(base: Point, cursor: Point) -> Point {
    let dx = (cursor.x - base.x).abs();
    let dy = (cursor.y - base.y).abs();
    if dx < dy {
        Point {
            x: base.x,
            y: cursor.y,
        }
    } else {
        Point {
            x: cursor.x,
            y: base.y,
        }
    }
}

pub fn apply_polar(
    base: Point,
    cursor: Point,
    angles_degrees: &[f64],
    tolerance_degrees: f64,
) -> Option<(Point, f64)> {
    let dx = cursor.x - base.x;
    let dy = cursor.y - base.y;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist <= 1e-9 {
        return None;
    }
    let angle = normalize_degrees(dy.atan2(dx).to_degrees());
    let nearest = nearest_polar_angle(angle, angles_degrees)?;
    if angular_distance_degrees(angle, nearest) > tolerance_degrees {
        return None;
    }
    let rad = nearest.to_radians();
    Some((
        Point {
            x: base.x + dist * rad.cos(),
            y: base.y + dist * rad.sin(),
        },
        nearest,
    ))
}

pub fn ortho_move_delta(dx: f64, dy: f64) -> (f64, f64) {
    if dx.abs() < dy.abs() {
        (0.0, dy)
    } else {
        (dx, 0.0)
    }
}

pub fn normalize_degrees(angle: f64) -> f64 {
    let mut a = angle % 360.0;
    if a < 0.0 {
        a += 360.0;
    }
    a
}

pub fn angular_distance_degrees(a: f64, b: f64) -> f64 {
    let diff = (normalize_degrees(a) - normalize_degrees(b)).abs();
    diff.min(360.0 - diff)
}

pub fn angle_degrees_from_points(base: Point, cursor: Point) -> f64 {
    let dx = cursor.x - base.x;
    let dy = cursor.y - base.y;
    normalize_degrees(dy.atan2(dx).to_degrees())
}

pub fn format_angle_degrees(angle: f64) -> String {
    format!("{:.0}°", normalize_degrees(angle))
}

pub fn dynamic_preview_label(base: Point, cursor: Point, unit: Unit, precision: u8) -> String {
    let length = base.distance_to(cursor);
    let angle = angle_degrees_from_points(base, cursor);
    format!(
        "{} @ {}",
        format_distance_with_unit(length, unit, precision),
        format_angle_degrees(angle)
    )
}

pub fn tracking_guide_label(guide: TrackingGuide) -> String {
    match guide.kind {
        TrackingGuideKind::Ortho => "ORTHO".to_string(),
        TrackingGuideKind::Polar { angle_degrees } => format_angle_degrees(angle_degrees),
    }
}

fn nearest_polar_angle(angle: f64, angles: &[f64]) -> Option<f64> {
    angles.iter().copied().min_by(|a, b| {
        angular_distance_degrees(angle, *a)
            .partial_cmp(&angular_distance_degrees(angle, *b))
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::snapping::{SnapKind, SnapTarget};

    fn state_ortho() -> PrecisionState {
        PrecisionState {
            ortho_enabled: true,
            polar_enabled: false,
            ..PrecisionState::default()
        }
    }

    fn state_polar() -> PrecisionState {
        PrecisionState {
            ortho_enabled: false,
            polar_enabled: true,
            ..PrecisionState::default()
        }
    }

    #[test]
    fn ortho_horizontal() {
        let base = Point { x: 0.0, y: 0.0 };
        let cursor = Point { x: 10.0, y: 3.0 };
        let p = apply_ortho(base, cursor);
        assert!((p.x - 10.0).abs() < 1e-9);
        assert!((p.y - 0.0).abs() < 1e-9);
    }

    #[test]
    fn ortho_vertical() {
        let base = Point { x: 0.0, y: 0.0 };
        let cursor = Point { x: 3.0, y: 10.0 };
        let p = apply_ortho(base, cursor);
        assert!((p.x - 0.0).abs() < 1e-9);
        assert!((p.y - 10.0).abs() < 1e-9);
    }

    #[test]
    fn ortho_picks_dominant_axis() {
        let base = Point { x: 0.0, y: 0.0 };
        let cursor = Point { x: 4.0, y: 5.0 };
        let p = apply_ortho(base, cursor);
        assert!((p.x - 0.0).abs() < 1e-9);
        assert!((p.y - 5.0).abs() < 1e-9);
    }

    #[test]
    fn polar_locks_45_degrees() {
        let base = Point { x: 0.0, y: 0.0 };
        let cursor = Point {
            x: 70.0 * 45.0_f64.to_radians().cos(),
            y: 70.0 * 45.0_f64.to_radians().sin(),
        };
        let (p, angle) = apply_polar(
            base,
            cursor,
            &PrecisionState::default().polar_angles_degrees,
            POLAR_TOLERANCE_DEGREES,
        )
        .expect("polar");
        assert!((angle - 45.0).abs() < 1e-6);
        assert!((p.x - cursor.x).abs() < 0.5);
        assert!((p.y - cursor.y).abs() < 0.5);
    }

    #[test]
    fn polar_locks_90_degrees() {
        let base = Point { x: 0.0, y: 0.0 };
        let cursor = Point { x: 1.0, y: 50.0 };
        let (_, angle) = apply_polar(
            base,
            cursor,
            &PrecisionState::default().polar_angles_degrees,
            POLAR_TOLERANCE_DEGREES,
        )
        .expect("polar");
        assert!((angle - 90.0).abs() < 1e-6);
    }

    #[test]
    fn polar_skips_when_outside_tolerance() {
        let base = Point { x: 0.0, y: 0.0 };
        let cursor = Point { x: 50.0, y: 20.0 };
        assert!(apply_polar(
            base,
            cursor,
            &PrecisionState::default().polar_angles_degrees,
            POLAR_TOLERANCE_DEGREES
        )
        .is_none());
    }

    #[test]
    fn osnap_wins_over_polar() {
        let snap = SnapTarget {
            point: Point { x: 1.0, y: 2.0 },
            kind: SnapKind::Endpoint,
        };
        let resolved = resolve_precision_point(
            Point { x: 100.0, y: 100.0 },
            Some(Point { x: 0.0, y: 0.0 }),
            Some(snap),
            &state_polar(),
        );
        assert_eq!(resolved.point, Point { x: 1.0, y: 2.0 });
        assert!(matches!(
            resolved.source,
            PointSource::Osnap(SnapKind::Endpoint)
        ));
    }

    #[test]
    fn raw_cursor_when_all_disabled() {
        let raw = Point { x: 3.0, y: 4.0 };
        let state = PrecisionState {
            ortho_enabled: false,
            polar_enabled: false,
            ..PrecisionState::default()
        };
        let resolved = resolve_precision_point(raw, Some(Point { x: 0.0, y: 0.0 }), None, &state);
        assert_eq!(resolved.point, raw);
        assert_eq!(resolved.source, PointSource::Raw);
    }

    #[test]
    fn dynamic_label_length_angle() {
        let label = dynamic_preview_label(
            Point { x: 0.0, y: 0.0 },
            Point { x: 100.0, y: 100.0 },
            Unit::Millimeters,
            2,
        );
        assert!(label.contains('@'));
        assert!(label.contains('°'));
    }

    #[test]
    fn angle_normalization_0_360() {
        assert!((normalize_degrees(-90.0) - 270.0).abs() < 1e-9);
        assert!((normalize_degrees(450.0) - 90.0).abs() < 1e-9);
    }

    #[test]
    fn rectangle_second_point_with_ortho() {
        let base = Point { x: 0.0, y: 0.0 };
        let corner = Point { x: 8.0, y: 5.0 };
        let locked = apply_ortho(base, corner);
        assert!((locked.x - 8.0).abs() < 1e-9);
        assert!((locked.y - 0.0).abs() < 1e-9);
    }

    #[test]
    fn polyline_next_point_with_polar() {
        let base = Point { x: 0.0, y: 0.0 };
        let cursor = Point { x: 40.0, y: 41.0 };
        let (locked, angle) = apply_polar(
            base,
            cursor,
            &PrecisionState::default().polar_angles_degrees,
            POLAR_TOLERANCE_DEGREES,
        )
        .expect("polar");
        assert!((angle - 45.0).abs() < 1e-6);
        assert!(locked.distance_to(base) > 50.0);
    }

    #[test]
    fn dimension_offset_ortho_from_last_point() {
        let base = Point { x: 10.0, y: 0.0 };
        let offset = Point { x: 14.0, y: 1.0 };
        let locked = apply_ortho(base, offset);
        assert!((locked.x - 14.0).abs() < 1e-9);
        assert!((locked.y - 0.0).abs() < 1e-9);
    }

    #[test]
    fn ortho_over_polar_in_resolve() {
        let state = PrecisionState {
            ortho_enabled: true,
            polar_enabled: true,
            ..PrecisionState::default()
        };
        let base = Point { x: 0.0, y: 0.0 };
        let cursor = Point {
            x: 50.0 * 45.0_f64.to_radians().cos(),
            y: 50.0 * 45.0_f64.to_radians().sin(),
        };
        let resolved = resolve_precision_point(cursor, Some(base), None, &state);
        assert_eq!(resolved.source, PointSource::Ortho);
    }
}
