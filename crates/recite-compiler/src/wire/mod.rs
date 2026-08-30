mod inspection;
mod shared;

pub(crate) use inspection::serialize_inspection_json;

use crate::compile::CompileError;
use recite_core::encode_compiled_dialogue_messagepack;

pub(crate) fn serialize_messagepack(
    dialogue: &recite_core::CompiledDialogue,
) -> Result<Vec<u8>, CompileError> {
    encode_compiled_dialogue_messagepack(dialogue)
        .map_err(|error| CompileError::Serialization(error.to_string()))
}
