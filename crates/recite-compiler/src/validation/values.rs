use recite_core::{Argument, ScalarValue, Value};

pub(super) fn value_has_non_finite_float(value: &Value) -> bool {
    match value {
        Value::Scalar(value) => scalar_has_non_finite_float(value),
        Value::Array(values) => values.iter().any(scalar_has_non_finite_float),
    }
}

pub(super) fn argument_has_non_finite_float(argument: &Argument) -> bool {
    match argument {
        Argument::Identifier(_) => false,
        Argument::Value(value) => scalar_has_non_finite_float(value),
    }
}

fn scalar_has_non_finite_float(value: &ScalarValue) -> bool {
    match value {
        ScalarValue::Float(value) => !value.is_finite(),
        ScalarValue::String(_) | ScalarValue::Integer(_) | ScalarValue::Boolean(_) => false,
    }
}
