//! Object snap (OSNAP) engine — pure geometry, no GTK.

use crate::document::{Document, Entity};
use crate::geometry::Point;

pub const SNAP_RADIUS_PX: f64 = 10.0;
pub const FULL_SNAP_ENTITY_LIMIT: usize = 2_500;

/// Global OSNAP toggles (toolbar / command bar).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OsnapState {
    pub enabled: bool,
    pub endpoint: bool,
    pub midpoint: bool,
    pub center: bool,
    pub intersection: bool,
    pub perpendicular: bool,
    pub tangent: bool,
    pub nearest: bool,
    pub quadrant: bool,
    pub node: bool,
}

impl Default for OsnapState {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint: true,
            midpoint: true,
            center: true,
            intersection: true,
            perpendicular: false,
            tangent: false,
            nearest: false,
            quadrant: true,
            node: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapKind {
    Endpoint,
    Midpoint,
    Center,
    Quadrant,
    Intersection,
    Perpendicular,
    Tangent,
    Nearest,
    Node,
    /// Internal ortho helper while placing (not in OSNAP panel).
    Ortho,
    /// Internal parallel helper while placing.
    Parallel,
}

impl SnapKind {
    pub fn label(self) -> &'static str {
        match self {
            SnapKind::Endpoint => "END",
            SnapKind::Midpoint => "MID",
            SnapKind::Center => "CEN",
            SnapKind::Quadrant => "QUAD",
            SnapKind::Intersection => "INT",
            SnapKind::Perpendicular => "PERP",
            SnapKind::Tangent => "TAN",
            SnapKind::Nearest => "NEAR",
            SnapKind::Node => "NODE",
            SnapKind::Ortho => "ORTHO",
            SnapKind::Parallel => "PAR",
        }
    }

    pub fn priority(self) -> u8 {
        match self {
            SnapKind::Endpoint | SnapKind::Intersection | SnapKind::Center => 100,
            SnapKind::Midpoint | SnapKind::Quadrant | SnapKind::Node => 80,
            SnapKind::Perpendicular | SnapKind::Tangent => 60,
            SnapKind::Nearest => 40,
            SnapKind::Ortho | SnapKind::Parallel => 70,
        }
    }

    pub fn allowed(self, state: &OsnapState) -> bool {
        if !state.enabled {
            return false;
        }
        match self {
            SnapKind::Endpoint => state.endpoint,
            SnapKind::Midpoint => state.midpoint,
            SnapKind::Center => state.center,
            SnapKind::Quadrant => state.quadrant,
            SnapKind::Intersection => state.intersection,
            SnapKind::Perpendicular => state.perpendicular,
            SnapKind::Tangent => state.tangent,
            SnapKind::Nearest => state.nearest,
            SnapKind::Node => state.node,
            SnapKind::Ortho | SnapKind::Parallel => true,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SnapCandidate {
    pub kind: SnapKind,
    pub point: Point,
    pub entity_id: Option<u64>,
    pub distance_screen: f64,
    pub priority: u8,
}

impl SnapCandidate {
    fn new(
        kind: SnapKind,
        point: Point,
        entity_id: Option<u64>,
        pointer: Point,
        zoom: f64,
    ) -> Self {
        let world_distance = point.distance_to(pointer);
        Self {
            kind,
            point,
            entity_id,
            distance_screen: world_distance * zoom,
            priority: kind.priority(),
        }
    }
}

/// Resolved snap passed to the canvas for drawing and click substitution.
#[derive(Clone, Copy, Debug)]
pub struct SnapTarget {
    pub point: Point,
    pub kind: SnapKind,
}

pub fn world_tolerance(zoom: f64) -> f64 {
    SNAP_RADIUS_PX / zoom.max(1e-9)
}

pub fn find_snap(
    document: &Document,
    pointer: Point,
    pending_start: Option<Point>,
    zoom: f64,
    state: &OsnapState,
) -> Option<SnapTarget> {
    if !state.enabled {
        return None;
    }
    let tolerance = world_tolerance(zoom);
    let mut candidates = Vec::new();
    collect_point_snaps(document, pointer, tolerance, zoom, state, &mut candidates);
    if document.entities.len() <= FULL_SNAP_ENTITY_LIMIT {
        if state.intersection {
            collect_intersection_snaps(document, pointer, zoom, &mut candidates);
        }
        if state.midpoint {
            collect_derived_midpoint_snaps(document, pointer, zoom, &mut candidates);
        }
    }
    if state.nearest {
        collect_nearest_snaps(document, pointer, tolerance, zoom, &mut candidates);
    }
    if let Some(start) = pending_start {
        if state.perpendicular {
            collect_perpendicular_snaps(document, pointer, start, zoom, &mut candidates);
        }
        // Ortho cursor lock is handled by `cad::precision` (F8), not OSNAP candidates.
        if let Some(parallel) = find_parallel_snap(document, pointer, start, tolerance * 0.45, zoom)
        {
            candidates.push(parallel);
        }
    }
    pick_best_candidate(&candidates, SNAP_RADIUS_PX).map(|c| SnapTarget {
        point: c.point,
        kind: c.kind,
    })
}

pub fn pick_best_candidate<'a>(
    candidates: &'a [SnapCandidate],
    max_screen_px: f64,
) -> Option<&'a SnapCandidate> {
    candidates
        .iter()
        .filter(|c| c.distance_screen <= max_screen_px)
        .max_by(|a, b| {
            a.priority.cmp(&b.priority).then_with(|| {
                b.distance_screen
                    .partial_cmp(&a.distance_screen)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        })
}

fn entity_eligible(document: &Document, entity: &Entity) -> bool {
    document.entity_visible_in_active_layout(entity.id())
}

fn push_if_enabled(
    state: &OsnapState,
    kind: SnapKind,
    point: Point,
    entity_id: Option<u64>,
    pointer: Point,
    zoom: f64,
    out: &mut Vec<SnapCandidate>,
) {
    if kind.allowed(state) {
        out.push(SnapCandidate::new(kind, point, entity_id, pointer, zoom));
    }
}

fn collect_point_snaps(
    document: &Document,
    pointer: Point,
    tolerance: f64,
    zoom: f64,
    state: &OsnapState,
    out: &mut Vec<SnapCandidate>,
) {
    let query_bounds = (
        Point {
            x: pointer.x - tolerance,
            y: pointer.y - tolerance,
        },
        Point {
            x: pointer.x + tolerance,
            y: pointer.y + tolerance,
        },
    );
    for entity in document
        .entities
        .iter()
        .filter(|e| entity_eligible(document, e))
    {
        if let Some(bounds) = document.cached_entity_bounds(entity) {
            if !bounds_intersect(bounds, query_bounds) {
                continue;
            }
        }
        let id = entity.id();
        match entity {
            Entity::Point { point, .. } => {
                if point.distance_to(pointer) <= tolerance {
                    push_if_enabled(state, SnapKind::Node, *point, Some(id), pointer, zoom, out);
                }
            }
            Entity::Line { start, end, .. }
            | Entity::Dimension { start, end, .. }
            | Entity::Guideline { start, end, .. } => {
                push_if_enabled(
                    state,
                    SnapKind::Endpoint,
                    *start,
                    Some(id),
                    pointer,
                    zoom,
                    out,
                );
                push_if_enabled(
                    state,
                    SnapKind::Endpoint,
                    *end,
                    Some(id),
                    pointer,
                    zoom,
                    out,
                );
                let mid = midpoint(*start, *end);
                if mid.distance_to(pointer) <= tolerance {
                    push_if_enabled(state, SnapKind::Midpoint, mid, Some(id), pointer, zoom, out);
                }
            }
            Entity::Polyline { points, closed, .. } => {
                collect_polyline_corners(points, *closed, id, pointer, tolerance, state, zoom, out);
            }
            Entity::Spline {
                control_points,
                closed,
                ..
            } => {
                let display = spline_display_points(control_points, *closed);
                collect_polyline_corners(
                    &display, *closed, id, pointer, tolerance, state, zoom, out,
                );
                for cp in control_points {
                    if cp.distance_to(pointer) <= tolerance {
                        push_if_enabled(
                            state,
                            SnapKind::Endpoint,
                            *cp,
                            Some(id),
                            pointer,
                            zoom,
                            out,
                        );
                    }
                }
            }
            Entity::Hatch { boundary, .. } => {
                collect_polyline_corners(boundary, true, id, pointer, tolerance, state, zoom, out);
            }
            Entity::Circle { center, radius, .. } => {
                if center.distance_to(pointer) <= tolerance {
                    push_if_enabled(
                        state,
                        SnapKind::Center,
                        *center,
                        Some(id),
                        pointer,
                        zoom,
                        out,
                    );
                }
                if state.quadrant {
                    for quad in circle_quadrants(*center, *radius) {
                        if quad.distance_to(pointer) <= tolerance {
                            push_if_enabled(
                                state,
                                SnapKind::Quadrant,
                                quad,
                                Some(id),
                                pointer,
                                zoom,
                                out,
                            );
                        }
                    }
                }
            }
            Entity::Text { origin, .. }
            | Entity::Table { origin, .. }
            | Entity::BlockReference {
                insertion: origin, ..
            } => {
                if origin.distance_to(pointer) <= tolerance {
                    push_if_enabled(state, SnapKind::Node, *origin, Some(id), pointer, zoom, out);
                }
            }
        }
    }
}

fn collect_polyline_corners(
    points: &[Point],
    closed: bool,
    entity_id: u64,
    pointer: Point,
    tolerance: f64,
    state: &OsnapState,
    zoom: f64,
    out: &mut Vec<SnapCandidate>,
) {
    for point in points {
        if point.distance_to(pointer) <= tolerance {
            push_if_enabled(
                state,
                SnapKind::Endpoint,
                *point,
                Some(entity_id),
                pointer,
                zoom,
                out,
            );
        }
    }
    for pair in points.windows(2) {
        let mid = midpoint(pair[0], pair[1]);
        if mid.distance_to(pointer) <= tolerance {
            push_if_enabled(
                state,
                SnapKind::Midpoint,
                mid,
                Some(entity_id),
                pointer,
                zoom,
                out,
            );
        }
    }
    if closed && points.len() > 2 {
        let mid = midpoint(*points.last().unwrap_or(&points[0]), points[0]);
        if mid.distance_to(pointer) <= tolerance {
            push_if_enabled(
                state,
                SnapKind::Midpoint,
                mid,
                Some(entity_id),
                pointer,
                zoom,
                out,
            );
        }
    }
}

fn collect_intersection_snaps(
    document: &Document,
    pointer: Point,
    zoom: f64,
    out: &mut Vec<SnapCandidate>,
) {
    let segments = entity_segments(document);
    for (index, first) in segments.iter().enumerate() {
        for second in segments.iter().skip(index + 1) {
            if let Some(point) = segment_intersection(first.0, first.1, second.0, second.1) {
                out.push(SnapCandidate::new(
                    SnapKind::Intersection,
                    point,
                    None,
                    pointer,
                    zoom,
                ));
            }
        }
    }
}

fn collect_derived_midpoint_snaps(
    document: &Document,
    pointer: Point,
    zoom: f64,
    out: &mut Vec<SnapCandidate>,
) {
    let segments = entity_segments(document);
    for (index, &(a, b)) in segments.iter().enumerate() {
        let mut cuts = vec![0.0, 1.0];
        for (other_index, &(c, d)) in segments.iter().enumerate() {
            if index == other_index {
                continue;
            }
            if let Some((_, t)) = segment_intersection_with_t(a, b, c, d) {
                if t > 1e-6 && t < 1.0 - 1e-6 {
                    cuts.push(t);
                }
            }
        }
        cuts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        for pair in cuts.windows(2) {
            let t = (pair[0] + pair[1]) * 0.5;
            out.push(SnapCandidate::new(
                SnapKind::Midpoint,
                point_on_segment(a, b, t),
                None,
                pointer,
                zoom,
            ));
        }
    }
}

fn collect_nearest_snaps(
    document: &Document,
    pointer: Point,
    tolerance: f64,
    zoom: f64,
    out: &mut Vec<SnapCandidate>,
) {
    for entity in document
        .entities
        .iter()
        .filter(|e| entity_eligible(document, e))
    {
        let id = entity.id();
        match entity {
            Entity::Line { start, end, .. }
            | Entity::Dimension { start, end, .. }
            | Entity::Guideline { start, end, .. } => {
                let closest = project_point_to_segment(pointer, *start, *end);
                if closest.distance_to(pointer) <= tolerance {
                    out.push(SnapCandidate::new(
                        SnapKind::Nearest,
                        closest,
                        Some(id),
                        pointer,
                        zoom,
                    ));
                }
            }
            Entity::Circle { center, radius, .. } => {
                if let Some(edge) = circle_edge_point(*center, *radius, pointer) {
                    if edge.distance_to(pointer) <= tolerance {
                        out.push(SnapCandidate::new(
                            SnapKind::Nearest,
                            edge,
                            Some(id),
                            pointer,
                            zoom,
                        ));
                    }
                }
            }
            Entity::Polyline { points, closed, .. } => {
                collect_nearest_on_polyline(points, *closed, id, pointer, tolerance, zoom, out);
            }
            Entity::Spline {
                control_points,
                closed,
                ..
            } => {
                let display = spline_display_points(control_points, *closed);
                collect_nearest_on_polyline(&display, *closed, id, pointer, tolerance, zoom, out);
            }
            Entity::Hatch { boundary, .. } => {
                collect_nearest_on_polyline(boundary, true, id, pointer, tolerance, zoom, out);
            }
            _ => {}
        }
    }
}

fn collect_nearest_on_polyline(
    points: &[Point],
    closed: bool,
    entity_id: u64,
    pointer: Point,
    tolerance: f64,
    zoom: f64,
    out: &mut Vec<SnapCandidate>,
) {
    for pair in points.windows(2) {
        let closest = project_point_to_segment(pointer, pair[0], pair[1]);
        if closest.distance_to(pointer) <= tolerance {
            out.push(SnapCandidate::new(
                SnapKind::Nearest,
                closest,
                Some(entity_id),
                pointer,
                zoom,
            ));
        }
    }
    if closed && points.len() > 2 {
        let closest =
            project_point_to_segment(pointer, *points.last().unwrap_or(&points[0]), points[0]);
        if closest.distance_to(pointer) <= tolerance {
            out.push(SnapCandidate::new(
                SnapKind::Nearest,
                closest,
                Some(entity_id),
                pointer,
                zoom,
            ));
        }
    }
}

fn collect_perpendicular_snaps(
    document: &Document,
    pointer: Point,
    start: Point,
    zoom: f64,
    out: &mut Vec<SnapCandidate>,
) {
    for (a, b) in entity_segments(document) {
        let projected = project_point_to_segment(start, a, b);
        if projected.distance_to(pointer) < pointer.distance_to(start).max(1.0) {
            out.push(SnapCandidate::new(
                SnapKind::Perpendicular,
                projected,
                None,
                pointer,
                zoom,
            ));
        }
    }
}

fn find_parallel_snap(
    document: &Document,
    pointer: Point,
    start: Point,
    tolerance: f64,
    zoom: f64,
) -> Option<SnapCandidate> {
    let pointer_dx = pointer.x - start.x;
    let pointer_dy = pointer.y - start.y;
    let pointer_length = (pointer_dx * pointer_dx + pointer_dy * pointer_dy).sqrt();
    if pointer_length <= 1e-9 {
        return None;
    }

    let mut best = None::<SnapCandidate>;
    for (a, b) in entity_segments(document) {
        let segment_dx = b.x - a.x;
        let segment_dy = b.y - a.y;
        let segment_length = (segment_dx * segment_dx + segment_dy * segment_dy).sqrt();
        if segment_length <= 1e-9 {
            continue;
        }
        let ux = segment_dx / segment_length;
        let uy = segment_dy / segment_length;
        let dot = pointer_dx * ux + pointer_dy * uy;
        let projected = Point {
            x: start.x + ux * dot,
            y: start.y + uy * dot,
        };
        let distance = projected.distance_to(pointer);
        if distance <= tolerance.min(pointer_length.max(1.0) * 0.02) {
            let candidate = SnapCandidate::new(SnapKind::Parallel, projected, None, pointer, zoom);
            if best
                .as_ref()
                .map(|b| distance < b.distance_screen / zoom.max(1e-9))
                .unwrap_or(true)
            {
                best = Some(candidate);
            }
        }
    }
    best
}

fn collect_ortho_snap(pointer: Point, start: Point, zoom: f64, out: &mut Vec<SnapCandidate>) {
    let dx = (pointer.x - start.x).abs();
    let dy = (pointer.y - start.y).abs();
    let ortho_point = if dx < dy {
        Point {
            x: start.x,
            y: pointer.y,
        }
    } else {
        Point {
            x: pointer.x,
            y: start.y,
        }
    };
    out.push(SnapCandidate::new(
        SnapKind::Ortho,
        ortho_point,
        None,
        pointer,
        zoom,
    ));
}

fn entity_segments(document: &Document) -> Vec<(Point, Point)> {
    let mut segments = Vec::new();
    for entity in document
        .entities
        .iter()
        .filter(|e| entity_eligible(document, e))
    {
        match entity {
            Entity::Line { start, end, .. }
            | Entity::Dimension { start, end, .. }
            | Entity::Guideline { start, end, .. } => segments.push((*start, *end)),
            Entity::Polyline { points, closed, .. } => {
                push_polyline_segments(&mut segments, points, *closed);
            }
            Entity::Spline {
                control_points,
                closed,
                ..
            } => {
                let points = spline_display_points(control_points, *closed);
                push_polyline_segments(&mut segments, &points, *closed);
            }
            Entity::Hatch { boundary, .. } => push_polyline_segments(&mut segments, boundary, true),
            Entity::Table {
                origin,
                rows,
                columns,
                cell_width,
                cell_height,
                ..
            } => {
                let width = *columns as f64 * *cell_width;
                let height = *rows as f64 * *cell_height;
                let a = *origin;
                let b = Point {
                    x: origin.x + width,
                    y: origin.y,
                };
                let c = Point {
                    x: origin.x + width,
                    y: origin.y + height,
                };
                let d = Point {
                    x: origin.x,
                    y: origin.y + height,
                };
                segments.extend([(a, b), (b, c), (c, d), (d, a)]);
            }
            _ => {}
        }
    }
    segments
}

fn push_polyline_segments(segments: &mut Vec<(Point, Point)>, points: &[Point], closed: bool) {
    for pair in points.windows(2) {
        segments.push((pair[0], pair[1]));
    }
    if closed && points.len() > 2 {
        segments.push((*points.last().unwrap_or(&points[0]), points[0]));
    }
}

pub fn segment_intersection(a: Point, b: Point, c: Point, d: Point) -> Option<Point> {
    segment_intersection_with_t(a, b, c, d).map(|(point, _)| point)
}

pub fn segment_intersection_with_t(a: Point, b: Point, c: Point, d: Point) -> Option<(Point, f64)> {
    let denominator = (a.x - b.x) * (c.y - d.y) - (a.y - b.y) * (c.x - d.x);
    if denominator.abs() < 1e-9 {
        return None;
    }
    let t = ((a.x - c.x) * (c.y - d.y) - (a.y - c.y) * (c.x - d.x)) / denominator;
    let u = -((a.x - b.x) * (a.y - c.y) - (a.y - b.y) * (a.x - c.x)) / denominator;
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some((point_on_segment(a, b, t), t))
    } else {
        None
    }
}

pub fn point_on_segment(start: Point, end: Point, t: f64) -> Point {
    Point {
        x: start.x + t * (end.x - start.x),
        y: start.y + t * (end.y - start.y),
    }
}

pub fn project_point_to_segment(point: Point, start: Point, end: Point) -> Point {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_squared = dx * dx + dy * dy;
    if length_squared <= f64::EPSILON {
        return start;
    }
    let t =
        (((point.x - start.x) * dx + (point.y - start.y) * dy) / length_squared).clamp(0.0, 1.0);
    Point {
        x: start.x + t * dx,
        y: start.y + t * dy,
    }
}

pub fn nearest_on_segment(point: Point, start: Point, end: Point) -> Point {
    project_point_to_segment(point, start, end)
}

fn circle_edge_point(center: Point, radius: f64, point: Point) -> Option<Point> {
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    let length = (dx * dx + dy * dy).sqrt();
    if length <= f64::EPSILON || radius <= 0.0 {
        return None;
    }
    Some(Point {
        x: center.x + dx / length * radius,
        y: center.y + dy / length * radius,
    })
}

pub fn circle_quadrants(center: Point, radius: f64) -> [Point; 4] {
    [
        Point {
            x: center.x + radius,
            y: center.y,
        },
        Point {
            x: center.x,
            y: center.y + radius,
        },
        Point {
            x: center.x - radius,
            y: center.y,
        },
        Point {
            x: center.x,
            y: center.y - radius,
        },
    ]
}

pub fn midpoint(a: Point, b: Point) -> Point {
    Point {
        x: (a.x + b.x) * 0.5,
        y: (a.y + b.y) * 0.5,
    }
}

fn bounds_intersect(a: (Point, Point), b: (Point, Point)) -> bool {
    let (a_min, a_max) = normalized_bounds(a);
    let (b_min, b_max) = normalized_bounds(b);
    a_min.x <= b_max.x && a_max.x >= b_min.x && a_min.y <= b_max.y && a_max.y >= b_min.y
}

fn normalized_bounds(bounds: (Point, Point)) -> (Point, Point) {
    (
        Point {
            x: bounds.0.x.min(bounds.1.x),
            y: bounds.0.y.min(bounds.1.y),
        },
        Point {
            x: bounds.0.x.max(bounds.1.x),
            y: bounds.0.y.max(bounds.1.y),
        },
    )
}

fn spline_display_points(points: &[Point], closed: bool) -> Vec<Point> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut display = Vec::with_capacity(points.len() * 8);
    let count = points.len();
    let segment_count = if closed { count } else { count - 1 };
    for index in 0..segment_count {
        let p0 = if index == 0 {
            if closed {
                points[count - 1]
            } else {
                points[0]
            }
        } else {
            points[index - 1]
        };
        let p1 = points[index];
        let p2 = points[(index + 1) % count];
        let p3 = if index + 2 < count {
            points[index + 2]
        } else if closed {
            points[(index + 2) % count]
        } else {
            points[count - 1]
        };
        for step in 0..8 {
            let t = step as f64 / 8.0;
            display.push(catmull_rom_point(p0, p1, p2, p3, t));
        }
    }
    if !closed {
        display.push(*points.last().unwrap_or(&points[0]));
    }
    display
}

fn catmull_rom_point(p0: Point, p1: Point, p2: Point, p3: Point, t: f64) -> Point {
    let t2 = t * t;
    let t3 = t2 * t;
    Point {
        x: 0.5
            * ((2.0 * p1.x)
                + (-p0.x + p2.x) * t
                + (2.0 * p0.x - 5.0 * p1.x + 4.0 * p2.x - p3.x) * t2
                + (-p0.x + 3.0 * p1.x - 3.0 * p2.x + p3.x) * t3),
        y: 0.5
            * ((2.0 * p1.y)
                + (-p0.y + p2.y) * t
                + (2.0 * p0.y - 5.0 * p1.y + 4.0 * p2.y - p3.y) * t2
                + (-p0.y + 3.0 * p1.y - 3.0 * p2.y + p3.y) * t3),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    fn line_doc(x1: f64, y1: f64, x2: f64, y2: f64) -> Document {
        let mut doc = Document::new_empty();
        doc.add_entity(Entity::Line {
            id: 1,
            layer: "Default".to_string(),
            start: Point { x: x1, y: y1 },
            end: Point { x: x2, y: y2 },
        });
        doc
    }

    #[test]
    fn line_endpoint_snap() {
        let doc = line_doc(0.0, 0.0, 100.0, 0.0);
        let state = OsnapState::default();
        let snap =
            find_snap(&doc, Point { x: 0.5, y: 0.2 }, None, 1.0, &state).expect("endpoint snap");
        assert_eq!(snap.kind, SnapKind::Endpoint);
        assert!((snap.point.x - 0.0).abs() < 1e-6);
    }

    #[test]
    fn line_midpoint_snap() {
        let doc = line_doc(0.0, 0.0, 100.0, 0.0);
        let state = OsnapState::default();
        let snap =
            find_snap(&doc, Point { x: 50.0, y: 0.3 }, None, 1.0, &state).expect("midpoint snap");
        assert_eq!(snap.kind, SnapKind::Midpoint);
        assert!((snap.point.x - 50.0).abs() < 1e-6);
    }

    #[test]
    fn circle_center_snap() {
        let mut doc = Document::new_empty();
        doc.add_entity(Entity::Circle {
            id: 2,
            layer: "Default".to_string(),
            center: Point { x: 10.0, y: 20.0 },
            radius: 5.0,
        });
        let state = OsnapState::default();
        let snap =
            find_snap(&doc, Point { x: 10.2, y: 20.1 }, None, 1.0, &state).expect("center snap");
        assert_eq!(snap.kind, SnapKind::Center);
    }

    #[test]
    fn circle_quadrant_snap() {
        let mut doc = Document::new_empty();
        doc.add_entity(Entity::Circle {
            id: 2,
            layer: "Default".to_string(),
            center: Point { x: 0.0, y: 0.0 },
            radius: 10.0,
        });
        let state = OsnapState::default();
        let snap =
            find_snap(&doc, Point { x: 10.0, y: 0.4 }, None, 1.0, &state).expect("quadrant snap");
        assert_eq!(snap.kind, SnapKind::Quadrant);
        assert!((snap.point.x - 10.0).abs() < 1e-6);
    }

    #[test]
    fn line_line_intersection_snap() {
        let mut doc = Document::new_empty();
        doc.add_entity(Entity::Line {
            id: 1,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 10.0 },
        });
        doc.add_entity(Entity::Line {
            id: 2,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 10.0 },
            end: Point { x: 10.0, y: 0.0 },
        });
        let state = OsnapState::default();
        let snap = find_snap(&doc, Point { x: 5.0, y: 5.2 }, None, 1.0, &state)
            .expect("intersection snap");
        assert_eq!(snap.kind, SnapKind::Intersection);
        assert!((snap.point.x - 5.0).abs() < 1e-6 && (snap.point.y - 5.0).abs() < 1e-6);
    }

    #[test]
    fn nearest_on_line() {
        let doc = line_doc(0.0, 0.0, 100.0, 0.0);
        let mut state = OsnapState::default();
        state.nearest = true;
        state.endpoint = false;
        state.midpoint = false;
        let snap =
            find_snap(&doc, Point { x: 25.0, y: 2.0 }, None, 1.0, &state).expect("nearest snap");
        assert_eq!(snap.kind, SnapKind::Nearest);
        assert!((snap.point.x - 25.0).abs() < 1e-6 && snap.point.y.abs() < 1e-6);
    }

    #[test]
    fn snap_priority_endpoint_over_nearest() {
        let doc = line_doc(0.0, 0.0, 100.0, 0.0);
        let mut state = OsnapState::default();
        state.nearest = true;
        let mut candidates = Vec::new();
        candidates.push(SnapCandidate::new(
            SnapKind::Nearest,
            Point { x: 0.0, y: 0.0 },
            Some(1),
            Point { x: 0.2, y: 0.2 },
            1.0,
        ));
        candidates.push(SnapCandidate::new(
            SnapKind::Endpoint,
            Point { x: 0.0, y: 0.0 },
            Some(1),
            Point { x: 0.2, y: 0.2 },
            1.0,
        ));
        let best = pick_best_candidate(&candidates, SNAP_RADIUS_PX).unwrap();
        assert_eq!(best.kind, SnapKind::Endpoint);
    }

    #[test]
    fn screen_tolerance_scales_with_zoom() {
        assert!((world_tolerance(2.0) - 5.0).abs() < 1e-9);
        assert!((world_tolerance(0.5) - 20.0).abs() < 1e-9);
    }

    #[test]
    fn hidden_layer_no_snap() {
        let mut doc = line_doc(0.0, 0.0, 100.0, 0.0);
        doc.create_layer("Hidden");
        doc.set_entity_layer(1, "Hidden");
        doc.set_layer_visible("Hidden", false);
        let state = OsnapState::default();
        assert!(find_snap(&doc, Point { x: 0.2, y: 0.0 }, None, 1.0, &state).is_none());
    }

    #[test]
    fn locked_layer_still_snaps() {
        let mut doc = line_doc(0.0, 0.0, 100.0, 0.0);
        doc.create_layer("Locked");
        doc.set_entity_layer(1, "Locked");
        doc.set_layer_locked("Locked", true);
        let state = OsnapState::default();
        assert!(find_snap(&doc, Point { x: 0.2, y: 0.0 }, None, 1.0, &state).is_some());
    }

    #[test]
    fn polyline_vertices_endpoint() {
        let mut doc = Document::new_empty();
        doc.add_entity(Entity::Polyline {
            id: 3,
            layer: "Default".to_string(),
            points: vec![Point { x: 0.0, y: 0.0 }, Point { x: 10.0, y: 0.0 }],
            closed: false,
        });
        let state = OsnapState::default();
        let snap = find_snap(&doc, Point { x: 10.05, y: 0.05 }, None, 1.0, &state)
            .expect("polyline endpoint");
        assert_eq!(snap.kind, SnapKind::Endpoint);
        assert!((snap.point.x - 10.0).abs() < 1e-6 && snap.point.y.abs() < 1e-6);
    }

    #[test]
    fn text_insertion_node_snap() {
        let mut doc = Document::new_empty();
        doc.add_entity(Entity::Text {
            id: 4,
            layer: "Default".to_string(),
            origin: Point { x: 5.0, y: 5.0 },
            text: "Hi".to_string(),
            height: 2.5,
            rotation: 0.0,
        });
        let state = OsnapState::default();
        let snap = find_snap(&doc, Point { x: 5.1, y: 5.1 }, None, 1.0, &state).expect("text node");
        assert_eq!(snap.kind, SnapKind::Node);
    }

    #[test]
    fn snap_disabled_returns_none() {
        let doc = line_doc(0.0, 0.0, 10.0, 0.0);
        let mut state = OsnapState::default();
        state.enabled = false;
        assert!(find_snap(&doc, Point { x: 0.0, y: 0.0 }, None, 1.0, &state).is_none());
    }

    #[test]
    fn best_candidate_picks_closest_when_same_priority() {
        let candidates = vec![
            SnapCandidate::new(
                SnapKind::Endpoint,
                Point { x: 0.0, y: 0.0 },
                None,
                Point { x: 1.0, y: 0.0 },
                1.0,
            ),
            SnapCandidate::new(
                SnapKind::Endpoint,
                Point { x: 2.0, y: 0.0 },
                None,
                Point { x: 2.5, y: 0.0 },
                1.0,
            ),
        ];
        let best = pick_best_candidate(&candidates, SNAP_RADIUS_PX).unwrap();
        assert!((best.point.x - 2.0).abs() < 1e-6);
    }
}
