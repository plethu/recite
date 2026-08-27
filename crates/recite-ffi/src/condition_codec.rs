use recite_runtime::{ConditionArgument, ConditionQuery};
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::io::Cursor;

/// ABI-v0 condition argument records. The named map representation is part of
/// the callback contract; it must not be replaced with the compiled asset's
/// positional tag representation.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum FfiConditionArg {
    Identifier { value: String },
    String { value: String },
    Integer { value: i64 },
    Float { value: f64 },
    Boolean { value: bool },
}

/// ABI-v0 condition result records returned by a host callback.
pub(crate) enum FfiConditionValue {
    Bool { value: bool },
    Enum { variant: String },
}

impl<'de> Deserialize<'de> for FfiConditionValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(ConditionValueVisitor)
    }
}

struct ConditionValueVisitor;

impl<'de> Visitor<'de> for ConditionValueVisitor {
    type Value = FfiConditionValue;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a two-field condition result map")
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut kind = None;
        let mut value = None;
        let mut variant = None;
        let mut field_count = 0;

        while let Some(key) = map.next_key::<String>()? {
            field_count += 1;
            match key.as_str() {
                "kind" => {
                    if kind.is_some() {
                        return Err(de::Error::duplicate_field("kind"));
                    }
                    kind = Some(map.next_value::<String>()?);
                }
                "value" => {
                    if value.is_some() {
                        return Err(de::Error::duplicate_field("value"));
                    }
                    value = Some(map.next_value::<bool>()?);
                }
                "variant" => {
                    if variant.is_some() {
                        return Err(de::Error::duplicate_field("variant"));
                    }
                    variant = Some(map.next_value::<String>()?);
                }
                _ => {
                    return Err(de::Error::unknown_field(
                        &key,
                        &["kind", "value", "variant"],
                    ));
                }
            }
        }

        if field_count != 2 {
            return Err(de::Error::custom(format!(
                "condition result map must contain exactly two fields, got {field_count}"
            )));
        }

        let kind = kind.ok_or_else(|| de::Error::missing_field("kind"))?;
        match (kind.as_str(), value, variant) {
            ("bool", Some(value), None) => Ok(FfiConditionValue::Bool { value }),
            ("enum", None, Some(variant)) => Ok(FfiConditionValue::Enum { variant }),
            ("bool", _, _) => Err(de::Error::custom(
                "condition bool result requires a boolean `value` field",
            )),
            ("enum", _, _) => Err(de::Error::custom(
                "condition enum result requires a string `variant` field",
            )),
            (kind, _, _) => Err(de::Error::unknown_variant(kind, &["bool", "enum"])),
        }
    }
}

pub(crate) fn encode_condition_args(query: ConditionQuery<'_>) -> Result<Vec<u8>, String> {
    let args: Vec<FfiConditionArg> = query
        .arguments()
        .iter()
        .map(|arg| match arg {
            ConditionArgument::Identifier(value) => FfiConditionArg::Identifier {
                value: value.to_owned(),
            },
            ConditionArgument::String(value) => FfiConditionArg::String {
                value: value.to_owned(),
            },
            ConditionArgument::Integer(value) => FfiConditionArg::Integer { value },
            ConditionArgument::Float(value) => FfiConditionArg::Float { value },
            ConditionArgument::Boolean(value) => FfiConditionArg::Boolean { value },
        })
        .collect();

    if query
        .arguments()
        .iter()
        .any(|argument| matches!(argument, ConditionArgument::Float(value) if !value.is_finite()))
    {
        return Err("condition arguments cannot contain a non-finite float".to_owned());
    }

    rmp_serde::to_vec_named(&args).map_err(|error| error.to_string())
}

pub(crate) fn decode_condition_value(bytes: &[u8]) -> Result<FfiConditionValue, String> {
    let mut deserializer = rmp_serde::Deserializer::new(Cursor::new(bytes));
    let value =
        FfiConditionValue::deserialize(&mut deserializer).map_err(|error| error.to_string())?;
    if deserializer.position() != bytes.len() as u64 {
        return Err("condition result contains trailing bytes".to_owned());
    }
    Ok(value)
}
