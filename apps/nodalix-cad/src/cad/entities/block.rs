use super::entity::BaseEntity;
use crate::cad::geometry::Point2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BlockReferenceEntity {
    #[serde(flatten)]
    pub base: BaseEntity,
    pub name: String,
    pub insertion: Point2,
    pub scale: f64,
    pub rotation: f64,
}

impl BlockReferenceEntity {
    pub fn new(
        base: BaseEntity,
        name: impl Into<String>,
        insertion: Point2,
        scale: f64,
        rotation: f64,
    ) -> Self {
        Self {
            base,
            name: name.into(),
            insertion,
            scale,
            rotation,
        }
    }
}
