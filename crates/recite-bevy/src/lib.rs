//! Bevy 0.19 companion for Recite compiled dialogue.
//!
//! Add [`RecitePlugin`] after Bevy's `AssetPlugin`. A single [`ReciteOwner`]
//! resource owns one session. Send [`ReciteRequest`] messages in order and read
//! [`ReciteOutput`] messages after [`ReciteSet::ProcessRequests`].
//! The adapter never executes game-side effects.

mod asset;
mod context;
mod output;
mod plugin;
mod schema;
mod session;

pub use asset::{
    ReciteAssetFreshness, ReciteAssetImport, ReciteAssetStatus, ReciteDialogueAsset,
    ReciteDialogueLoadError, ReciteDialogueLoader, ReciteRevision,
};
pub use context::{ReciteCatalog, ReciteConditions, ReciteInterpolation};
pub use output::{ReciteOutput, ReciteOutputValue};
pub use plugin::{RecitePlugin, ReciteSet};
pub use schema::{
    ConditionBuilder, EffectBuilder, ReciteEnum, ReciteSchema, ReciteType, SchemaRegistrationError,
};
pub use session::{ReciteOwner, ReciteRequest};

pub use recite_adapter::{AdapterError, AdapterErrorKind, ReciteDialogueCatalog};
