use godot::builtin::{GString, VarDictionary, Variant, VariantType};
use godot::prelude::ToGodot;
use recite_core::{ScalarValue, Value};
use recite_runtime::InterpolationValues;

use crate::adapter::{AdapterError, AdapterErrorKind, AdapterValue};

use super::core::set_variant;

pub(crate) fn adapter_value_variant(value: &AdapterValue) -> Variant {
    match value {
        AdapterValue::Identifier(value) | AdapterValue::String(value) => value.to_variant(),
        AdapterValue::Integer(value) => value.to_variant(),
        AdapterValue::Float(value) => value.to_variant(),
        AdapterValue::Boolean(value) => value.to_variant(),
    }
}

pub(crate) fn adapter_value_dictionary(value: &AdapterValue) -> VarDictionary {
    let mut dictionary = VarDictionary::new();
    dictionary.set("kind", adapter_value_kind(value));
    set_variant(&mut dictionary, "value", adapter_value_variant(value));
    dictionary
}

/// Copies a Godot scalar dictionary into the canonical runtime interpolation
/// map. Godot's `Variant` type preserves the scalar type; arrays, objects, and
/// other values are rejected rather than stringified.
pub(crate) fn interpolation_values(
    values: &VarDictionary,
) -> Result<InterpolationValues, AdapterError> {
    let mut parsed = InterpolationValues::new();
    for (key, value) in values.iter_shared() {
        let name = key.try_to::<GString>().map_err(|error| {
            AdapterError::with_detail(
                AdapterErrorKind::Localisation,
                format!("interpolation value key must be a string: {error}"),
            )
        })?;
        let name = name.to_string();
        if name.is_empty() {
            return Err(AdapterError::with_detail(
                AdapterErrorKind::Localisation,
                "interpolation value name is empty",
            ));
        }
        let value = match value.get_type() {
            VariantType::STRING => ScalarValue::String(
                value
                    .try_to::<GString>()
                    .map_err(|error| invalid_interpolation_value(&name, error))?
                    .to_string(),
            ),
            VariantType::INT => ScalarValue::Integer(
                value
                    .try_to::<i64>()
                    .map_err(|error| invalid_interpolation_value(&name, error))?,
            ),
            VariantType::FLOAT => {
                let value = value
                    .try_to::<f64>()
                    .map_err(|error| invalid_interpolation_value(&name, error))?;
                if !value.is_finite() {
                    return Err(invalid_interpolation_value(&name, "float must be finite"));
                }
                ScalarValue::Float(value)
            }
            VariantType::BOOL => ScalarValue::Boolean(
                value
                    .try_to::<bool>()
                    .map_err(|error| invalid_interpolation_value(&name, error))?,
            ),
            actual => {
                return Err(AdapterError::with_detail(
                    AdapterErrorKind::Localisation,
                    format!(
                        "interpolation value `{name}` must be a string, int, float, or bool; got {actual:?}"
                    ),
                ));
            }
        };
        parsed.insert(name, value);
    }
    Ok(parsed)
}

fn invalid_interpolation_value(name: &str, error: impl std::fmt::Display) -> AdapterError {
    AdapterError::with_detail(
        AdapterErrorKind::Localisation,
        format!("invalid interpolation value `{name}`: {error}"),
    )
}

fn adapter_value_kind(value: &AdapterValue) -> &'static str {
    match value {
        AdapterValue::Identifier(_) => "identifier",
        AdapterValue::String(_) => "string",
        AdapterValue::Integer(_) => "integer",
        AdapterValue::Float(_) => "float",
        AdapterValue::Boolean(_) => "boolean",
    }
}

pub(super) fn value_dictionary(value: &Value) -> VarDictionary {
    let mut dictionary = VarDictionary::new();
    match value {
        Value::Scalar(value) => {
            dictionary.set("kind", scalar_value_kind(value));
            set_variant(&mut dictionary, "value", scalar_variant(value));
        }
        Value::Array(values) => {
            dictionary.set("kind", "array");
            let mut array = godot::builtin::VarArray::new();
            for value in values {
                super::core::push_variant(&mut array, scalar_variant(value));
            }
            set_variant(&mut dictionary, "value", array.to_variant());
        }
    }
    dictionary
}

fn scalar_value_kind(value: &ScalarValue) -> &'static str {
    match value {
        ScalarValue::String(_) => "string",
        ScalarValue::Integer(_) => "integer",
        ScalarValue::Float(_) => "float",
        ScalarValue::Boolean(_) => "boolean",
    }
}

fn scalar_variant(value: &ScalarValue) -> Variant {
    match value {
        ScalarValue::String(value) => value.to_variant(),
        ScalarValue::Integer(value) => value.to_variant(),
        ScalarValue::Float(value) => value.to_variant(),
        ScalarValue::Boolean(value) => value.to_variant(),
    }
}
