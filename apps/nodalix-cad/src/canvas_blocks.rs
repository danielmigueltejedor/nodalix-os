//! BLOCK tool: create from selection, insert, explode, commands.

use crate::{
    cad::geometry::blocks::{
        clone_entity_to_block_space, explode_block_reference, instanced_entities,
        normalize_block_name, selection_bbox_center,
    },
    cad::history::{
        capture_entity_snapshots, entity_snapshot_for_add, record_create_block_from_selection,
        record_entities_added, record_explode_block_reference, EntitySnapshot,
        LegacyHistoryManager,
    },
    document::{BlockDefinition, Document, Entity},
    geometry::Point,
    tool_parameters::ToolParametersState,
};
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

#[derive(Clone, Debug)]
pub struct BlockInsertPending {
    pub block_name: String,
}

pub fn active_layer_allows_block_edit(document: &Document) -> bool {
    !document.layer_locked(&document.active_layer_name)
}

pub fn create_block_from_selection(
    document: &mut Document,
    history: &mut LegacyHistoryManager,
    name: &str,
    selected_ids: &[u64],
) -> Result<u64, &'static str> {
    if !active_layer_allows_block_edit(document) {
        return Err("active layer locked");
    }
    let name = normalize_block_name(name);
    if name.is_empty() {
        return Err("empty block name");
    }
    let ids: Vec<u64> = selected_ids
        .iter()
        .copied()
        .filter(|id| document.entities.iter().any(|e| e.id() == *id))
        .filter(|id| {
            document
                .entities
                .iter()
                .find(|e| e.id() == *id)
                .map(|e| !matches!(e, Entity::BlockReference { .. }))
                .unwrap_or(false)
        })
        .collect();
    if ids.is_empty() {
        return Err("no valid selection");
    }
    let base_point = selection_bbox_center(&ids, document).ok_or("empty bounds")?;
    let previous_block = document.block_definition(&name).cloned();
    let removed = capture_entity_snapshots(document, &ids);
    let mut block_entities = Vec::new();
    for id in &ids {
        let entity = document
            .entities
            .iter()
            .find(|e| e.id() == *id)
            .ok_or("missing entity")?;
        block_entities.push(clone_entity_to_block_space(entity, base_point));
    }
    for id in &ids {
        document.remove_entity(*id);
    }
    let new_block = BlockDefinition {
        name: name.clone(),
        base_point,
        entities: block_entities,
    };
    document.insert_block_definition(new_block.clone());
    let reference_id = document.next_id();
    let reference = Entity::BlockReference {
        id: reference_id,
        layer: document.active_layer_name.clone(),
        name: name.clone(),
        insertion: base_point,
        scale: 1.0,
        scale_y: None,
        rotation: 0.0,
    };
    document.add_entity(reference.clone());
    document.modified = true;
    let reference_snapshot = entity_snapshot_for_add(reference, &document.active_layout);
    record_create_block_from_selection(
        history,
        name,
        previous_block,
        new_block,
        removed,
        reference_snapshot,
    );
    Ok(reference_id)
}

pub fn insert_block_reference(
    document: &mut Document,
    history: &mut LegacyHistoryManager,
    name: &str,
    insertion: Point,
    params: &ToolParametersState,
) -> Option<u64> {
    if !active_layer_allows_block_edit(document) {
        return None;
    }
    let name = normalize_block_name(name);
    if document.block_definition(&name).is_none() {
        return None;
    }
    let id = document.next_id();
    let entity = Entity::BlockReference {
        id,
        layer: document.active_layer_name.clone(),
        name,
        insertion,
        scale: params.block_scale,
        scale_y: Some(params.block_scale_y),
        rotation: params.block_rotation.to_radians(),
    };
    document.add_entity(entity);
    document.modified = true;
    record_entities_added(history, document, &[id]);
    Some(id)
}

pub fn build_block_insert_preview(
    document: &Document,
    name: &str,
    insertion: Point,
    params: &ToolParametersState,
) -> Vec<Entity> {
    let name = normalize_block_name(name);
    instanced_entities(
        document,
        &name,
        insertion,
        params.block_scale,
        Some(params.block_scale_y),
        params.block_rotation.to_radians(),
        0,
        &mut BTreeSet::new(),
    )
}

pub fn explode_selected_block_reference(
    document: &mut Document,
    history: &mut LegacyHistoryManager,
    reference_id: u64,
) -> Option<Vec<u64>> {
    if document.entity_layer_locked(reference_id) {
        return None;
    }
    let reference = document
        .entities
        .iter()
        .find(|e| e.id() == reference_id)?
        .clone();
    let exploded = explode_block_reference(document, &reference)?;
    if exploded.is_empty() {
        return None;
    }
    let reference_snapshot = capture_entity_snapshots(document, &[reference_id])
        .into_iter()
        .next()?;
    document.remove_entity(reference_id);
    let mut added_ids = Vec::new();
    let mut added_snapshots = Vec::new();
    let mut next_id = reference_id.max(document.entities.iter().map(Entity::id).max().unwrap_or(0));
    for mut entity in exploded {
        next_id += 1;
        let id = next_id;
        if let Entity::Point { id: entity_id, .. }
        | Entity::Line { id: entity_id, .. }
        | Entity::Polyline { id: entity_id, .. }
        | Entity::Spline { id: entity_id, .. }
        | Entity::Circle { id: entity_id, .. }
        | Entity::Text { id: entity_id, .. }
        | Entity::Dimension { id: entity_id, .. }
        | Entity::Hatch { id: entity_id, .. }
        | Entity::Table { id: entity_id, .. }
        | Entity::BlockReference { id: entity_id, .. }
        | Entity::Guideline { id: entity_id, .. } = &mut entity
        {
            *entity_id = id;
        }
        let snapshot = entity_snapshot_for_add(entity.clone(), &document.active_layout);
        document.add_entity(entity);
        added_ids.push(id);
        added_snapshots.push(snapshot);
    }
    document.modified = true;
    record_explode_block_reference(history, reference_snapshot, added_snapshots);
    Some(added_ids)
}

pub fn handle_block_insert_click(
    document: &mut Document,
    history: &Rc<RefCell<LegacyHistoryManager>>,
    point: Point,
    pending: &Rc<RefCell<Option<BlockInsertPending>>>,
    params: &ToolParametersState,
) -> bool {
    let Some(insert) = pending.borrow().clone() else {
        return false;
    };
    let _ = insert_block_reference(
        document,
        &mut history.borrow_mut(),
        &insert.block_name,
        point,
        params,
    );
    *pending.borrow_mut() = None;
    true
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockCommandOutcome {
    ActivateInsert { block_name: String },
    CreateFromSelection { block_name: String },
    Explode,
}

pub fn try_execute_block_command(
    command: &str,
    selected_ids: &[u64],
) -> Option<BlockCommandOutcome> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    let Some(cmd) = parts.first().copied() else {
        return None;
    };
    if cmd.eq_ignore_ascii_case("explode") || cmd == "x" {
        return Some(BlockCommandOutcome::Explode);
    }
    if cmd.eq_ignore_ascii_case("block") {
        let name = parts.get(1).copied().unwrap_or("").trim();
        if name.is_empty() {
            return None;
        }
        return Some(BlockCommandOutcome::CreateFromSelection {
            block_name: name.to_string(),
        });
    }
    if cmd.eq_ignore_ascii_case("insert") || cmd == "i" {
        let name = parts.get(1).copied()?.trim();
        if name.is_empty() {
            return None;
        }
        return Some(BlockCommandOutcome::ActivateInsert {
            block_name: name.to_string(),
        });
    }
    let _ = selected_ids;
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad::history::LegacyHistoryManager;

    #[test]
    fn command_parser_block_insert_explode() {
        assert!(matches!(
            try_execute_block_command("block Door", &[]),
            Some(BlockCommandOutcome::CreateFromSelection { .. })
        ));
        assert!(matches!(
            try_execute_block_command("insert Door", &[]),
            Some(BlockCommandOutcome::ActivateInsert { .. })
        ));
        assert!(matches!(
            try_execute_block_command("i Door", &[]),
            Some(BlockCommandOutcome::ActivateInsert { .. })
        ));
        assert!(matches!(
            try_execute_block_command("explode", &[]),
            Some(BlockCommandOutcome::Explode)
        ));
        assert!(matches!(
            try_execute_block_command("x", &[]),
            Some(BlockCommandOutcome::Explode)
        ));
    }

    #[test]
    fn insert_creates_block_reference() {
        let mut doc = Document::new_empty();
        doc.insert_block_definition(BlockDefinition {
            name: "B".to_string(),
            base_point: Point { x: 0.0, y: 0.0 },
            entities: vec![Entity::Line {
                id: 0,
                layer: "Default".to_string(),
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 1.0, y: 0.0 },
            }],
        });
        let mut history = LegacyHistoryManager::new();
        let params = ToolParametersState::default();
        let id = insert_block_reference(
            &mut doc,
            &mut history,
            "B",
            Point { x: 5.0, y: 5.0 },
            &params,
        )
        .unwrap();
        assert!(doc.entities.iter().any(|e| e.id() == id));
    }

    #[test]
    fn create_block_undo_redo() {
        let mut doc = Document::new_empty();
        let line_id = doc.next_id();
        doc.add_entity(Entity::Line {
            id: line_id,
            layer: "Default".to_string(),
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 10.0, y: 0.0 },
        });
        let mut history = LegacyHistoryManager::new();
        create_block_from_selection(&mut doc, &mut history, "Sym", &[line_id]).unwrap();
        assert_eq!(doc.entities.len(), 1);
        assert!(doc.block_definition("Sym").is_some());
        assert!(history.undo(&mut doc));
        assert_eq!(doc.entities.len(), 1);
        assert!(doc.block_definition("Sym").is_none());
        assert!(history.redo(&mut doc));
        assert!(doc.block_definition("Sym").is_some());
    }

    #[test]
    fn insert_undo_redo() {
        let mut doc = Document::new_empty();
        doc.insert_block_definition(BlockDefinition {
            name: "B".to_string(),
            base_point: Point { x: 0.0, y: 0.0 },
            entities: vec![Entity::Line {
                id: 0,
                layer: "Default".to_string(),
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 1.0, y: 0.0 },
            }],
        });
        let mut history = LegacyHistoryManager::new();
        let params = ToolParametersState::default();
        let id = insert_block_reference(
            &mut doc,
            &mut history,
            "B",
            Point { x: 1.0, y: 1.0 },
            &params,
        )
        .unwrap();
        assert_eq!(doc.entities.len(), 1);
        assert!(history.undo(&mut doc));
        assert!(doc.entities.is_empty());
        assert!(history.redo(&mut doc));
        assert!(doc.entities.iter().any(|e| e.id() == id));
    }

    #[test]
    fn block_on_hidden_layer_not_visible() {
        let mut doc = Document::new_empty();
        doc.insert_block_definition(BlockDefinition {
            name: "H".to_string(),
            base_point: Point { x: 0.0, y: 0.0 },
            entities: vec![Entity::Line {
                id: 0,
                layer: "Default".to_string(),
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 10.0, y: 0.0 },
            }],
        });
        let id = doc.next_id();
        doc.add_entity(Entity::BlockReference {
            id,
            layer: "Default".to_string(),
            name: "H".to_string(),
            insertion: Point { x: 0.0, y: 0.0 },
            scale: 1.0,
            scale_y: None,
            rotation: 0.0,
        });
        doc.set_layer_visible("Default", false);
        assert!(!doc.entity_visible_in_active_layout(id));
    }

    #[test]
    fn explode_fails_on_locked_layer() {
        let mut doc = Document::new_empty();
        doc.create_layer("Locked");
        doc.set_layer_locked("Locked", true);
        doc.insert_block_definition(BlockDefinition {
            name: "E".to_string(),
            base_point: Point { x: 0.0, y: 0.0 },
            entities: vec![Entity::Line {
                id: 0,
                layer: "Locked".to_string(),
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 4.0, y: 0.0 },
            }],
        });
        let ref_id = doc.next_id();
        doc.add_entity(Entity::BlockReference {
            id: ref_id,
            layer: "Locked".to_string(),
            name: "E".to_string(),
            insertion: Point { x: 0.0, y: 0.0 },
            scale: 1.0,
            scale_y: None,
            rotation: 0.0,
        });
        let mut history = LegacyHistoryManager::new();
        assert!(explode_selected_block_reference(&mut doc, &mut history, ref_id).is_none());
    }

    #[test]
    fn explode_undo_redo() {
        let mut doc = Document::new_empty();
        doc.insert_block_definition(BlockDefinition {
            name: "E".to_string(),
            base_point: Point { x: 0.0, y: 0.0 },
            entities: vec![Entity::Line {
                id: 0,
                layer: "Default".to_string(),
                start: Point { x: 0.0, y: 0.0 },
                end: Point { x: 4.0, y: 0.0 },
            }],
        });
        let ref_id = doc.next_id();
        doc.add_entity(Entity::BlockReference {
            id: ref_id,
            layer: "Default".to_string(),
            name: "E".to_string(),
            insertion: Point { x: 0.0, y: 0.0 },
            scale: 1.0,
            scale_y: None,
            rotation: 0.0,
        });
        let mut history = LegacyHistoryManager::new();
        explode_selected_block_reference(&mut doc, &mut history, ref_id).unwrap();
        assert_eq!(doc.entities.len(), 1);
        assert!(!doc.entities.iter().any(|e| e.id() == ref_id));
        assert!(history.undo(&mut doc));
        assert!(doc.entities.iter().any(|e| e.id() == ref_id));
    }
}
