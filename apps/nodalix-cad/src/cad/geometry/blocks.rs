//! Block definition geometry: transforms, bounds, explode, instancing.

use crate::document::{BlockDefinition, Document, Entity};
use crate::geometry::Point;
use std::collections::BTreeSet;

pub const MAX_BLOCK_RENDER_DEPTH: usize = 8;

pub fn normalize_block_name(name: &str) -> String {
    name.trim().to_string()
}

pub fn block_scale_y(scale: f64, scale_y: Option<f64>) -> f64 {
    let scale_y = scale_y.unwrap_or(scale);
    if scale_y.is_finite() && scale_y > 0.0 {
        scale_y
    } else {
        scale
    }
}

pub fn transform_point_for_block(
    point: Point,
    base: Point,
    insertion: Point,
    scale_x: f64,
    scale_y: f64,
    rotation: f64,
) -> Point {
    let x = (point.x - base.x) * scale_x;
    let y = (point.y - base.y) * scale_y;
    let cos = rotation.cos();
    let sin = rotation.sin();
    Point {
        x: insertion.x + x * cos - y * sin,
        y: insertion.y + x * sin + y * cos,
    }
}

pub fn transform_entity_for_block_insert(
    entity: &mut Entity,
    base: Point,
    insertion: Point,
    scale_x: f64,
    scale_y: f64,
    rotation: f64,
) {
    let transform = |point: &mut Point| {
        *point = transform_point_for_block(*point, base, insertion, scale_x, scale_y, rotation);
    };
    match entity {
        Entity::Point { point, .. } | Entity::Text { origin: point, .. } => transform(point),
        Entity::BlockReference {
            insertion: point, ..
        } => transform(point),
        Entity::Line { start, end, .. }
        | Entity::Dimension { start, end, .. }
        | Entity::Guideline { start, end, .. } => {
            transform(start);
            transform(end);
        }
        Entity::Polyline { points, .. }
        | Entity::Spline {
            control_points: points,
            ..
        } => {
            for point in points {
                transform(point);
            }
        }
        Entity::Hatch {
            boundary: points, ..
        } => {
            for point in points {
                transform(point);
            }
        }
        Entity::Circle { center, radius, .. } => {
            transform(center);
            let uniform = ((scale_x.abs() + scale_y.abs()) * 0.5).max(f64::EPSILON);
            *radius *= uniform;
        }
        Entity::Table {
            origin,
            cell_width,
            cell_height,
            ..
        } => {
            transform(origin);
            *cell_width *= scale_x.abs().max(f64::EPSILON);
            *cell_height *= scale_y.abs().max(f64::EPSILON);
        }
    }
}

pub fn clone_entity_to_block_space(entity: &Entity, base: Point) -> Entity {
    let mut copy = entity.clone();
    translate_entity(&mut copy, -base.x, -base.y);
    strip_entity_id(&mut copy);
    copy
}

fn strip_entity_id(entity: &mut Entity) {
    match entity {
        Entity::Point { id, .. }
        | Entity::Line { id, .. }
        | Entity::Polyline { id, .. }
        | Entity::Spline { id, .. }
        | Entity::Circle { id, .. }
        | Entity::Text { id, .. }
        | Entity::Dimension { id, .. }
        | Entity::Hatch { id, .. }
        | Entity::Table { id, .. }
        | Entity::BlockReference { id, .. }
        | Entity::Guideline { id, .. } => *id = 0,
    }
}

fn translate_entity(entity: &mut Entity, dx: f64, dy: f64) {
    entity.translate(dx, dy);
}

pub fn block_definition_bounds(def: &BlockDefinition) -> Option<(Point, Point)> {
    entity_group_bounds(&def.entities)
}

pub fn block_reference_world_bounds(
    document: &Document,
    name: &str,
    insertion: Point,
    scale: f64,
    scale_y: Option<f64>,
    rotation: f64,
) -> Option<(Point, Point)> {
    let def = document.block_definition(name)?;
    let scale_x = scale;
    let scale_y = block_scale_y(scale, scale_y);
    let mut bounds: Option<(Point, Point)> = None;
    for entity in &def.entities {
        let mut world = entity.clone();
        transform_entity_for_block_insert(
            &mut world,
            def.base_point,
            insertion,
            scale_x,
            scale_y,
            rotation,
        );
        if let Some(entity_bounds) = entity_bounds_simple(&world) {
            bounds = Some(match bounds {
                Some((min, max)) => merge_bounds(min, max, entity_bounds),
                None => entity_bounds,
            });
        }
    }
    bounds.or_else(|| Some((insertion, insertion)))
}

pub fn instanced_entities(
    document: &Document,
    name: &str,
    insertion: Point,
    scale: f64,
    scale_y: Option<f64>,
    rotation: f64,
    depth: usize,
    visited: &mut BTreeSet<String>,
) -> Vec<Entity> {
    if depth >= MAX_BLOCK_RENDER_DEPTH {
        return Vec::new();
    }
    let key = normalize_block_name(name).to_ascii_uppercase();
    if visited.contains(&key) {
        return Vec::new();
    }
    visited.insert(key.clone());
    let Some(def) = document.block_definition(name) else {
        visited.remove(&key);
        return Vec::new();
    };
    let scale_x = scale;
    let scale_y = block_scale_y(scale, scale_y);
    let mut out = Vec::new();
    for entity in &def.entities {
        if let Entity::BlockReference {
            name: nested,
            insertion: nested_insertion,
            scale: nested_scale,
            scale_y: nested_scale_y,
            rotation: nested_rotation,
            ..
        } = entity
        {
            let nested_insertion = transform_point_for_block(
                *nested_insertion,
                def.base_point,
                insertion,
                scale_x,
                scale_y,
                rotation,
            );
            let nested_scale = scale_x * nested_scale;
            let nested_scale_y = scale_y * block_scale_y(nested_scale, *nested_scale_y);
            let nested_rotation = rotation + nested_rotation;
            out.extend(instanced_entities(
                document,
                nested,
                nested_insertion,
                nested_scale,
                Some(nested_scale_y),
                nested_rotation,
                depth + 1,
                visited,
            ));
            continue;
        }
        let mut world = entity.clone();
        transform_entity_for_block_insert(
            &mut world,
            def.base_point,
            insertion,
            scale_x,
            scale_y,
            rotation,
        );
        out.push(world);
    }
    visited.remove(&key);
    out
}

pub fn explode_block_reference(document: &Document, reference: &Entity) -> Option<Vec<Entity>> {
    let Entity::BlockReference {
        name,
        insertion,
        scale,
        scale_y,
        rotation,
        layer,
        ..
    } = reference
    else {
        return None;
    };
    let mut visited = BTreeSet::new();
    let mut entities = instanced_entities(
        document,
        name,
        *insertion,
        *scale,
        *scale_y,
        *rotation,
        0,
        &mut visited,
    );
    for entity in &mut entities {
        assign_exploded_layer(entity, layer);
        strip_entity_id(entity);
    }
    Some(entities)
}

fn assign_exploded_layer(entity: &mut Entity, reference_layer: &str) {
    match entity {
        Entity::Point { layer, .. }
        | Entity::Line { layer, .. }
        | Entity::Polyline { layer, .. }
        | Entity::Spline { layer, .. }
        | Entity::Circle { layer, .. }
        | Entity::Text { layer, .. }
        | Entity::Dimension { layer, .. }
        | Entity::Hatch { layer, .. }
        | Entity::Table { layer, .. }
        | Entity::BlockReference { layer, .. }
        | Entity::Guideline { layer, .. } => {
            if layer.is_empty() {
                *layer = reference_layer.to_string();
            }
        }
    }
}

pub fn block_reference_hit_distance(
    document: &Document,
    name: &str,
    insertion: Point,
    scale: f64,
    scale_y: Option<f64>,
    rotation: f64,
    point: Point,
) -> Option<f64> {
    let instances = instanced_entities(
        document,
        name,
        insertion,
        scale,
        scale_y,
        rotation,
        0,
        &mut BTreeSet::new(),
    );
    if instances.is_empty() {
        return point.distance_to(insertion).into();
    }
    let mut best = f64::INFINITY;
    for entity in &instances {
        if let Some(distance) = hit_distance_entity(entity, point) {
            best = best.min(distance);
        }
    }
    best.is_finite().then_some(best)
}

fn hit_distance_entity(entity: &Entity, point: Point) -> Option<f64> {
    match entity {
        Entity::Point { point: p, .. } => Some(point.distance_to(*p)),
        Entity::Line { start, end, .. }
        | Entity::Dimension { start, end, .. }
        | Entity::Guideline { start, end, .. } => Some(distance_to_segment(point, *start, *end)),
        Entity::Polyline { points, closed, .. } => polyline_distance(point, points, *closed),
        Entity::Spline {
            control_points,
            closed,
            ..
        } => polyline_distance(point, control_points, *closed),
        Entity::Circle { center, radius, .. } => Some((point.distance_to(*center) - *radius).abs()),
        Entity::Text { origin, .. } | Entity::Table { origin, .. } => {
            Some(point.distance_to(*origin))
        }
        Entity::Hatch { boundary, .. } => polyline_distance(point, boundary, true),
        Entity::BlockReference { insertion, .. } => Some(point.distance_to(*insertion)),
    }
}

fn distance_to_segment(point: Point, start: Point, end: Point) -> f64 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_sq = dx * dx + dy * dy;
    if length_sq <= f64::EPSILON {
        return point.distance_to(start);
    }
    let t = ((point.x - start.x) * dx + (point.y - start.y) * dy) / length_sq;
    let t = t.clamp(0.0, 1.0);
    let closest = Point {
        x: start.x + t * dx,
        y: start.y + t * dy,
    };
    point.distance_to(closest)
}

fn polyline_distance(point: Point, points: &[Point], closed: bool) -> Option<f64> {
    if points.is_empty() {
        return None;
    }
    let mut best = f64::INFINITY;
    for pair in points.windows(2) {
        best = best.min(distance_to_segment(point, pair[0], pair[1]));
    }
    if closed && points.len() > 2 {
        let last = *points.last()?;
        best = best.min(distance_to_segment(point, last, points[0]));
    }
    for vertex in points {
        best = best.min(point.distance_to(*vertex));
    }
    best.is_finite().then_some(best)
}

fn entity_bounds_simple(entity: &Entity) -> Option<(Point, Point)> {
    match entity {
        Entity::Point { point, .. }
        | Entity::Text { origin: point, .. }
        | Entity::BlockReference {
            insertion: point, ..
        } => Some((*point, *point)),
        Entity::Line { start, end, .. }
        | Entity::Dimension { start, end, .. }
        | Entity::Guideline { start, end, .. } => Some(points_bounds([*start, *end])),
        Entity::Polyline { points, .. } => points_iter_bounds(points.iter().copied()),
        Entity::Spline { control_points, .. } => points_iter_bounds(control_points.iter().copied()),
        Entity::Hatch { boundary, .. } => points_iter_bounds(boundary.iter().copied()),
        Entity::Circle { center, radius, .. } => {
            let radius = radius.abs();
            Some((
                Point {
                    x: center.x - radius,
                    y: center.y - radius,
                },
                Point {
                    x: center.x + radius,
                    y: center.y + radius,
                },
            ))
        }
        Entity::Table {
            origin,
            rows,
            columns,
            cell_width,
            cell_height,
            ..
        } => Some(points_bounds([
            *origin,
            Point {
                x: origin.x + *columns as f64 * *cell_width,
                y: origin.y + *rows as f64 * *cell_height,
            },
        ])),
    }
}

fn points_bounds<const N: usize>(points: [Point; N]) -> (Point, Point) {
    let mut min = points[0];
    let mut max = points[0];
    for point in points.into_iter().skip(1) {
        min.x = min.x.min(point.x);
        min.y = min.y.min(point.y);
        max.x = max.x.max(point.x);
        max.y = max.y.max(point.y);
    }
    (min, max)
}

fn points_iter_bounds(mut points: impl Iterator<Item = Point>) -> Option<(Point, Point)> {
    let first = points.next()?;
    let mut min = first;
    let mut max = first;
    for point in points {
        min.x = min.x.min(point.x);
        min.y = min.y.min(point.y);
        max.x = max.x.max(point.x);
        max.y = max.y.max(point.y);
    }
    Some((min, max))
}

fn entity_group_bounds(entities: &[Entity]) -> Option<(Point, Point)> {
    entities
        .iter()
        .filter_map(entity_bounds_simple)
        .reduce(|(amin, amax), (bmin, bmax)| merge_bounds(amin, amax, (bmin, bmax)))
}

fn merge_bounds(min: Point, max: Point, (bmin, bmax): (Point, Point)) -> (Point, Point) {
    (
        Point {
            x: min.x.min(bmin.x),
            y: min.y.min(bmin.y),
        },
        Point {
            x: max.x.max(bmax.x),
            y: max.y.max(bmax.y),
        },
    )
}

pub fn selection_bbox_center(ids: &[u64], document: &Document) -> Option<Point> {
    let mut bounds: Option<(Point, Point)> = None;
    for id in ids {
        let entity = document.entities.iter().find(|e| e.id() == *id)?;
        let entity_bounds = document.cached_entity_bounds(entity).or_else(|| {
            entity_bounds_simple(entity).or_else(|| {
                if let Entity::BlockReference {
                    name,
                    insertion,
                    scale,
                    scale_y,
                    rotation,
                    ..
                } = entity
                {
                    block_reference_world_bounds(
                        document, name, *insertion, *scale, *scale_y, *rotation,
                    )
                } else {
                    None
                }
            })
        })?;
        bounds = Some(match bounds {
            Some((min, max)) => merge_bounds(min, max, entity_bounds),
            None => entity_bounds,
        });
    }
    let (min, max) = bounds?;
    Some(Point {
        x: (min.x + max.x) * 0.5,
        y: (min.y + max.y) * 0.5,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    fn sample_line() -> Entity {
        Entity::Line {
            id: 1,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        }
    }

    #[test]
    fn create_block_definition_from_entities() {
        let mut doc = Document::new_empty();
        doc.insert_block_definition(BlockDefinition {
            name: "Door".to_string(),
            base_point: Point { x: 5.0, y: 5.0 },
            entities: vec![sample_line()],
        });
        let def = doc.block_definition("Door").unwrap();
        assert_eq!(def.entities.len(), 1);
        assert!((def.base_point.x - 5.0).abs() < 1e-9);
    }

    #[test]
    fn insert_transforms_line_endpoints() {
        let mut doc = Document::new_empty();
        doc.insert_block_definition(BlockDefinition {
            name: "B".to_string(),
            base_point: Point { x: 0.0, y: 0.0 },
            entities: vec![sample_line()],
        });
        let instances = instanced_entities(
            &doc,
            "B",
            Point { x: 100.0, y: 0.0 },
            1.0,
            None,
            0.0,
            0,
            &mut BTreeSet::new(),
        );
        if let Entity::Line { start, end, .. } = &instances[0] {
            assert!((start.x - 100.0).abs() < 1e-9);
            assert!((end.x - 110.0).abs() < 1e-9);
        } else {
            panic!("expected line");
        }
    }

    #[test]
    fn rotation_90_transforms_internal_entity() {
        let mut doc = Document::new_empty();
        doc.insert_block_definition(BlockDefinition {
            name: "R".to_string(),
            base_point: Point { x: 0.0, y: 0.0 },
            entities: vec![Entity::Line {
                id: 0,
                layer: "Default".to_string(),
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 10.0, y: 0.0 },
            }],
        });
        let instances = instanced_entities(
            &doc,
            "R",
            Point { x: 0.0, y: 0.0 },
            1.0,
            None,
            std::f64::consts::FRAC_PI_2,
            0,
            &mut BTreeSet::new(),
        );
        if let Entity::Line { end, .. } = &instances[0] {
            assert!((end.x).abs() < 1e-6);
            assert!((end.y - 10.0).abs() < 1e-6);
        } else {
            panic!("expected line");
        }
    }

    #[test]
    fn scale_2_transforms_internal_entity() {
        let mut doc = Document::new_empty();
        doc.insert_block_definition(BlockDefinition {
            name: "S".to_string(),
            base_point: Point { x: 0.0, y: 0.0 },
            entities: vec![sample_line()],
        });
        let instances = instanced_entities(
            &doc,
            "S",
            Point { x: 0.0, y: 0.0 },
            2.0,
            None,
            0.0,
            0,
            &mut BTreeSet::new(),
        );
        if let Entity::Line { end, .. } = &instances[0] {
            assert!((end.x - 20.0).abs() < 1e-9);
        } else {
            panic!("expected line");
        }
    }

    #[test]
    fn hit_test_block_bbox() {
        let mut doc = Document::new_empty();
        doc.insert_block_definition(BlockDefinition {
            name: "H".to_string(),
            base_point: Point { x: 0.0, y: 0.0 },
            entities: vec![sample_line()],
        });
        let distance = block_reference_hit_distance(
            &doc,
            "H",
            Point { x: 0.0, y: 0.0 },
            1.0,
            None,
            0.0,
            Point { x: 5.0, y: 0.0 },
        )
        .unwrap();
        assert!(distance < 0.1);
    }

    #[test]
    fn explode_block_creates_transformed_entities() {
        let mut doc = Document::new_empty();
        doc.insert_block_definition(BlockDefinition {
            name: "E".to_string(),
            base_point: Point { x: 0.0, y: 0.0 },
            entities: vec![sample_line()],
        });
        let reference = Entity::BlockReference {
            id: 99,
            layer: "Default".to_string(),
            name: "E".to_string(),
            insertion: Point { x: 50.0, y: 0.0 },
            scale: 1.0,
            scale_y: None,
            rotation: 0.0,
        };
        let exploded = explode_block_reference(&doc, &reference).unwrap();
        assert_eq!(exploded.len(), 1);
        if let Entity::Line { start, .. } = &exploded[0] {
            assert!((start.x - 50.0).abs() < 1e-9);
        } else {
            panic!("expected line");
        }
    }

    #[test]
    fn avoid_infinite_recursion_on_self_reference() {
        let mut doc = Document::new_empty();
        doc.insert_block_definition(BlockDefinition {
            name: "Loop".to_string(),
            base_point: Point { x: 0.0, y: 0.0 },
            entities: vec![Entity::BlockReference {
                id: 0,
                layer: "Default".to_string(),
                name: "Loop".to_string(),
                insertion: Point { x: 0.0, y: 0.0 },
                scale: 1.0,
                scale_y: None,
                rotation: 0.0,
            }],
        });
        let instances = instanced_entities(
            &doc,
            "Loop",
            Point { x: 0.0, y: 0.0 },
            1.0,
            None,
            0.0,
            0,
            &mut BTreeSet::new(),
        );
        assert!(instances.is_empty());
    }

    #[test]
    fn save_load_preserves_block_definitions() {
        let mut doc = Document::new_empty();
        doc.insert_block_definition(BlockDefinition {
            name: "Persist".to_string(),
            base_point: Point { x: 1.0, y: 2.0 },
            entities: vec![sample_line()],
        });
        let json = serde_json::to_string(&doc).unwrap();
        let loaded: Document = serde_json::from_str(&json).unwrap();
        assert!(loaded.block_definition("Persist").is_some());
    }
}
