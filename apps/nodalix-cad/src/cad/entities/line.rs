use super::entity::BaseEntity;
use crate::cad::geometry::Point2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LineEntity {
    #[serde(flatten)]
    pub base: BaseEntity,
    pub start: Point2,
    pub end: Point2,
}

impl LineEntity {
    pub fn new(base: BaseEntity, start: Point2, end: Point2) -> Self {
        Self { base, start, end }
    }

    pub fn length(&self) -> f64 {
        self.start.distance_to(self.end)
    }
}
