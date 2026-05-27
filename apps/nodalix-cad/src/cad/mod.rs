//! CAD core: document model, geometry, tools, and editing — independent of GTK.
//!
//! `ui.rs` and `canvas.rs` remain the integration layer until migration completes.

pub mod commands;
pub mod dimensions;
pub mod document;
pub mod entities;
pub mod geometry;
pub mod history;
pub mod layers;
pub mod layouts;
pub mod precision;
pub mod rendering;
pub mod selection;
pub mod snapping;
pub mod tools;

pub use commands::command_registry::CommandRegistry;
pub use document::adapter;
pub use document::cad_document::{
    BlockDefinition, CADDocument, CADSettings, DrawingUnit, Layout, LayoutKind, ModelSpace,
    PageSetup, PaperSetup,
};
pub use document::serialization;
pub use entities::entity::{BaseEntity, CADEntity};
pub use entities::{CircleEntity, LineEntity, PolylineEntity};
pub use geometry::{BoundingBox2, Point2, Transform2, Vector2};
pub use history::history_manager::HistoryManager;
pub use layers::layer_manager::LayerManager;
pub use selection::hit_testing;
pub use selection::selection_manager::SelectionManager;
pub use tools::tool_manager::ToolManager;
