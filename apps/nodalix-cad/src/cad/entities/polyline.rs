use super::entity::BaseEntity;
use crate::cad::geometry::Point2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PolylineEntity {
    #[serde(flatten)]
    pub base: BaseEntity,
    pub points: Vec<Point2>,
    pub closed: bool,
}

impl PolylineEntity {
    pub fn new(base: BaseEntity, points: Vec<Point2>, closed: bool) -> Self {
        Self {
            base,
            points,
            closed,
        }
    }

    pub fn as_rectangle(start: Point2, end: Point2, base: BaseEntity) -> Self {
        Self {
            base,
            points: vec![
                start,
                Point2::new(end.x, start.y),
                end,
                Point2::new(start.x, end.y),
            ],
            closed: true,
        }
    }
}
