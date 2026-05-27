//! Reversible actions on the legacy `Document` (full entity data, no CADDocument roundtrip).
//!
// TODO(phase-3): migrate to `HistoryAction` on `CADDocument` when adapter covers all entity kinds.

use crate::document::{Document, Entity, Layer};

/// Snapshot of a legacy entity plus per-entity metadata removed with it.
#[derive(Clone, Debug)]
pub struct EntitySnapshot {
    pub entity: Entity,
    pub color: Option<String>,
    pub line_weight: Option<f64>,
    pub line_type: Option<String>,
    pub layout: Option<String>,
}

pub trait LegacyHistoryAction: Send {
    fn description(&self) -> &'static str;
    fn apply(&self, document: &mut Document);
    fn undo(&self, document: &mut Document);
}

/// Snapshot for a not-yet-added entity (command bar / tools). Avoids reading the document
/// after `borrow_mut`, which can panic during GTK reentrancy.
pub fn entity_snapshot_for_add(entity: Entity, active_layout: &str) -> EntitySnapshot {
    let layout = {
        let name = active_layout.trim();
        (name != "Model" && !name.is_empty()).then(|| name.to_string())
    };
    EntitySnapshot {
        entity,
        color: None,
        line_weight: None,
        line_type: None,
        layout,
    }
}

pub fn capture_entity_snapshot(document: &Document, id: u64) -> Option<EntitySnapshot> {
    let entity = document
        .entities
        .iter()
        .find(|entity| entity.id() == id)?
        .clone();
    let color = document.entity_colors.get(&id).cloned();
    let line_weight = document.entity_line_weights.get(&id).copied();
    let line_type = document.entity_line_types.get(&id).cloned();
    let layout = document.entity_layouts.get(&id).cloned();
    Some(EntitySnapshot {
        entity,
        color,
        line_weight,
        line_type,
        layout,
    })
}

pub fn capture_entity_snapshots(document: &Document, ids: &[u64]) -> Vec<EntitySnapshot> {
    ids.iter()
        .filter_map(|id| capture_entity_snapshot(document, *id))
        .collect()
}

pub fn restore_entity_snapshot(document: &mut Document, snapshot: &EntitySnapshot) {
    let id = snapshot.entity.id();
    if document.entities.iter().any(|entity| entity.id() == id) {
        apply_entity_snapshot_in_place(document, snapshot);
        return;
    }
    document.entity_colors.remove(&id);
    document.entity_line_weights.remove(&id);
    document.entity_line_types.remove(&id);
    document.entity_layouts.remove(&id);
    if let Some(color) = &snapshot.color {
        document.entity_colors.insert(id, color.clone());
    }
    if let Some(weight) = snapshot.line_weight {
        document.entity_line_weights.insert(id, weight);
    }
    if let Some(line_type) = &snapshot.line_type {
        document.entity_line_types.insert(id, line_type.clone());
    }
    let layout = snapshot.layout.as_deref();
    document.add_entity_on_layout(snapshot.entity.clone(), layout);
}

/// Replace an existing entity (rotate/scale/mirror undo/redo).
pub fn apply_entity_snapshot_in_place(document: &mut Document, snapshot: &EntitySnapshot) {
    let id = snapshot.entity.id();
    let Some(entity) = document
        .entities
        .iter_mut()
        .find(|entity| entity.id() == id)
    else {
        restore_entity_snapshot(document, snapshot);
        return;
    };
    *entity = snapshot.entity.clone();
    document.invalidate_entity_bounds(id);
    document.entity_colors.remove(&id);
    document.entity_line_weights.remove(&id);
    document.entity_line_types.remove(&id);
    document.entity_layouts.remove(&id);
    if let Some(color) = &snapshot.color {
        document.entity_colors.insert(id, color.clone());
    }
    if let Some(weight) = snapshot.line_weight {
        document.entity_line_weights.insert(id, weight);
    }
    if let Some(line_type) = &snapshot.line_type {
        document.entity_line_types.insert(id, line_type.clone());
    }
    if let Some(layout) = &snapshot.layout {
        document.entity_layouts.insert(id, layout.clone());
    }
    document.modified = true;
}

/// Removes entities; undo restores full snapshots (all legacy entity kinds).
pub struct LegacyRemoveEntitiesAction {
    pub removed: Vec<EntitySnapshot>,
}

impl LegacyHistoryAction for LegacyRemoveEntitiesAction {
    fn description(&self) -> &'static str {
        "Remove entities"
    }

    fn apply(&self, document: &mut Document) {
        for snapshot in &self.removed {
            document.remove_entity(snapshot.entity.id());
        }
    }

    fn undo(&self, document: &mut Document) {
        for snapshot in &self.removed {
            restore_entity_snapshot(document, snapshot);
        }
    }
}

/// Records entities added by paste; undo removes them, redo restores them with stable ids.
pub struct LegacyPasteEntitiesAction {
    pub pasted: Vec<EntitySnapshot>,
}

impl LegacyPasteEntitiesAction {
    pub fn after_paste(document: &Document, pasted_ids: &[u64]) -> Self {
        Self {
            pasted: capture_entity_snapshots(document, pasted_ids),
        }
    }
}

impl LegacyHistoryAction for LegacyPasteEntitiesAction {
    fn description(&self) -> &'static str {
        "Paste entities"
    }

    fn apply(&self, document: &mut Document) {
        for snapshot in &self.pasted {
            restore_entity_snapshot(document, snapshot);
        }
    }

    fn undo(&self, document: &mut Document) {
        for snapshot in &self.pasted {
            document.remove_entity(snapshot.entity.id());
        }
    }
}

/// Adds entities with explicit ids (used after create/paste/duplicate is recorded).
pub struct LegacyAddEntitiesAction {
    pub added: Vec<EntitySnapshot>,
}

impl LegacyAddEntitiesAction {
    pub fn after_add(document: &Document, added_ids: &[u64]) -> Self {
        Self {
            added: capture_entity_snapshots(document, added_ids),
        }
    }
}

/// Records entities already present in the document (stable ids for undo/redo).
pub fn record_entities_added(
    history: &mut crate::cad::history::legacy_history_manager::LegacyHistoryManager,
    document: &Document,
    added_ids: &[u64],
) {
    let added = capture_entity_snapshots(document, added_ids);
    if added.is_empty() {
        return;
    }
    history.record(Box::new(LegacyAddEntitiesAction { added }));
}

impl LegacyHistoryAction for LegacyAddEntitiesAction {
    fn description(&self) -> &'static str {
        "Add entities"
    }

    fn apply(&self, document: &mut Document) {
        for snapshot in &self.added {
            restore_entity_snapshot(document, snapshot);
        }
    }

    fn undo(&self, document: &mut Document) {
        for snapshot in &self.added {
            document.remove_entity(snapshot.entity.id());
        }
    }
}

/// Translates entities by a delta; undo applies the inverse.
pub struct LegacyMoveEntitiesAction {
    pub entity_ids: Vec<u64>,
    pub dx: f64,
    pub dy: f64,
}

#[derive(Clone, Debug)]
pub struct LayerStateSnapshot {
    pub layers: Vec<Layer>,
    pub active_layer_name: String,
}

#[derive(Clone, Debug)]
pub struct LegacyUpdateLayersAction {
    pub before: LayerStateSnapshot,
    pub after: LayerStateSnapshot,
}

pub fn capture_layer_state(document: &Document) -> LayerStateSnapshot {
    LayerStateSnapshot {
        layers: document.layers.clone(),
        active_layer_name: document.active_layer_name.clone(),
    }
}

pub fn apply_layer_state(document: &mut Document, snapshot: &LayerStateSnapshot) {
    document.layers = snapshot.layers.clone();
    document.active_layer_name = snapshot.active_layer_name.clone();
    document.ensure_active_layer_exists();
    document.modified = true;
}

impl LegacyHistoryAction for LegacyUpdateLayersAction {
    fn description(&self) -> &'static str {
        "Update layers"
    }

    fn apply(&self, document: &mut Document) {
        apply_layer_state(document, &self.after);
    }

    fn undo(&self, document: &mut Document) {
        apply_layer_state(document, &self.before);
    }
}

const MOVE_DELTA_EPSILON: f64 = 1e-9;

/// Records a completed canvas move (entities already translated in the document).
pub fn record_entity_move_if_nonzero(
    history: &mut crate::cad::history::legacy_history_manager::LegacyHistoryManager,
    entity_ids: Vec<u64>,
    dx: f64,
    dy: f64,
) {
    if entity_ids.is_empty() || (dx.abs() < MOVE_DELTA_EPSILON && dy.abs() < MOVE_DELTA_EPSILON) {
        return;
    }
    history.record(Box::new(LegacyMoveEntitiesAction { entity_ids, dx, dy }));
}

impl LegacyHistoryAction for LegacyMoveEntitiesAction {
    fn description(&self) -> &'static str {
        "Move entities"
    }

    fn apply(&self, document: &mut Document) {
        for id in &self.entity_ids {
            document.translate_entity(*id, self.dx, self.dy);
        }
    }

    fn undo(&self, document: &mut Document) {
        for id in &self.entity_ids {
            document.translate_entity(*id, -self.dx, -self.dy);
        }
    }
}

/// Full before/after entity snapshots for rotate, scale, mirror.
pub struct LegacyTransformEntitiesAction {
    pub before: Vec<EntitySnapshot>,
    pub after: Vec<EntitySnapshot>,
}

impl LegacyHistoryAction for LegacyTransformEntitiesAction {
    fn description(&self) -> &'static str {
        "Transform entities"
    }

    fn apply(&self, document: &mut Document) {
        for snapshot in &self.after {
            apply_entity_snapshot_in_place(document, snapshot);
        }
    }

    fn undo(&self, document: &mut Document) {
        for snapshot in &self.before {
            apply_entity_snapshot_in_place(document, snapshot);
        }
    }
}

pub fn record_entity_transform<F>(
    history: &mut crate::cad::history::legacy_history_manager::LegacyHistoryManager,
    document: &mut Document,
    entity_ids: &[u64],
    mut transform: F,
) where
    F: FnMut(&mut Entity),
{
    if entity_ids.is_empty() {
        return;
    }
    let before = capture_entity_snapshots(document, entity_ids);
    for id in entity_ids {
        if document.entity_layer_locked(*id) {
            continue;
        }
        if let Some(entity) = document.entities.iter_mut().find(|e| e.id() == *id) {
            transform(entity);
            document.invalidate_entity_bounds(*id);
        }
    }
    document.modified = true;
    let after = capture_entity_snapshots(document, entity_ids);
    if before.is_empty() || after.is_empty() {
        return;
    }
    history.record(Box::new(LegacyTransformEntitiesAction { before, after }));
}

/// Reversible entity property edits (layer, color, line style, text content).
#[derive(Clone, Debug, PartialEq)]
pub struct EntityPropertyState {
    pub layer: String,
    pub color: Option<String>,
    pub line_weight: Option<f64>,
    pub line_type: Option<String>,
    pub text: Option<String>,
}

#[derive(Clone, Debug)]
pub struct EntityPropertyChange {
    pub entity_id: u64,
    pub before: EntityPropertyState,
    pub after: EntityPropertyState,
}

pub fn capture_entity_property_state(document: &Document, id: u64) -> Option<EntityPropertyState> {
    let entity = document.entities.iter().find(|entity| entity.id() == id)?;
    let text = match entity {
        Entity::Text { text, .. } => Some(text.clone()),
        _ => None,
    };
    Some(EntityPropertyState {
        layer: entity.layer().to_string(),
        color: document.entity_colors.get(&id).cloned(),
        line_weight: document.entity_line_weights.get(&id).copied(),
        line_type: document.entity_line_types.get(&id).cloned(),
        text,
    })
}

pub fn apply_entity_property_state(document: &mut Document, id: u64, state: &EntityPropertyState) {
    if !document.entities.iter().any(|entity| entity.id() == id) {
        return;
    }
    document.set_entity_layer(id, &state.layer);
    match &state.color {
        Some(color) => {
            document.entity_colors.insert(id, color.clone());
        }
        None => {
            document.entity_colors.remove(&id);
        }
    }
    match state.line_weight {
        Some(weight) => {
            document.entity_line_weights.insert(id, weight);
        }
        None => {
            document.entity_line_weights.remove(&id);
        }
    }
    match &state.line_type {
        Some(line_type) => {
            document.entity_line_types.insert(id, line_type.clone());
        }
        None => {
            document.entity_line_types.remove(&id);
        }
    }
    if let Some(text) = &state.text {
        document.set_text_entity_text(id, text);
    }
    document.modified = true;
}

pub struct LegacyUpdateEntityPropertiesAction {
    pub changes: Vec<EntityPropertyChange>,
}

impl LegacyHistoryAction for LegacyUpdateEntityPropertiesAction {
    fn description(&self) -> &'static str {
        "Update entity properties"
    }

    fn apply(&self, document: &mut Document) {
        for change in &self.changes {
            apply_entity_property_state(document, change.entity_id, &change.after);
        }
    }

    fn undo(&self, document: &mut Document) {
        for change in &self.changes {
            apply_entity_property_state(document, change.entity_id, &change.before);
        }
    }
}

pub fn record_entity_property_changes(
    history: &mut crate::cad::history::legacy_history_manager::LegacyHistoryManager,
    changes: Vec<EntityPropertyChange>,
) {
    if changes.is_empty() {
        return;
    }
    history.record(Box::new(LegacyUpdateEntityPropertiesAction { changes }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::history::legacy_history_manager::LegacyHistoryManager;
    use crate::geometry::Point;

    #[test]
    fn remove_entities_undo_redo_preserves_data() {
        let mut document = Document::new_empty();
        document.add_entity(Entity::Line {
            id: 1,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 5.0, y: 0.0 },
        });
        document.entity_colors.insert(1, "#ff0000".to_string());

        let removed = capture_entity_snapshots(&document, &[1]);
        let mut history = LegacyHistoryManager::new();
        history.execute(
            Box::new(LegacyRemoveEntitiesAction { removed }),
            &mut document,
        );
        assert!(document.entities.is_empty());

        assert!(history.undo(&mut document));
        assert_eq!(document.entities.len(), 1);
        assert_eq!(
            document.entity_colors.get(&1).map(String::as_str),
            Some("#ff0000")
        );

        assert!(history.redo(&mut document));
        assert!(document.entities.is_empty());
    }

    #[test]
    fn paste_entities_undo_redo() {
        let mut document = Document::new_empty();
        let source = Entity::Text {
            id: 99,
            layer: "Default".to_string(),
            origin: Point { x: 1.0, y: 2.0 },
            text: "Hi".to_string(),
            height: 2.5,
            rotation: 0.0,
        };
        let pasted_ids = document.paste_entities_at(&[source], Point { x: 0.0, y: 0.0 });
        assert_eq!(pasted_ids.len(), 1);

        let action = LegacyPasteEntitiesAction::after_paste(&document, &pasted_ids);
        let mut history = LegacyHistoryManager::new();
        history.record(Box::new(action));
        assert_eq!(document.entities.len(), 1);

        assert!(history.undo(&mut document));
        assert!(document.entities.is_empty());

        assert!(history.redo(&mut document));
        assert_eq!(document.entities.len(), 1);
        match &document.entities[0] {
            Entity::Text { text, .. } => assert_eq!(text, "Hi"),
            _ => panic!("expected text"),
        }
    }

    #[test]
    fn move_entities_undo_redo() {
        let mut document = Document::new_empty();
        document.add_entity(Entity::Line {
            id: 1,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 1.0, y: 0.0 },
        });
        let mut history = LegacyHistoryManager::new();
        history.execute(
            Box::new(LegacyMoveEntitiesAction {
                entity_ids: vec![1],
                dx: 3.0,
                dy: 4.0,
            }),
            &mut document,
        );
        let line = &document.entities[0];
        let Entity::Line { start, .. } = line else {
            panic!("expected line");
        };
        assert!((start.x - 3.0).abs() < f64::EPSILON);

        assert!(history.undo(&mut document));
        let Entity::Line { start, .. } = &document.entities[0] else {
            panic!("expected line");
        };
        assert!(start.x.abs() < f64::EPSILON);

        assert!(history.redo(&mut document));
        let Entity::Line { start, .. } = &document.entities[0] else {
            panic!("expected line");
        };
        assert!((start.x - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn record_move_skips_zero_delta() {
        let mut history = LegacyHistoryManager::new();
        record_entity_move_if_nonzero(&mut history, vec![1], 0.0, 0.0);
        assert!(!history.can_undo());
    }

    #[test]
    fn record_move_negative_delta_undo_redo() {
        let mut document = Document::new_empty();
        document.add_entity(Entity::Line {
            id: 1,
            layer: "Default".to_string(),
            start: Point { x: 10.0, y: 10.0 },
            end: Point { x: 20.0, y: 10.0 },
        });
        document.translate_entity(1, 5.0, 0.0);
        let mut history = LegacyHistoryManager::new();
        record_entity_move_if_nonzero(&mut history, vec![1], 5.0, 0.0);
        assert!(history.undo(&mut document));
        let Entity::Line { start, .. } = &document.entities[0] else {
            panic!("expected line");
        };
        assert!((start.x - 10.0).abs() < f64::EPSILON);
        assert!(history.redo(&mut document));
        let Entity::Line { start, .. } = &document.entities[0] else {
            panic!("expected line");
        };
        assert!((start.x - 15.0).abs() < f64::EPSILON);
    }

    #[test]
    fn paste_move_delete_undo_redo_sequence() {
        let mut document = Document::new_empty();
        let mut history = LegacyHistoryManager::new();

        let source = Entity::Line {
            id: 99,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 4.0, y: 0.0 },
        };
        let pasted_ids = document.paste_entities_at(&[source], Point { x: 0.0, y: 0.0 });
        let pasted_id = pasted_ids[0];
        let Entity::Line {
            start: after_paste, ..
        } = &document.entities[0]
        else {
            panic!("expected line");
        };
        let at_paste = *after_paste;
        history.record(Box::new(LegacyPasteEntitiesAction::after_paste(
            &document,
            &pasted_ids,
        )));

        document.translate_entity(pasted_id, 2.0, 1.0);
        record_entity_move_if_nonzero(&mut history, vec![pasted_id], 2.0, 1.0);
        let Entity::Line {
            start: after_move, ..
        } = &document.entities[0]
        else {
            panic!("expected line");
        };
        let at_move = *after_move;

        let removed = capture_entity_snapshots(&document, &[pasted_id]);
        history.execute(
            Box::new(LegacyRemoveEntitiesAction { removed }),
            &mut document,
        );
        assert!(document.entities.is_empty());

        assert!(history.undo(&mut document));
        assert_eq!(document.entities.len(), 1);
        let Entity::Line { start, .. } = &document.entities[0] else {
            panic!("expected line");
        };
        assert!(
            (start.x - at_move.x).abs() < f64::EPSILON
                && (start.y - at_move.y).abs() < f64::EPSILON
        );

        assert!(history.undo(&mut document));
        let Entity::Line { start, .. } = &document.entities[0] else {
            panic!("expected line");
        };
        assert!(
            (start.x - at_paste.x).abs() < f64::EPSILON
                && (start.y - at_paste.y).abs() < f64::EPSILON
        );

        assert!(history.undo(&mut document));
        assert!(document.entities.is_empty());

        assert!(history.redo(&mut document));
        assert_eq!(document.entities.len(), 1);
        let Entity::Line { start, .. } = &document.entities[0] else {
            panic!("expected line");
        };
        assert!(
            (start.x - at_paste.x).abs() < f64::EPSILON
                && (start.y - at_paste.y).abs() < f64::EPSILON
        );

        assert!(history.redo(&mut document));
        let Entity::Line { start, .. } = &document.entities[0] else {
            panic!("expected line");
        };
        assert!(
            (start.x - at_move.x).abs() < f64::EPSILON
                && (start.y - at_move.y).abs() < f64::EPSILON
        );

        assert!(history.redo(&mut document));
        assert!(document.entities.is_empty());
    }

    #[test]
    fn entity_snapshot_for_add_matches_post_add_on_model() {
        let mut document = Document::new_empty();
        let entity = Entity::Line {
            id: 1,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 5.0, y: 0.0 },
        };
        let pre = entity_snapshot_for_add(entity.clone(), &document.active_layout);
        document.add_entity(entity);
        let post = capture_entity_snapshot(&document, 1).expect("entity");
        assert_eq!(pre.entity.id(), post.entity.id());
        assert_eq!(pre.layout, post.layout);
        assert_eq!(pre.color, post.color);
    }

    #[test]
    fn add_entity_undo_redo_preserves_data() {
        let mut document = Document::new_empty();
        let line = Entity::Line {
            id: 7,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        };
        document.add_entity(line);
        document.entity_colors.insert(7, "#00ff00".to_string());

        let mut history = LegacyHistoryManager::new();
        record_entities_added(&mut history, &document, &[7]);
        assert_eq!(document.entities.len(), 1);

        assert!(history.undo(&mut document));
        assert!(document.entities.is_empty());
        assert!(!document.entity_colors.contains_key(&7));

        assert!(history.redo(&mut document));
        assert_eq!(document.entities.len(), 1);
        assert_eq!(document.entities[0].id(), 7);
        assert_eq!(
            document.entity_colors.get(&7).map(String::as_str),
            Some("#00ff00")
        );
        let Entity::Line { end, .. } = &document.entities[0] else {
            panic!("expected line");
        };
        assert!((end.x - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn add_multiple_entities_undo_redo() {
        let mut document = Document::new_empty();
        document.add_entity(Entity::Line {
            id: 1,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 1.0, y: 0.0 },
        });
        document.add_entity(Entity::Circle {
            id: 2,
            layer: "Default".to_string(),
            center: Point { x: 5.0, y: 5.0 },
            radius: 3.0,
        });

        let mut history = LegacyHistoryManager::new();
        record_entities_added(&mut history, &document, &[1, 2]);
        assert_eq!(document.entities.len(), 2);

        assert!(history.undo(&mut document));
        assert!(document.entities.is_empty());

        assert!(history.redo(&mut document));
        assert_eq!(document.entities.len(), 2);
        let ids: Vec<u64> = document.entities.iter().map(|e| e.id()).collect();
        assert_eq!(ids, vec![1, 2]);
    }

    #[test]
    fn add_undo_redo_keeps_entity_ids() {
        let mut document = Document::new_empty();
        document.add_entity(Entity::Circle {
            id: 42,
            layer: "Default".to_string(),
            center: Point { x: 1.0, y: 2.0 },
            radius: 25.0,
        });

        let mut history = LegacyHistoryManager::new();
        record_entities_added(&mut history, &document, &[42]);

        assert!(history.undo(&mut document));
        assert!(history.redo(&mut document));
        assert_eq!(document.entities[0].id(), 42);
        let Entity::Circle { radius, .. } = &document.entities[0] else {
            panic!("expected circle");
        };
        assert!((*radius - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn add_move_delete_undo_redo_sequence() {
        let mut document = Document::new_empty();
        let mut history = LegacyHistoryManager::new();

        let line_id = 1u64;
        document.add_entity(Entity::Line {
            id: line_id,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        });
        record_entities_added(&mut history, &document, &[line_id]);
        let Entity::Line { start: at_add, .. } = &document.entities[0] else {
            panic!("expected line");
        };
        let at_add = *at_add;

        document.translate_entity(line_id, 3.0, 0.0);
        record_entity_move_if_nonzero(&mut history, vec![line_id], 3.0, 0.0);
        let Entity::Line { start: at_move, .. } = &document.entities[0] else {
            panic!("expected line");
        };
        let at_move = *at_move;

        let removed = capture_entity_snapshots(&document, &[line_id]);
        history.execute(
            Box::new(LegacyRemoveEntitiesAction { removed }),
            &mut document,
        );
        assert!(document.entities.is_empty());

        assert!(history.undo(&mut document));
        let Entity::Line { start, .. } = &document.entities[0] else {
            panic!("expected line");
        };
        assert!((start.x - at_move.x).abs() < f64::EPSILON);

        assert!(history.undo(&mut document));
        let Entity::Line { start, .. } = &document.entities[0] else {
            panic!("expected line");
        };
        assert!((start.x - at_add.x).abs() < f64::EPSILON);

        assert!(history.undo(&mut document));
        assert!(document.entities.is_empty());

        assert!(history.redo(&mut document));
        assert_eq!(document.entities[0].id(), line_id);
        let Entity::Line { start, .. } = &document.entities[0] else {
            panic!("expected line");
        };
        assert!((start.x - at_add.x).abs() < f64::EPSILON);

        assert!(history.redo(&mut document));
        let Entity::Line { start, .. } = &document.entities[0] else {
            panic!("expected line");
        };
        assert!((start.x - at_move.x).abs() < f64::EPSILON);

        assert!(history.redo(&mut document));
        assert!(document.entities.is_empty());
    }

    /// Simulates command-bar geometry create: entity added, then `record_entities_added`.
    #[test]
    fn geometry_command_style_add_records_history() {
        let mut document = Document::new_empty();
        let mut history = LegacyHistoryManager::new();

        let entity = Entity::Line {
            id: document.next_id(),
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 10.0 },
        };
        let id = entity.id();
        document.add_entity(entity);
        record_entities_added(&mut history, &document, &[id]);

        assert!(history.undo(&mut document));
        assert!(document.entities.is_empty());
        assert!(history.redo(&mut document));
        assert_eq!(document.entities[0].id(), id);
    }

    fn add_and_record(
        document: &mut Document,
        history: &mut LegacyHistoryManager,
        entity: Entity,
    ) -> u64 {
        let id = entity.id();
        document.add_entity(entity);
        record_entities_added(history, document, &[id]);
        id
    }

    #[test]
    fn text_add_undo_redo_preserves_data() {
        let mut document = Document::new_empty();
        let mut history = LegacyHistoryManager::new();
        let id = add_and_record(
            &mut document,
            &mut history,
            Entity::Text {
                id: 1,
                layer: "Default".to_string(),
                origin: Point { x: 2.0, y: 3.0 },
                text: "Hello".to_string(),
                height: 2.5,
                rotation: 15.0,
            },
        );
        assert_eq!(id, 1);

        assert!(history.undo(&mut document));
        assert!(document.entities.is_empty());

        assert!(history.redo(&mut document));
        let Entity::Text {
            text,
            origin,
            height,
            rotation,
            ..
        } = &document.entities[0]
        else {
            panic!("expected text");
        };
        assert_eq!(text, "Hello");
        assert!((origin.x - 2.0).abs() < f64::EPSILON);
        assert!((*height - 2.5).abs() < f64::EPSILON);
        assert!((*rotation - 15.0).abs() < f64::EPSILON);
        assert_eq!(document.entities[0].id(), 1);
    }

    #[test]
    fn table_add_undo_redo_preserves_data() {
        let mut document = Document::new_empty();
        let mut history = LegacyHistoryManager::new();
        add_and_record(
            &mut document,
            &mut history,
            Entity::Table {
                id: 5,
                layer: "Default".to_string(),
                origin: Point { x: 0.0, y: 0.0 },
                rows: 3,
                columns: 4,
                cell_width: 18.0,
                cell_height: 7.0,
            },
        );

        assert!(history.undo(&mut document));
        assert!(document.entities.is_empty());

        assert!(history.redo(&mut document));
        let Entity::Table {
            rows,
            columns,
            cell_width,
            cell_height,
            ..
        } = &document.entities[0]
        else {
            panic!("expected table");
        };
        assert_eq!(*rows, 3);
        assert_eq!(*columns, 4);
        assert!((*cell_width - 18.0).abs() < f64::EPSILON);
        assert!((*cell_height - 7.0).abs() < f64::EPSILON);
        assert_eq!(document.entities[0].id(), 5);
    }

    #[test]
    fn block_reference_add_undo_redo_preserves_data() {
        let mut document = Document::new_empty();
        let mut history = LegacyHistoryManager::new();
        add_and_record(
            &mut document,
            &mut history,
            Entity::BlockReference {
                id: 9,
                layer: "Default".to_string(),
                name: "MyBlock".to_string(),
                insertion: Point { x: 10.0, y: 20.0 },
                scale: 2.0,
                rotation: 45.0,
            },
        );

        assert!(history.undo(&mut document));
        assert!(document.entities.is_empty());

        assert!(history.redo(&mut document));
        let Entity::BlockReference {
            name,
            insertion,
            scale,
            rotation,
            ..
        } = &document.entities[0]
        else {
            panic!("expected block reference");
        };
        assert_eq!(name, "MyBlock");
        assert!((insertion.x - 10.0).abs() < f64::EPSILON);
        assert!((*scale - 2.0).abs() < f64::EPSILON);
        assert!((*rotation - 45.0).abs() < f64::EPSILON);
        assert_eq!(document.entities[0].id(), 9);
    }

    #[test]
    fn add_multiple_entities_undo_redo_keeps_all_ids() {
        let mut document = Document::new_empty();
        let mut history = LegacyHistoryManager::new();
        let entities = vec![
            Entity::Text {
                id: 1,
                layer: "Default".to_string(),
                origin: Point { x: 0.0, y: 0.0 },
                text: "A".to_string(),
                height: 2.0,
                rotation: 0.0,
            },
            Entity::Table {
                id: 2,
                layer: "Default".to_string(),
                origin: Point { x: 1.0, y: 1.0 },
                rows: 2,
                columns: 2,
                cell_width: 10.0,
                cell_height: 5.0,
            },
        ];
        let ids: Vec<u64> = entities.iter().map(|e| e.id()).collect();
        for entity in entities {
            document.add_entity(entity);
        }
        record_entities_added(&mut history, &document, &ids);

        assert!(history.undo(&mut document));
        assert!(document.entities.is_empty());
        assert!(history.redo(&mut document));
        assert_eq!(
            document.entities.iter().map(|e| e.id()).collect::<Vec<_>>(),
            vec![1, 2]
        );
    }

    #[test]
    fn text_add_delete_undo_redo_sequence() {
        let mut document = Document::new_empty();
        let mut history = LegacyHistoryManager::new();

        let text_id = add_and_record(
            &mut document,
            &mut history,
            Entity::Text {
                id: 1,
                layer: "Default".to_string(),
                origin: Point { x: 0.0, y: 0.0 },
                text: "Note".to_string(),
                height: 2.5,
                rotation: 0.0,
            },
        );

        let removed = capture_entity_snapshots(&document, &[text_id]);
        history.execute(
            Box::new(LegacyRemoveEntitiesAction { removed }),
            &mut document,
        );
        assert!(document.entities.is_empty());

        assert!(history.undo(&mut document));
        assert_eq!(document.entities.len(), 1);
        let Entity::Text { text, .. } = &document.entities[0] else {
            panic!("expected text");
        };
        assert_eq!(text, "Note");

        assert!(history.undo(&mut document));
        assert!(document.entities.is_empty());

        assert!(history.redo(&mut document));
        assert_eq!(document.entities[0].id(), text_id);

        assert!(history.redo(&mut document));
        assert!(document.entities.is_empty());
    }

    fn apply_property_change(
        document: &mut Document,
        history: &mut LegacyHistoryManager,
        id: u64,
        after: EntityPropertyState,
    ) {
        let before = capture_entity_property_state(document, id).expect("entity");
        apply_entity_property_state(document, id, &after);
        record_entity_property_changes(
            history,
            vec![EntityPropertyChange {
                entity_id: id,
                before,
                after,
            }],
        );
    }

    #[test]
    fn change_color_undo_redo() {
        let mut document = Document::new_empty();
        let mut history = LegacyHistoryManager::new();
        let id = add_and_record(
            &mut document,
            &mut history,
            Entity::Line {
                id: 1,
                layer: "Default".to_string(),
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 10.0, y: 0.0 },
            },
        );
        history.clear();

        let after = EntityPropertyState {
            color: Some("#ff0000".to_string()),
            ..capture_entity_property_state(&document, id).unwrap()
        };
        apply_property_change(&mut document, &mut history, id, after);
        assert_eq!(
            document.entity_colors.get(&id).map(String::as_str),
            Some("#ff0000")
        );

        assert!(history.undo(&mut document));
        assert!(document.entity_colors.get(&id).is_none());

        assert!(history.redo(&mut document));
        assert_eq!(
            document.entity_colors.get(&id).map(String::as_str),
            Some("#ff0000")
        );
    }

    #[test]
    fn change_layer_undo_redo() {
        let mut document = Document::new_empty();
        let mut history = LegacyHistoryManager::new();
        let id = add_and_record(
            &mut document,
            &mut history,
            Entity::Line {
                id: 2,
                layer: "Default".to_string(),
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 5.0, y: 0.0 },
            },
        );
        history.clear();

        let after = EntityPropertyState {
            layer: "Dimensions".to_string(),
            ..capture_entity_property_state(&document, id).unwrap()
        };
        apply_property_change(&mut document, &mut history, id, after);
        assert_eq!(document.entities[0].layer(), "Dimensions");

        assert!(history.undo(&mut document));
        assert_eq!(document.entities[0].layer(), "Default");

        assert!(history.redo(&mut document));
        assert_eq!(document.entities[0].layer(), "Dimensions");
    }

    #[test]
    fn change_line_weight_undo_redo() {
        let mut document = Document::new_empty();
        let mut history = LegacyHistoryManager::new();
        let id = add_and_record(
            &mut document,
            &mut history,
            Entity::Line {
                id: 3,
                layer: "Default".to_string(),
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 1.0, y: 0.0 },
            },
        );
        history.clear();

        let after = EntityPropertyState {
            line_weight: Some(1.5),
            ..capture_entity_property_state(&document, id).unwrap()
        };
        apply_property_change(&mut document, &mut history, id, after);
        assert!((document.entity_line_weight(id) - 1.5).abs() < f64::EPSILON);

        assert!(history.undo(&mut document));
        assert!((document.entity_line_weight(id) - 0.25).abs() < f64::EPSILON);

        assert!(history.redo(&mut document));
        assert!((document.entity_line_weight(id) - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn change_text_content_undo_redo() {
        let mut document = Document::new_empty();
        let mut history = LegacyHistoryManager::new();
        let id = add_and_record(
            &mut document,
            &mut history,
            Entity::Text {
                id: 4,
                layer: "Default".to_string(),
                origin: Point { x: 0.0, y: 0.0 },
                text: "Hello".to_string(),
                height: 2.5,
                rotation: 0.0,
            },
        );
        history.clear();

        let after = EntityPropertyState {
            text: Some("Updated".to_string()),
            ..capture_entity_property_state(&document, id).unwrap()
        };
        apply_property_change(&mut document, &mut history, id, after);
        let Entity::Text { text, .. } = &document.entities[0] else {
            panic!("expected text");
        };
        assert_eq!(text, "Updated");

        assert!(history.undo(&mut document));
        let Entity::Text { text, .. } = &document.entities[0] else {
            panic!("expected text");
        };
        assert_eq!(text, "Hello");

        assert!(history.redo(&mut document));
        let Entity::Text { text, .. } = &document.entities[0] else {
            panic!("expected text");
        };
        assert_eq!(text, "Updated");
    }

    #[test]
    fn add_color_move_delete_undo_redo_sequence() {
        let mut document = Document::new_empty();
        let mut history = LegacyHistoryManager::new();

        let line_id = add_and_record(
            &mut document,
            &mut history,
            Entity::Line {
                id: 1,
                layer: "Default".to_string(),
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 10.0, y: 0.0 },
            },
        );

        let color_after = EntityPropertyState {
            color: Some("#00ff00".to_string()),
            ..capture_entity_property_state(&document, line_id).unwrap()
        };
        apply_property_change(&mut document, &mut history, line_id, color_after);
        document.translate_entity(line_id, 5.0, 0.0);
        record_entity_move_if_nonzero(&mut history, vec![line_id], 5.0, 0.0);

        let removed = capture_entity_snapshots(&document, &[line_id]);
        history.execute(
            Box::new(LegacyRemoveEntitiesAction { removed }),
            &mut document,
        );
        assert!(document.entities.is_empty());

        assert!(history.undo(&mut document));
        assert_eq!(document.entities.len(), 1);
        assert!(history.undo(&mut document));
        let Entity::Line { start, .. } = &document.entities[0] else {
            panic!("expected line");
        };
        assert!(start.x.abs() < f64::EPSILON);
        assert!(history.undo(&mut document));
        assert!(document.entity_colors.get(&line_id).is_none());
        assert!(history.undo(&mut document));
        assert!(document.entities.is_empty());

        assert!(history.redo(&mut document));
        assert_eq!(document.entities.len(), 1);
        assert!(history.redo(&mut document));
        assert_eq!(
            document.entity_colors.get(&line_id).map(String::as_str),
            Some("#00ff00")
        );
        assert!(history.redo(&mut document));
        let Entity::Line { start, .. } = &document.entities[0] else {
            panic!("expected line");
        };
        assert!((start.x - 5.0).abs() < f64::EPSILON);
        assert!(history.redo(&mut document));
        assert!(document.entities.is_empty());
    }
}
