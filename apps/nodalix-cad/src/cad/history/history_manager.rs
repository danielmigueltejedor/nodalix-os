use super::actions::HistoryAction;

pub struct HistoryManager {
    undo_stack: Vec<Box<dyn HistoryAction>>,
    redo_stack: Vec<Box<dyn HistoryAction>>,
}

impl Default for HistoryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HistoryManager {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn execute(
        &mut self,
        action: Box<dyn HistoryAction>,
        document: &mut crate::cad::document::CADDocument,
    ) {
        action.apply(document);
        self.redo_stack.clear();
        self.undo_stack.push(action);
    }

    pub fn undo(&mut self, document: &mut crate::cad::document::CADDocument) -> bool {
        let Some(action) = self.undo_stack.pop() else {
            return false;
        };
        action.undo(document);
        self.redo_stack.push(action);
        true
    }

    pub fn redo(&mut self, document: &mut crate::cad::document::CADDocument) -> bool {
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
    use crate::cad::document::CADDocument;
    use crate::cad::entities::{BaseEntity, CADEntity, LineEntity};
    use crate::cad::geometry::Point2;
    use crate::cad::history::AddEntityAction;

    #[test]
    fn add_entity_undo_redo() {
        let mut doc = CADDocument::new_empty();
        let mut history = HistoryManager::new();
        let entity = CADEntity::Line(LineEntity::new(
            BaseEntity::new("entity-99", "layer-default"),
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
        ));
        history.execute(Box::new(AddEntityAction { entity }), &mut doc);
        assert_eq!(doc.model_space.entities.len(), 1);
        assert!(history.undo(&mut doc));
        assert!(doc.model_space.entities.is_empty());
        assert!(history.redo(&mut doc));
        assert_eq!(doc.model_space.entities.len(), 1);
    }

    #[test]
    fn move_entity_undo_redo() {
        use crate::cad::history::MoveEntityAction;

        let mut doc = CADDocument::new_empty();
        let mut history = HistoryManager::new();
        let entity = CADEntity::Line(LineEntity::new(
            BaseEntity::new("entity-1", "layer-default"),
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
        ));
        history.execute(Box::new(AddEntityAction { entity }), &mut doc);
        history.execute(
            Box::new(MoveEntityAction {
                entity_id: "entity-1".to_string(),
                dx: 2.0,
                dy: 3.0,
            }),
            &mut doc,
        );
        let line = match &doc.model_space.entities[0] {
            CADEntity::Line(line) => line,
            _ => panic!("expected line"),
        };
        assert!((line.start.x - 2.0).abs() < f64::EPSILON);
        assert!(history.undo(&mut doc));
        let line = match &doc.model_space.entities[0] {
            CADEntity::Line(line) => line,
            _ => panic!("expected line"),
        };
        assert!((line.start.x).abs() < f64::EPSILON);
    }
}
