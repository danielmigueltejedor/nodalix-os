use super::legacy_actions::LegacyHistoryAction;
use crate::document::Document;

/// Undo/redo stack for legacy `Document` mutations (UI integration).
pub struct LegacyHistoryManager {
    undo_stack: Vec<Box<dyn LegacyHistoryAction>>,
    redo_stack: Vec<Box<dyn LegacyHistoryAction>>,
}

impl Default for LegacyHistoryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LegacyHistoryManager {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn execute(&mut self, action: Box<dyn LegacyHistoryAction>, document: &mut Document) {
        action.apply(document);
        self.redo_stack.clear();
        self.undo_stack.push(action);
    }

    /// Record an action whose `apply` effects are already present in `document` (e.g. paste).
    pub fn record(&mut self, action: Box<dyn LegacyHistoryAction>) {
        self.redo_stack.clear();
        self.undo_stack.push(action);
    }

    pub fn undo(&mut self, document: &mut Document) -> bool {
        let Some(action) = self.undo_stack.pop() else {
            return false;
        };
        action.undo(document);
        self.redo_stack.push(action);
        true
    }

    pub fn redo(&mut self, document: &mut Document) -> bool {
        let Some(action) = self.redo_stack.pop() else {
            return false;
        };
        action.apply(document);
        self.undo_stack.push(action);
        true
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_undo_redo_is_safe() {
        let mut document = Document::new_empty();
        let mut history = LegacyHistoryManager::new();
        assert!(!history.undo(&mut document));
        assert!(!history.redo(&mut document));
    }
}
