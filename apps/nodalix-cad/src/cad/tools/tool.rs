use crate::cad::document::CADDocument;
use crate::cad::geometry::Point2;
use crate::cad::selection::SelectionManager;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolPhase {
    #[default]
    Idle,
    AwaitingFirstPoint,
    AwaitingSecondPoint,
    Dragging,
}

#[derive(Debug)]
pub struct ToolContext<'a> {
    pub document: &'a mut CADDocument,
    pub selection: &'a mut SelectionManager,
    pub phase: ToolPhase,
    pub pending_point: Option<Point2>,
    pub preview_point: Option<Point2>,
}

#[derive(Clone, Copy, Debug)]
pub struct CADPointerEvent {
    pub world: Point2,
    pub screen_x: f64,
    pub screen_y: f64,
    pub button: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct CADKeyEvent {
    pub key: &'static str,
    pub shift: bool,
    pub ctrl: bool,
}

pub trait CADTool: Send {
    fn id(&self) -> &'static str;
    fn label(&self) -> &'static str;

    fn activate(&mut self, _ctx: &mut ToolContext<'_>) {}
    fn deactivate(&mut self, _ctx: &mut ToolContext<'_>) {}

    fn pointer_down(&mut self, _event: CADPointerEvent, _ctx: &mut ToolContext<'_>) {}
    fn pointer_move(&mut self, _event: CADPointerEvent, _ctx: &mut ToolContext<'_>) {}
    fn pointer_up(&mut self, _event: CADPointerEvent, _ctx: &mut ToolContext<'_>) {}
    fn key_down(&mut self, _event: CADKeyEvent, _ctx: &mut ToolContext<'_>) {}
}
