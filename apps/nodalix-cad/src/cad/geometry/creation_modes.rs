//! Creation mode identifiers for CAD geometry tools (toolbar / command bar).
//!
// TODO(phase-4): wire toolbar mode pickers to these enums via `cad/tools/draw`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircleCreationMode {
    CenterRadius,
    CenterDiameter,
    TwoPointDiameter,
    ThreePoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RectangleCreationMode {
    TwoCorners,
    CornerDimensions,
    CenterDimensions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineCreationMode {
    TwoPoints,
    PointLengthAngle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArcCreationMode {
    ThreePoint,
    /// TODO(phase-3.12): requires dedicated Arc entity or bulge polyline metadata.
    CenterStartEnd,
}
