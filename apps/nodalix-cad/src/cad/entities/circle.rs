use super::entity::BaseEntity;
use crate::cad::geometry::Point2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CircleEntity {
    #[serde(flatten)]
    pub base: BaseEntity,
    pub center: Point2,
    pub radius: f64,
}

impl CircleEntity {
    pub fn new(base: BaseEntity, center: Point2, radius: f64) -> Self {
        Self {
            base,
            center,
            radius,
        }
    }
}
