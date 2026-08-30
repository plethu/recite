//! Canonical v0 MessagePack encoder.

use crate::{CompiledDialogue, TableRange};

mod root;
mod rows;
mod tables;
mod tags;

pub(super) fn serialize_messagepack(
    dialogue: &CompiledDialogue,
) -> Result<Vec<u8>, super::CompiledAssetEncodeError> {
    rmp_serde::to_vec(&root::MsgDialogue::from(dialogue))
        .map_err(|error| super::CompiledAssetEncodeError::MessagePack(error.to_string()))
}

fn range_to_u32<I: Copy>(range: TableRange<I>, index: impl Fn(I) -> u32) -> (u32, u32) {
    (index(range.start), range.len)
}
fn scalar_value_tag(value: &crate::ScalarValue) -> u8 {
    match value {
        crate::ScalarValue::String(_) => crate::V0_SCALAR_TAG_STRING,
        crate::ScalarValue::Integer(_) => crate::V0_SCALAR_TAG_INTEGER,
        crate::ScalarValue::Float(_) => crate::V0_SCALAR_TAG_FLOAT,
        crate::ScalarValue::Boolean(_) => crate::V0_SCALAR_TAG_BOOLEAN,
    }
}
fn value_tag(value: &crate::Value) -> u8 {
    match value {
        crate::Value::Scalar(_) => crate::V0_VALUE_TAG_SCALAR,
        crate::Value::Array(_) => crate::V0_VALUE_TAG_ARRAY,
    }
}
