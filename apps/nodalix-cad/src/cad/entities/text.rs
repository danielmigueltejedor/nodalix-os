use super::entity::BaseEntity;
use crate::cad::geometry::Point2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextEntity {
    #[serde(flatten)]
    pub base: BaseEntity,
    pub origin: Point2,
    pub text: String,
    pub height: f64,
    pub rotation: f64,
}

impl TextEntity {
    pub fn new(
        base: BaseEntity,
        origin: Point2,
        text: impl Into<String>,
        height: f64,
        rotation: f64,
    ) -> Self {
        Self {
            base,
            origin,
            text: text.into(),
            height,
            rotation,
        }
    }
}
