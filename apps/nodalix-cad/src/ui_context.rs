use crate::{
    cad::history::LegacyHistoryManager,
    canvas::CadCanvas,
    document::{Document, Entity},
    tool_parameters::ToolParametersState,
};
use std::{cell::RefCell, path::PathBuf, rc::Rc};

#[derive(Clone)]
pub(crate) struct DocumentTab {
    pub title: String,
    pub path: Option<PathBuf>,
    pub document: Document,
}

/// Core CAD state shared across toolbar, canvas handlers, and keyboard shortcuts.
#[derive(Clone)]
pub(crate) struct UiCadContext {
    pub document: Rc<RefCell<Document>>,
    pub current_path: Rc<RefCell<Option<PathBuf>>>,
    pub selected_entity: Rc<RefCell<Vec<u64>>>,
    pub clipboard: Rc<RefCell<Vec<Entity>>>,
    /// Reversible edits on the legacy document (delete, paste, move, …).
    pub history: Rc<RefCell<LegacyHistoryManager>>,
    pub tool_parameters: Rc<RefCell<ToolParametersState>>,
    /// Full-document snapshot fallback for operations not yet on `history`.
    pub undo_stack: Rc<RefCell<Vec<Document>>>,
}

impl UiCadContext {
    pub(crate) fn new(document: Rc<RefCell<Document>>) -> Self {
        Self {
            document,
            current_path: Rc::new(RefCell::new(None)),
            selected_entity: Rc::new(RefCell::new(Vec::new())),
            clipboard: Rc::new(RefCell::new(Vec::new())),
            history: Rc::new(RefCell::new(LegacyHistoryManager::new())),
            tool_parameters: Rc::new(RefCell::new(ToolParametersState::default())),
            undo_stack: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

/// GTK widgets refreshed after document or layout mutations.
#[derive(Clone)]
pub(crate) struct UiViewContext {
    pub canvas: CadCanvas,
    pub properties: gtk::Box,
    pub layout_tabs: gtk::Box,
    pub modified_label: gtk::Label,
}

/// Multi-document tab bar state.
#[derive(Clone)]
pub(crate) struct UiDocumentTabsContext {
    pub tabs: Rc<RefCell<Vec<DocumentTab>>>,
    pub active_tab: Rc<RefCell<usize>>,
    pub bar: gtk::Box,
}
