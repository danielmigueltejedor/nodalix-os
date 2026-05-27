pub mod adapter;
pub mod cad_document;
pub mod serialization;

pub use adapter::{from_legacy_document, to_legacy_document};
pub use cad_document::{
    BlockDefinition, CADDocument, CADSettings, DrawingUnit, Layer, Layout, LayoutKind, ModelSpace,
    PageSetup, PaperSetup,
};
pub use serialization::{load_cad_document, save_cad_document, CAD_DOCUMENT_FORMAT_VERSION};
