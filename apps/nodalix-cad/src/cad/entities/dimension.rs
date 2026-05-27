use super::entity::BaseEntity;
use crate::cad::geometry::Point2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DimensionEntity {
    #[serde(flatten)]
    pub base: BaseEntity,
    pub start: Point2,
    pub end: Point2,
    pub label: String,
    pub style: String,
    pub precision: u8,
}

impl DimensionEntity {
    pub fn new(
        base: BaseEntity,
        start: Point2,
        end: Point2,
        label: impl Into<String>,
        style: impl Into<String>,
        precision: u8,
    ) -> Self {
        Self {
            base,
            start,
            end,
            label: label.into(),
            style: style.into(),
            precision,
        }
    }
}
