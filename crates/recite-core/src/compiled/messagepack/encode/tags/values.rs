use super::super::{scalar_value_tag, value_tag};
use super::{
    MsgArgument, MsgConditionCall, MsgConditionExpression, MsgScalarValue, MsgSourceSpan, MsgValue,
};
use crate::{CompiledArgument, CompiledConditionExpression, ScalarValue, Value};
use serde::Serialize;
use serde::ser::SerializeTuple;

impl Serialize for MsgConditionExpression<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            CompiledConditionExpression::Call(call) => serialize_tagged!(
                serializer,
                crate::V0_CONDITION_TAG_CALL,
                MsgConditionCall(call)
            ),
            CompiledConditionExpression::And(expressions) => serialize_tagged!(
                serializer,
                crate::V0_CONDITION_TAG_AND,
                expressions
                    .iter()
                    .map(MsgConditionExpression)
                    .collect::<Vec<_>>()
            ),
            CompiledConditionExpression::Or(expressions) => serialize_tagged!(
                serializer,
                crate::V0_CONDITION_TAG_OR,
                expressions
                    .iter()
                    .map(MsgConditionExpression)
                    .collect::<Vec<_>>()
            ),
            CompiledConditionExpression::Not(expression) => serialize_tagged!(
                serializer,
                crate::V0_CONDITION_TAG_NOT,
                MsgConditionExpression(expression)
            ),
        }
    }
}

impl Serialize for MsgConditionCall<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(crate::V0_CONDITION_CALL_FIELDS as usize)?;
        tuple.serialize_element(self.0.function.as_str())?;
        tuple.serialize_element(&self.0.args.iter().map(MsgArgument).collect::<Vec<_>>())?;
        tuple.end()
    }
}

impl Serialize for MsgArgument<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            CompiledArgument::Identifier(value) => {
                serialize_tagged!(serializer, crate::V0_ARGUMENT_TAG_IDENTIFIER, value)
            }
            CompiledArgument::Value(value) => serialize_tagged!(
                serializer,
                crate::V0_ARGUMENT_TAG_VALUE,
                MsgScalarValue(value)
            ),
        }
    }
}

impl Serialize for MsgValue<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            Value::Scalar(value) => {
                serialize_tagged!(serializer, value_tag(self.0), MsgScalarValue(value))
            }
            Value::Array(values) => serialize_tagged!(
                serializer,
                value_tag(self.0),
                values.iter().map(MsgScalarValue).collect::<Vec<_>>()
            ),
        }
    }
}

impl Serialize for MsgScalarValue<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            ScalarValue::String(value) => {
                serialize_tagged!(serializer, scalar_value_tag(self.0), value)
            }
            ScalarValue::Integer(value) => {
                serialize_tagged!(serializer, scalar_value_tag(self.0), value)
            }
            ScalarValue::Float(value) => {
                serialize_tagged!(serializer, scalar_value_tag(self.0), value)
            }
            ScalarValue::Boolean(value) => {
                serialize_tagged!(serializer, scalar_value_tag(self.0), value)
            }
        }
    }
}

impl Serialize for MsgSourceSpan<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(crate::V0_SOURCE_SPAN_FIELDS as usize)?;
        tuple.serialize_element(self.0.file.as_str())?;
        tuple.serialize_element(&self.0.start.line())?;
        tuple.serialize_element(&self.0.start.column())?;
        tuple.serialize_element(&self.0.end.map(|end| end.line()))?;
        tuple.serialize_element(&self.0.end.map(|end| end.column()))?;
        tuple.end()
    }
}
