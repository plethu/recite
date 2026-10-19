//! Canonical v0 MessagePack encoder.

use crate::{compiled::CompiledDialogue, compiled::TableRange};
use serde::Serialize;

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

pub(super) fn write_messagepack(
    dialogue: &CompiledDialogue,
    writer: &mut impl std::io::Write,
) -> Result<(), super::CompiledAssetEncodeError> {
    rmp_serde::encode::write(writer, &root::MsgDialogue::from(dialogue))
        .map_err(|error| super::CompiledAssetEncodeError::MessagePack(error.to_string()))
}

fn range_to_u32<I: Copy>(range: TableRange<I>, index: impl Fn(I) -> u32) -> (u32, u32) {
    (index(range.start), range.len)
}
fn scalar_value_tag(value: &crate::ScalarValue) -> u8 {
    match value {
        crate::ScalarValue::String(_) => crate::compiled::V0_SCALAR_TAG_STRING,
        crate::ScalarValue::Integer(_) => crate::compiled::V0_SCALAR_TAG_INTEGER,
        crate::ScalarValue::Float(_) => crate::compiled::V0_SCALAR_TAG_FLOAT,
        crate::ScalarValue::Boolean(_) => crate::compiled::V0_SCALAR_TAG_BOOLEAN,
    }
}
fn value_tag(value: &crate::Value) -> u8 {
    match value {
        crate::Value::Scalar(_) => crate::compiled::V0_VALUE_TAG_SCALAR,
        crate::Value::Array(_) => crate::compiled::V0_VALUE_TAG_ARRAY,
    }
}

// Project each borrowed row as it is written instead of retaining whole wire tables.
pub(super) struct TableRows<I>(pub(super) I);

impl<I> Serialize for TableRows<I>
where
    I: ExactSizeIterator + Clone,
    I::Item: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_seq(self.0.clone())
    }
}
