//! Scalar, argument and availability-reason value tags.
use recite_core::{
    ScalarValue, Value,
    compiled::{
        CompiledArgument, CompiledAvailabilityReasonArgBinding, CompiledAvailabilityReasonArgValue,
    },
};
use serde::{Serialize, Serializer, ser::SerializeStruct};

use super::{Inspection, Rows, serialize_tagged};

impl Serialize for Inspection<'_, ScalarValue> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            ScalarValue::String(value) => serialize_tagged(serializer, "string", value),
            ScalarValue::Integer(value) => serialize_tagged(serializer, "integer", value),
            ScalarValue::Float(value) => serialize_tagged(serializer, "float", value),
            ScalarValue::Boolean(value) => serialize_tagged(serializer, "boolean", value),
        }
    }
}

impl Serialize for Inspection<'_, Value> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Value::Scalar(value) => serialize_tagged(serializer, "scalar", &Inspection(value)),
            Value::Array(values) => serialize_tagged(serializer, "array", &Rows(values)),
        }
    }
}

impl Serialize for Inspection<'_, CompiledArgument> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            CompiledArgument::Identifier(value) => {
                serialize_tagged(serializer, "identifier", value)
            }
            CompiledArgument::Value(value) => {
                serialize_tagged(serializer, "value", &Inspection(value))
            }
        }
    }
}

impl Serialize for Inspection<'_, CompiledAvailabilityReasonArgBinding> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let binding = self.0;
        let mut object = serializer.serialize_struct("CompiledAvailabilityReasonArgBinding", 2)?;
        object.serialize_field("name", binding.name.as_str())?;
        object.serialize_field("value", &Inspection(&binding.value))?;
        object.end()
    }
}

impl Serialize for Inspection<'_, CompiledAvailabilityReasonArgValue> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            CompiledAvailabilityReasonArgValue::ConditionArg(index) => {
                serialize_tagged(serializer, "condition_arg", index)
            }
            CompiledAvailabilityReasonArgValue::Literal(value) => match value {
                ScalarValue::String(value) => serialize_tagged(serializer, "literal_string", value),
                ScalarValue::Integer(value) => serialize_tagged(serializer, "literal_int", value),
                ScalarValue::Float(value) => serialize_tagged(serializer, "literal_float", value),
                ScalarValue::Boolean(value) => serialize_tagged(serializer, "literal_bool", value),
            },
        }
    }
}
