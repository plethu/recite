use recite_core::{Argument, ScalarValue, SourceMetadataScalar, SourceMetadataValue};

pub(super) fn source_metadata_value_has_non_finite_float(value: &SourceMetadataValue) -> bool {
    match value {
        SourceMetadataValue::Scalar(value) => source_metadata_scalar_has_non_finite_float(value),
        SourceMetadataValue::Array(values) => values
            .iter()
            .any(source_metadata_scalar_has_non_finite_float),
    }
}

pub(super) fn argument_has_non_finite_float(argument: &Argument) -> bool {
    match argument {
        Argument::Identifier(_) => false,
        Argument::Value(value) => scalar_has_non_finite_float(value),
    }
}

fn source_metadata_scalar_has_non_finite_float(value: &SourceMetadataScalar) -> bool {
    match value {
        SourceMetadataScalar::Float(value) => !value.is_finite(),
        SourceMetadataScalar::Symbol(_)
        | SourceMetadataScalar::StringLiteral(_)
        | SourceMetadataScalar::Integer(_)
        | SourceMetadataScalar::Bool(_) => false,
    }
}

fn scalar_has_non_finite_float(value: &ScalarValue) -> bool {
    match value {
        ScalarValue::Float(value) => !value.is_finite(),
        ScalarValue::String(_) | ScalarValue::Integer(_) | ScalarValue::Boolean(_) => false,
    }
}
