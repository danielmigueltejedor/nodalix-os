use super::entity::BaseEntity;
use crate::cad::geometry::Point2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HatchEntity {
    #[serde(flatten)]
    pub base: BaseEntity,
    pub boundary: Vec<Point2>,
    pub pattern: String,
    pub scale: f64,
    pub angle: f64,
}

impl HatchEntity {
    pub fn new(
        base: BaseEntity,
        boundary: Vec<Point2>,
        pattern: impl Into<String>,
        scale: f64,
        angle: f64,
    ) -> Self {
        Self {
            base,
            boundary,
            pattern: pattern.into(),
            scale,
            angle,
        }
    }
}
