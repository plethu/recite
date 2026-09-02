mod convert;
mod encode;
mod model;
#[cfg(test)]
mod tests;

pub(crate) use encode::{encode_batch, should_continue};
pub(crate) use model::FfiOutputEncodeError;

use crate::buffer::ReciteBuffer;
use crate::error::ReciteStatus;
use recite_runtime::DialogueEvent;

pub(crate) fn encode_batch_output(
    events: Vec<DialogueEvent>,
    encoder: fn(Vec<DialogueEvent>) -> Result<Vec<u8>, FfiOutputEncodeError>,
) -> Result<ReciteBuffer, (ReciteStatus, String)> {
    encoder(events)
        .map(ReciteBuffer::from_bytes)
        .map_err(flatten_output_encode_error)
}

fn flatten_output_encode_error(error: FfiOutputEncodeError) -> (ReciteStatus, String) {
    // This is the C ABI boundary: preserve the typed encoder error internally,
    // then expose the existing stable status and thread-local detail string.
    (ReciteStatus::DialogueFault, error.to_string())
}
