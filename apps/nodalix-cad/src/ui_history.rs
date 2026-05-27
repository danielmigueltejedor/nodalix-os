//! UI helpers for legacy document undo/redo (`LegacyHistoryManager`).
//!
// TODO(phase-3): retire `undo_stack` snapshot fallback once all edits use reversible actions.

use crate::{
    cad::history::{
        capture_entity_property_state, capture_entity_snapshots, entity_snapshot_for_add,
        record_entity_property_changes, EntityPropertyChange, LegacyAddEntitiesAction,
        LegacyPasteEntitiesAction, LegacyRemoveEntitiesAction,
    },
    canvas::{update_selection_label, CadCanvas},
    document::Entity,
    geometry::Point,
    ui_context::UiCadContext,
};
use gtk::prelude::*;
use std::{cell::RefCell, rc::Rc};

/// Document `RefCell` is held elsewhere (GTK draw / nested handler).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DocumentBusy;

pub(crate) fn document_active_layout(cad: &UiCadContext) -> String {
    cad.document
        .try_borrow()
        .map(|document| document.active_layout.clone())
        .unwrap_or_else(|_| "Model".to_string())
}

pub(crate) fn clear_document_history(cad: &UiCadContext) {
    cad.history.borrow_mut().clear();
    cad.undo_stack.borrow_mut().clear();
}

pub(crate) fn try_add_entity_with_history(
    cad: &UiCadContext,
    entity: Entity,
    active_layout: &str,
) -> Result<u64, DocumentBusy> {
    let id = entity.id();
    let snapshot = entity_snapshot_for_add(entity.clone(), active_layout);
    {
        let mut document = cad.document.try_borrow_mut().map_err(|_| DocumentBusy)?;
        document.add_entity(entity);
    }
    cad.history
        .borrow_mut()
        .record(Box::new(LegacyAddEntitiesAction {
            added: vec![snapshot],
        }));
    Ok(id)
}

pub(crate) fn try_add_entities_with_history(
    cad: &UiCadContext,
    entities: Vec<Entity>,
    active_layout: &str,
) -> Result<Vec<u64>, DocumentBusy> {
    let snapshots: Vec<_> = entities
        .iter()
        .map(|entity| entity_snapshot_for_add(entity.clone(), active_layout))
        .collect();
    let ids: Vec<u64> = snapshots
        .iter()
        .map(|snapshot| snapshot.entity.id())
        .collect();
    {
        let mut document = cad.document.try_borrow_mut().map_err(|_| DocumentBusy)?;
        for entity in entities {
            document.add_entity(entity);
        }
    }
    if !snapshots.is_empty() {
        cad.history
            .borrow_mut()
            .record(Box::new(LegacyAddEntitiesAction { added: snapshots }));
    }
    Ok(ids)
}

pub(crate) fn duplicate_entity_with_history(cad: &UiCadContext, id: u64) -> Option<u64> {
    let copy_id = {
        let mut document = cad.document.try_borrow_mut().ok()?;
        document.duplicate_entity(id)?
    };
    let document = cad.document.try_borrow().ok()?;
    let added = capture_entity_snapshots(&document, &[copy_id]);
    if !added.is_empty() {
        cad.history
            .borrow_mut()
            .record(Box::new(LegacyAddEntitiesAction { added }));
    }
    Some(copy_id)
}

pub(crate) fn delete_selected_entities(cad: &UiCadContext, ids: &[u64]) -> bool {
    if ids.is_empty() {
        return false;
    }
    let removed = {
        let document = cad.document.borrow();
        capture_entity_snapshots(&document, ids)
    };
    if removed.is_empty() {
        return false;
    }
    cad.history.borrow_mut().execute(
        Box::new(LegacyRemoveEntitiesAction { removed }),
        &mut cad.document.borrow_mut(),
    );
    true
}

pub(crate) fn paste_entities_at(
    cad: &UiCadContext,
    entities: &[Entity],
    target: Point,
) -> Vec<u64> {
    if entities.is_empty() {
        return Vec::new();
    }
    let ids = cad
        .document
        .borrow_mut()
        .paste_entities_at(entities, target);
    if ids.is_empty() {
        return ids;
    }
    let action = LegacyPasteEntitiesAction::after_paste(&cad.document.borrow(), &ids);
    cad.history.borrow_mut().record(Box::new(action));
    ids
}

pub(crate) fn update_entity_properties_with_history<F>(
    cad: &UiCadContext,
    ids: &[u64],
    mut updater: F,
) -> bool
where
    F: FnMut(&mut crate::document::Document, u64) -> bool,
{
    let before: Vec<(u64, crate::cad::history::EntityPropertyState)> = {
        let document = cad.document.borrow();
        ids.iter()
            .filter_map(|id| {
                capture_entity_property_state(&document, *id).map(|state| (*id, state))
            })
            .collect()
    };
    if before.is_empty() {
        return false;
    }

    let mut any_changed = false;
    {
        let mut document = cad.document.borrow_mut();
        for (id, _) in &before {
            any_changed |= updater(&mut document, *id);
        }
    }
    if !any_changed {
        return false;
    }

    let changes: Vec<EntityPropertyChange> = before
        .into_iter()
        .filter_map(|(id, before_state)| {
            let after = capture_entity_property_state(&cad.document.borrow(), id)?;
            if before_state == after {
                None
            } else {
                Some(EntityPropertyChange {
                    entity_id: id,
                    before: before_state,
                    after,
                })
            }
        })
        .collect();

    if !changes.is_empty() {
        record_entity_property_changes(&mut cad.history.borrow_mut(), changes);
        true
    } else {
        false
    }
}

pub(crate) fn perform_undo(cad: &UiCadContext) -> bool {
    if cad
        .history
        .borrow_mut()
        .undo(&mut cad.document.borrow_mut())
    {
        return true;
    }
    let Some(previous) = cad.undo_stack.borrow_mut().pop() else {
        return false;
    };
    *cad.document.borrow_mut() = previous;
    true
}

pub(crate) fn perform_redo(cad: &UiCadContext) -> bool {
    cad.history
        .borrow_mut()
        .redo(&mut cad.document.borrow_mut())
}

pub(crate) fn refresh_after_history_change(
    cad: &UiCadContext,
    canvas: &CadCanvas,
    selection_label: &gtk::Label,
    properties: &gtk::Box,
    modified_label: &gtk::Label,
    selected_entity: &Rc<RefCell<Vec<u64>>>,
) {
    selected_entity.borrow_mut().retain(|id| {
        cad.document
            .borrow()
            .entities
            .iter()
            .any(|entity| entity.id() == *id)
    });
    if let Ok(document) = cad.document.try_borrow() {
        let selected = selected_entity.borrow();
        update_selection_label(selection_label, &document, &selected);
        crate::ui::refresh_properties(properties, &document);
    }
    modified_label.set_text("Modified");
    canvas.widget().queue_draw();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;
    use crate::geometry::Point;

    fn test_cad() -> UiCadContext {
        UiCadContext::new(Rc::new(RefCell::new(Document::new_empty())))
    }

    #[test]
    fn try_add_entity_command_style_build() {
        let cad = test_cad();
        let entity = {
            let id = cad.document.borrow().next_id();
            Entity::Line {
                id,
                layer: "Default".to_string(),
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 100.0, y: 100.0 },
            }
        };
        let id = try_add_entity_with_history(&cad, entity, "Model").expect("add");
        assert_eq!(id, 1);
        assert_eq!(cad.document.borrow().entities.len(), 1);
        assert!(cad.history.borrow().can_undo());
    }

    #[test]
    fn try_add_returns_busy_when_document_mut_borrowed() {
        let cad = test_cad();
        let entity = Entity::Line {
            id: 1,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 1.0, y: 0.0 },
        };
        let _guard = cad.document.borrow_mut();
        assert_eq!(
            try_add_entity_with_history(&cad, entity, "Model"),
            Err(DocumentBusy)
        );
    }
}
