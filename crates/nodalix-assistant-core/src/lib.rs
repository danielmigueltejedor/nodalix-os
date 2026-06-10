//! Local-first assistant primitives for Nodalix OS.
//!
//! This crate deliberately avoids provider lock-in. It owns the contracts for
//! intents, permissions, context search and model providers; apps can build UI
//! or connectors around those contracts.

pub mod assistant;
pub mod context;
pub mod entities;
pub mod intents;
pub mod permissions;
pub mod providers;

pub use assistant::{AssistantCore, AssistantResponse, SuggestedAction};
pub use context::{ContextDocument, ContextIndex, ContextSearchResult, SourceRef};
pub use intents::{IntentDefinition, IntentManifest, IntentRegistry};
pub use permissions::{PermissionBroker, PermissionDecision, PermissionRequest};
pub use providers::{MockProvider, ModelProvider};
