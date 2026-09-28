use bevy_asset::AssetId;
use bevy_ecs::message::Message;
use recite_adapter::AdapterError;
use recite_runtime::DialogueEvent;

use crate::ReciteDialogueAsset;

/// One observed result. Each request gets a monotonic sequence number and an
/// ordered output batch so independent readers see the same runtime order.
#[derive(Message, Clone, Debug)]
pub struct ReciteOutput {
    pub sequence: u64,
    pub value: ReciteOutputValue,
}

#[derive(Clone, Debug)]
pub enum ReciteOutputValue {
    Dialogue(Vec<DialogueEvent>),
    Snapshot(Vec<u8>),
    Ended,
    Error {
        error: AdapterError,
        handle_id: Option<AssetId<ReciteDialogueAsset>>,
    },
}
