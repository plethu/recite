//! Godot 4 GDExtension adapter for Recite.
//!
//! The public Godot surface is intentionally thin: Godot classes and signals
//! translate to a host-independent adapter core that preserves Recite runtime
//! semantics.

mod adapter;
mod adapter_error;
mod adapter_model;
mod binding_types;
mod bindings;
mod convert;

pub use adapter::{
    AdapterValue, ConditionCall, ConditionHandlerResult, ReciteDialogueAsset, ReciteDialogueDriver,
    ReciteOutput,
};
pub use adapter_error::{AdapterError, AdapterErrorKind, AdapterResult};
pub use binding_types::{
    ReciteAdapterError, ReciteOperationResult, ReciteOutput as GodotReciteOutput,
};
pub use bindings::{ReciteDialogueNode, ReciteDialogueResource};

struct ReciteGodotExtension;

// godot-rust requires an unsafe marker impl for the dynamic GDExtension entry
// point. Runtime behavior remains inside safe adapter code.
#[godot::prelude::gdextension]
unsafe impl godot::prelude::ExtensionLibrary for ReciteGodotExtension {}
