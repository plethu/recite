use std::io::Write;

use recite_runtime::{DialogueEffectMode, DialogueEffectRequest, DialogueEvent};

use super::convert::ffi_event;
use super::model::{FfiOutputBatch, FfiOutputEncodeError};

pub(crate) fn encode_batch(events: Vec<DialogueEvent>) -> Result<Vec<u8>, FfiOutputEncodeError> {
    let mut bytes = Vec::new();
    encode_batch_to_writer(events, &mut bytes)?;
    Ok(bytes)
}

pub(crate) fn encode_batch_to_writer<W: Write + ?Sized>(
    events: Vec<DialogueEvent>,
    writer: &mut W,
) -> Result<(), FfiOutputEncodeError> {
    let ffi_events = events.into_iter().map(ffi_event).collect();
    let batch = FfiOutputBatch {
        batch_format_version: super::model::BATCH_FORMAT_VERSION,
        events: ffi_events,
    };
    rmp_serde::encode::write_named(writer, &batch).map_err(|source| FfiOutputEncodeError { source })
}

/// Returns true for events that do not stop the drain loop.
pub(crate) fn should_continue(event: &DialogueEvent) -> bool {
    matches!(
        event,
        DialogueEvent::Line(_)
            | DialogueEvent::Effect(DialogueEffectRequest {
                mode: DialogueEffectMode::Immediate,
                ..
            })
    )
}
