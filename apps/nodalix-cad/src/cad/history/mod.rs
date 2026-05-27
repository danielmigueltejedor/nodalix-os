pub mod actions;
pub mod history_manager;
pub mod legacy_actions;
pub mod legacy_history_manager;

pub use actions::{AddEntityAction, HistoryAction, MoveEntityAction, RemoveEntityAction};
pub use history_manager::HistoryManager;
pub use legacy_actions::{
    apply_entity_property_state, capture_entity_property_state, capture_entity_snapshot,
    capture_entity_snapshots, entity_snapshot_for_add, record_entities_added,
    record_entity_move_if_nonzero, record_entity_property_changes, restore_entity_snapshot,
    EntityPropertyChange, EntityPropertyState, EntitySnapshot, LegacyAddEntitiesAction,
    LegacyHistoryAction, LegacyMoveEntitiesAction, LegacyPasteEntitiesAction,
    LegacyRemoveEntitiesAction, LegacyUpdateEntityPropertiesAction,
};
pub use legacy_history_manager::LegacyHistoryManager;
