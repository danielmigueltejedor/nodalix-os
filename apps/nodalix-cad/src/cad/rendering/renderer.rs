use crate::cad::document::CADDocument;
use crate::cad::entities::CADEntity;
use crate::cad::geometry::Point2;

/// View transform passed to renderers (decoupled from GTK canvas camera).
#[derive(Clone, Copy, Debug, Default)]
pub struct ViewState {
    pub zoom: f64,
    pub pan: Point2,
    pub rotation: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, Default)]
pub struct RenderPreview {
    pub segments: Vec<(Point2, Point2)>,
    pub points: Vec<Point2>,
}

/// Abstraction for CAD drawing backends (Cairo canvas today, WebGL later).
///
// TODO(phase-2): implement `CanvasRenderer` in `canvas.rs` using this trait.
pub trait CADRenderer {
    fn render_document(&mut self, document: &CADDocument, view: &ViewState);
    fn render_entity(&mut self, entity: &CADEntity, view: &ViewState);
    fn render_preview(&mut self, preview: &RenderPreview, view: &ViewState);
}
