//! Canonical byte encoding of compiled rows for revision-independent digests.

use recite_core::{
    ScalarValue,
    ast::InterpolationType,
    compiled::{
        CompiledArgument, CompiledConditionExpression, CompiledInterpolationBinding,
        CompiledInterpolationMode,
    },
};

use super::Digest;

pub(crate) fn condition_digest(condition: &CompiledConditionExpression) -> Digest {
    let mut hasher = blake3::Hasher::new();
    write_condition(&mut hasher, condition);
    *hasher.finalize().as_bytes()
}

pub(crate) fn call_digest(function: &str, arguments: &[CompiledArgument]) -> Digest {
    let mut hasher = blake3::Hasher::new();
    write_call(&mut hasher, function, arguments);
    *hasher.finalize().as_bytes()
}

pub(crate) fn arguments_digest(arguments: &[CompiledArgument]) -> Digest {
    let mut hasher = blake3::Hasher::new();
    write_arguments(&mut hasher, arguments);
    *hasher.finalize().as_bytes()
}

pub(super) fn write_condition(
    hasher: &mut blake3::Hasher,
    condition: &CompiledConditionExpression,
) {
    match condition {
        CompiledConditionExpression::Call(call) => {
            write_tag(hasher, b"call");
            write_call(hasher, &call.function, &call.args);
        }
        CompiledConditionExpression::And(terms) => {
            write_tag(hasher, b"and");
            write_len(hasher, terms.len());
            for term in terms {
                write_condition(hasher, term);
            }
        }
        CompiledConditionExpression::Or(terms) => {
            write_tag(hasher, b"or");
            write_len(hasher, terms.len());
            for term in terms {
                write_condition(hasher, term);
            }
        }
        CompiledConditionExpression::Not(term) => {
            write_tag(hasher, b"not");
            write_condition(hasher, term);
        }
    }
}

pub(super) fn write_call(
    hasher: &mut blake3::Hasher,
    function: &str,
    arguments: &[CompiledArgument],
) {
    write_str(hasher, function);
    write_arguments(hasher, arguments);
}

pub(super) fn write_arguments(hasher: &mut blake3::Hasher, arguments: &[CompiledArgument]) {
    write_len(hasher, arguments.len());
    for argument in arguments {
        match argument {
            CompiledArgument::Identifier(identifier) => {
                write_tag(hasher, b"identifier");
                write_str(hasher, identifier);
            }
            CompiledArgument::Value(value) => {
                write_tag(hasher, b"value");
                write_scalar(hasher, value);
            }
        }
    }
}

pub(super) fn write_scalar(hasher: &mut blake3::Hasher, value: &ScalarValue) {
    match value {
        ScalarValue::String(value) => {
            write_tag(hasher, b"string");
            write_str(hasher, value);
        }
        ScalarValue::Integer(value) => {
            write_tag(hasher, b"integer");
            hasher.update(&value.to_le_bytes());
        }
        ScalarValue::Float(value) => {
            write_tag(hasher, b"float");
            hasher.update(&value.to_bits().to_le_bytes());
        }
        ScalarValue::Boolean(value) => {
            write_tag(hasher, b"boolean");
            hasher.update(&[u8::from(*value)]);
        }
    }
}

pub(super) fn write_bindings(
    hasher: &mut blake3::Hasher,
    bindings: &[CompiledInterpolationBinding],
) {
    write_len(hasher, bindings.len());
    for binding in bindings {
        write_str(hasher, &binding.name);
        write_str(hasher, &binding.value);
        write_tag(
            hasher,
            match binding.value_type {
                InterpolationType::String => b"string",
                InterpolationType::Integer => b"integer",
                InterpolationType::Float => b"float",
                InterpolationType::Boolean => b"boolean",
            },
        );
    }
}

pub(super) fn write_interpolation_mode(
    hasher: &mut blake3::Hasher,
    mode: CompiledInterpolationMode,
) {
    write_tag(
        hasher,
        match mode {
            CompiledInterpolationMode::Current => b"current",
            CompiledInterpolationMode::Legacy => b"legacy",
        },
    );
}

pub(super) fn write_optional_str(hasher: &mut blake3::Hasher, value: Option<&str>) {
    match value {
        Some(value) => {
            write_tag(hasher, b"some");
            write_str(hasher, value);
        }
        None => write_tag(hasher, b"none"),
    }
}

pub(super) fn write_tag(hasher: &mut blake3::Hasher, tag: &[u8]) {
    write_bytes(hasher, tag);
}

pub(super) fn write_str(hasher: &mut blake3::Hasher, value: &str) {
    write_bytes(hasher, value.as_bytes());
}

pub(super) fn write_bytes(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    write_len(hasher, bytes.len());
    hasher.update(bytes);
}

pub(super) fn write_u32(hasher: &mut blake3::Hasher, value: u32) {
    hasher.update(&value.to_le_bytes());
}

pub(super) fn write_len(hasher: &mut blake3::Hasher, len: usize) {
    hasher.update(&(len as u64).to_le_bytes());
}
