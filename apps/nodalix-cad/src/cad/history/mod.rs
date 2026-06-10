pub mod actions;
pub mod history_manager;
pub mod legacy_actions;
pub mod legacy_history_manager;

pub use actions::{AddEntityAction, HistoryAction, MoveEntityAction, RemoveEntityAction};
pub use history_manager::HistoryManager;
pub use legacy_actions::{
    apply_entity_property_state, apply_entity_snapshot_in_place, apply_layer_state,
    capture_entity_property_state, capture_entity_snapshot, capture_entity_snapshots,
    capture_layer_state, entity_snapshot_for_add, record_create_block_from_selection,
    record_entities_added, record_entity_move_if_nonzero, record_entity_property_changes,
    record_entity_transform, record_explode_block_reference, restore_entity_snapshot,
    EntityPropertyChange, EntityPropertyState, EntitySnapshot, LayerStateSnapshot,
    LegacyAddEntitiesAction, LegacyCreateBlockFromSelectionAction,
    LegacyExplodeBlockReferenceAction, LegacyHistoryAction, LegacyModifyAndAddEntitiesAction,
    LegacyMoveEntitiesAction, LegacyPasteEntitiesAction, LegacyRemoveEntitiesAction,
    LegacyTransformEntitiesAction, LegacyUpdateEntityPropertiesAction, LegacyUpdateLayersAction,
};
pub use legacy_history_manager::LegacyHistoryManager;
