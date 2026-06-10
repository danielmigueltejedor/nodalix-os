use super::{
    BlockReferenceEntity, CircleEntity, DimensionEntity, HatchEntity, LineEntity, PolylineEntity,
    TextEntity,
};
use crate::cad::geometry::{BoundingBox2, Point2};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BaseEntity {
    pub id: String,
    pub layer_id: String,
    pub color: Option<String>,
    pub line_type: Option<String>,
    pub line_weight: Option<f64>,
    pub visible: bool,
    pub locked: bool,
    #[serde(default)]
    pub metadata: BTreeMap<String, serde_json::Value>,
}

impl BaseEntity {
    pub fn new(id: impl Into<String>, layer_id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            layer_id: layer_id.into(),
            visible: true,
            locked: false,
            color: None,
            line_type: None,
            line_weight: None,
            metadata: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CADEntity {
    Line(LineEntity),
    Circle(CircleEntity),
    Polyline(PolylineEntity),
    Text(TextEntity),
    Dimension(DimensionEntity),
    Hatch(HatchEntity),
    BlockReference(BlockReferenceEntity),
}

impl CADEntity {
    pub fn id(&self) -> &str {
        match self {
            CADEntity::Line(e) => &e.base.id,
            CADEntity::Circle(e) => &e.base.id,
            CADEntity::Polyline(e) => &e.base.id,
            CADEntity::Text(e) => &e.base.id,
            CADEntity::Dimension(e) => &e.base.id,
            CADEntity::Hatch(e) => &e.base.id,
            CADEntity::BlockReference(e) => &e.base.id,
        }
    }

    pub fn layer_id(&self) -> &str {
        match self {
            CADEntity::Line(e) => &e.base.layer_id,
            CADEntity::Circle(e) => &e.base.layer_id,
            CADEntity::Polyline(e) => &e.base.layer_id,
            CADEntity::Text(e) => &e.base.layer_id,
            CADEntity::Dimension(e) => &e.base.layer_id,
            CADEntity::Hatch(e) => &e.base.layer_id,
            CADEntity::BlockReference(e) => &e.base.layer_id,
        }
    }

    pub fn base(&self) -> &BaseEntity {
        match self {
            CADEntity::Line(e) => &e.base,
            CADEntity::Circle(e) => &e.base,
            CADEntity::Polyline(e) => &e.base,
            CADEntity::Text(e) => &e.base,
            CADEntity::Dimension(e) => &e.base,
            CADEntity::Hatch(e) => &e.base,
            CADEntity::BlockReference(e) => &e.base,
        }
    }

    pub fn base_mut(&mut self) -> &mut BaseEntity {
        match self {
            CADEntity::Line(e) => &mut e.base,
            CADEntity::Circle(e) => &mut e.base,
            CADEntity::Polyline(e) => &mut e.base,
            CADEntity::Text(e) => &mut e.base,
            CADEntity::Dimension(e) => &mut e.base,
            CADEntity::Hatch(e) => &mut e.base,
            CADEntity::BlockReference(e) => &mut e.base,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            CADEntity::Line(_) => "Line",
            CADEntity::Circle(_) => "Circle",
            CADEntity::Polyline(e) if e.closed => "Rectangle",
            CADEntity::Polyline(_) => "Polyline",
            CADEntity::Text(_) => "Text",
            CADEntity::Dimension(_) => "Dimension",
            CADEntity::Hatch(_) => "Hatch",
            CADEntity::BlockReference(_) => "BlockReference",
        }
    }

    pub fn bounds(&self) -> Option<BoundingBox2> {
        match self {
            CADEntity::Line(e) => BoundingBox2::from_points(&[e.start, e.end]),
            CADEntity::Circle(e) => {
                let r = e.radius.abs();
                Some(BoundingBox2 {
                    min: Point2::new(e.center.x - r, e.center.y - r),
                    max: Point2::new(e.center.x + r, e.center.y + r),
                })
            }
            CADEntity::Polyline(e) => BoundingBox2::from_points(&e.points),
            CADEntity::Text(e) => Some(BoundingBox2 {
                min: e.origin,
                max: e.origin,
            }),
            CADEntity::Dimension(e) => BoundingBox2::from_points(&[e.start, e.end]),
            CADEntity::Hatch(e) => BoundingBox2::from_points(&e.boundary),
            CADEntity::BlockReference(e) => Some(BoundingBox2 {
                min: e.insertion,
                max: e.insertion,
            }),
        }
    }

    pub fn translate(&mut self, dx: f64, dy: f64) {
        match self {
            CADEntity::Line(e) => {
                e.start = e.start.translate(dx, dy);
                e.end = e.end.translate(dx, dy);
            }
            CADEntity::Circle(e) => {
                e.center = e.center.translate(dx, dy);
            }
            CADEntity::Polyline(e) => {
                for point in &mut e.points {
                    *point = point.translate(dx, dy);
                }
            }
            CADEntity::Text(e) => {
                e.origin = e.origin.translate(dx, dy);
            }
            CADEntity::Dimension(e) => {
                e.start = e.start.translate(dx, dy);
                e.end = e.end.translate(dx, dy);
            }
            CADEntity::Hatch(e) => {
                for point in &mut e.boundary {
                    *point = point.translate(dx, dy);
                }
            }
            CADEntity::BlockReference(e) => {
                e.insertion = e.insertion.translate(dx, dy);
            }
        }
    }
}
