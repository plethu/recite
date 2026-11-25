use bevy_asset::AssetId;
use bevy_ecs::message::Message;
use recite_adapter::AdapterError;
use recite_runtime::DialogueEvent;
use recite_runtime::resume::{ResumeFallback, ResumeReport};

use crate::ReciteDialogueAsset;
mod sequence;
pub use sequence::OutputSequence;

/// One observed result. Each request gets a monotonic sequence number and an
/// ordered output batch so independent readers see the same runtime order.
#[derive(Message, Clone, Debug)]
pub struct ReciteOutput {
    pub sequence: OutputSequence,
    pub value: ReciteOutputValue,
}

#[derive(Clone, Debug)]
pub enum ReciteOutputValue {
    Dialogue(Vec<DialogueEvent>),
    /// A restored save is active again. `events` is its first batch, which
    /// re-presents a pending prompt or re-emits a pending blocking effect.
    Resumed {
        report: ResumeReport,
        events: Vec<DialogueEvent>,
    },
    /// A save could not resume in the current revision and no session is
    /// active. The fallback returns the saved block and effects.
    ResumeFallback(Box<ResumeFallback>),
    Snapshot(Vec<u8>),
    Ended,
    Error {
        error: AdapterError,
        handle_id: Option<AssetId<ReciteDialogueAsset>>,
    },
}
