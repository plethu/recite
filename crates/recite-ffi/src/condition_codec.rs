use crate::tagged_value::TaggedValue;
use recite_runtime::{ConditionArgument, ConditionQuery};
use serde::de::{MapAccess, Visitor, value::MapAccessDeserializer};
use serde::{Deserialize, Deserializer};
use std::io::Cursor;

/// ABI-v0 condition result records returned by a host callback.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum FfiConditionValue {
    Bool { value: bool },
    Enum { variant: String },
}

struct ConditionValueVisitor;

impl<'de> Visitor<'de> for ConditionValueVisitor {
    type Value = FfiConditionValue;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a named condition result map")
    }

    fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        FfiConditionValue::deserialize(MapAccessDeserializer::new(map))
    }
}

pub(crate) fn encode_condition_args(query: ConditionQuery<'_>) -> Result<Vec<u8>, String> {
    if query
        .arguments()
        .iter()
        .any(|argument| matches!(argument, ConditionArgument::Float(value) if !value.is_finite()))
    {
        return Err("condition arguments cannot contain a non-finite float".to_owned());
    }
    let args: Vec<TaggedValue> = query.arguments().iter().map(TaggedValue::from).collect();
    rmp_serde::to_vec_named(&args).map_err(|error| error.to_string())
}

pub(crate) fn decode_condition_value(bytes: &[u8]) -> Result<FfiConditionValue, String> {
    let mut deserializer = rmp_serde::Deserializer::new(Cursor::new(bytes));
    let value = deserializer
        .deserialize_map(ConditionValueVisitor)
        .map_err(|error| error.to_string())?;
    if deserializer.position() != bytes.len() as u64 {
        return Err("condition result contains trailing bytes".to_owned());
    }
    Ok(value)
}
