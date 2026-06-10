use crate::cad::document::CADDocument;
use crate::cad::entities::CADEntity;
use crate::cad::geometry::Point2;

pub trait HistoryAction {
    fn description(&self) -> &'static str;
    fn apply(&self, document: &mut CADDocument);
    fn undo(&self, document: &mut CADDocument);
}

pub struct AddEntityAction {
    pub entity: CADEntity,
}

impl HistoryAction for AddEntityAction {
    fn description(&self) -> &'static str {
        "Add entity"
    }

    fn apply(&self, document: &mut CADDocument) {
        document.add_entity(self.entity.clone());
    }

    fn undo(&self, document: &mut CADDocument) {
        document.remove_entity(self.entity.id());
    }
}

pub struct RemoveEntityAction {
    pub entity: CADEntity,
    pub layout_id: String,
}

impl HistoryAction for RemoveEntityAction {
    fn description(&self) -> &'static str {
        "Remove entity"
    }

    fn apply(&self, document: &mut CADDocument) {
        document.remove_entity(self.entity.id());
    }

    fn undo(&self, document: &mut CADDocument) {
        let previous_active = document.active_layout_id.clone();
        document.active_layout_id = self.layout_id.clone();
        document.add_entity(self.entity.clone());
        document.active_layout_id = previous_active;
    }
}

pub struct MoveEntityAction {
    pub entity_id: String,
    pub dx: f64,
    pub dy: f64,
}

impl HistoryAction for MoveEntityAction {
    fn description(&self) -> &'static str {
        "Move entity"
    }

    fn apply(&self, document: &mut CADDocument) {
        translate_entity(document, &self.entity_id, self.dx, self.dy);
    }

    fn undo(&self, document: &mut CADDocument) {
        translate_entity(document, &self.entity_id, -self.dx, -self.dy);
    }
}

fn translate_entity(document: &mut CADDocument, id: &str, dx: f64, dy: f64) {
    let space = document.entities_in_active_space_mut();
    if let Some(entity) = space.iter_mut().find(|e| e.id() == id) {
        entity.translate(dx, dy);
        document.modified = true;
    }
}

pub fn point_delta(from: Point2, to: Point2) -> (f64, f64) {
    (to.x - from.x, to.y - from.y)
}
