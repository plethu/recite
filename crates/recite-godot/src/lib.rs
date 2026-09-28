//! Godot 4 GDExtension adapter for Recite.
//!
//! The public Godot surface is intentionally thin: Godot classes and signals
//! translate to a host-independent adapter core that preserves Recite runtime
//! semantics.

mod adapter;
mod adapter_error;
mod adapter_model;
mod adapter_policy;
mod adapter_surface;
mod binding_types;
mod bindings;
mod catalog;
mod catalog_persistence;
mod catalog_po_resource;
mod catalog_resource;
mod convert;
mod dialogue_resource;

pub use adapter::{ConditionHandlerResult, ReciteDialogueDriver};
pub use adapter_error::{AdapterError, AdapterErrorKind, AdapterResult};
pub use adapter_surface::{AdapterValue, ConditionCall, ReciteDialogueAsset, ReciteOutput};
pub use binding_types::{
    ReciteAdapterError, ReciteConditionFailure, ReciteOperationResult, ReciteOutputObject,
};
pub use bindings::ReciteDialogueNode;
pub use catalog::ReciteDialogueCatalog;
pub use catalog_resource::ReciteDialogueCatalogResource;
pub use dialogue_resource::ReciteDialogueResource;

struct ReciteGodotExtension;

// godot-rust requires an unsafe marker impl for the dynamic GDExtension entry
// point. Runtime behavior remains inside safe adapter code.
#[godot::prelude::gdextension]
unsafe impl godot::prelude::ExtensionLibrary for ReciteGodotExtension {}
