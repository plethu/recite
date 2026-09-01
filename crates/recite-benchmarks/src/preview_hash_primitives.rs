pub(super) fn tag(hasher: &mut blake3::Hasher, value: u8) {
    hasher.update(&[value]);
}
pub(super) fn hash_u64(hasher: &mut blake3::Hasher, value: u64) {
    hasher.update(&value.to_le_bytes());
}
pub(super) fn hash_i64(hasher: &mut blake3::Hasher, value: i64) {
    hasher.update(&value.to_le_bytes());
}
pub(super) fn hash_len(hasher: &mut blake3::Hasher, value: usize) {
    hash_u64(hasher, value as u64);
}
pub(super) fn hash_bool(hasher: &mut blake3::Hasher, value: bool) {
    tag(hasher, u8::from(value));
}
pub(super) fn hash_bytes(hasher: &mut blake3::Hasher, value: &[u8]) {
    hash_len(hasher, value.len());
    hasher.update(value);
}

pub(super) fn hash_text(hasher: &mut blake3::Hasher, value: &str) {
    hash_len(hasher, value.len());
    hasher.update(value.as_bytes());
}

pub(super) fn hash_optional_text(hasher: &mut blake3::Hasher, value: Option<&str>) {
    if let Some(value) = value {
        tag(hasher, 1);
        hash_text(hasher, value);
    } else {
        tag(hasher, 0);
    }
}

pub(super) fn hash_optional_u64(hasher: &mut blake3::Hasher, value: Option<u64>) {
    if let Some(value) = value {
        tag(hasher, 1);
        hash_u64(hasher, value);
    } else {
        tag(hasher, 0);
    }
}

pub(super) fn hash_value(hasher: &mut blake3::Hasher, value: &recite_core::Value) {
    match value {
        recite_core::Value::Scalar(value) => {
            tag(hasher, 0);
            hash_scalar(hasher, value);
        }
        recite_core::Value::Array(values) => {
            tag(hasher, 1);
            hash_len(hasher, values.len());
            for value in values {
                hash_scalar(hasher, value);
            }
        }
    }
}

fn hash_scalar(hasher: &mut blake3::Hasher, value: &recite_core::ScalarValue) {
    match value {
        recite_core::ScalarValue::String(value) => {
            tag(hasher, 0);
            hash_text(hasher, value);
        }
        recite_core::ScalarValue::Integer(value) => {
            tag(hasher, 1);
            hash_i64(hasher, *value);
        }
        recite_core::ScalarValue::Float(value) => {
            tag(hasher, 2);
            hash_u64(hasher, value.to_bits());
        }
        recite_core::ScalarValue::Boolean(value) => {
            tag(hasher, 3);
            hash_bool(hasher, *value);
        }
    }
}

pub(super) fn hash_optional_span(
    hasher: &mut blake3::Hasher,
    span: Option<&recite_core::SourceSpan>,
) {
    if let Some(span) = span {
        tag(hasher, 1);
        hash_span(hasher, span);
    } else {
        tag(hasher, 0);
    }
}

pub(super) fn hash_span(hasher: &mut blake3::Hasher, span: &recite_core::SourceSpan) {
    hash_text(hasher, &span.file);
    hash_u64(hasher, span.start.line() as u64);
    hash_u64(hasher, span.start.column() as u64);
    if let Some(end) = span.end {
        tag(hasher, 1);
        hash_u64(hasher, end.line() as u64);
        hash_u64(hasher, end.column() as u64);
    } else {
        tag(hasher, 0);
    }
}

pub(super) fn hash_expected_type(
    hasher: &mut blake3::Hasher,
    value: recite_runtime::ConditionExpectedType,
) {
    tag(
        hasher,
        match value {
            recite_runtime::ConditionExpectedType::Bool => 0,
            recite_runtime::ConditionExpectedType::Enum => 1,
        },
    );
}

pub(super) fn hash_schema_fingerprint(
    hasher: &mut blake3::Hasher,
    value: &recite_runtime::DialogueSchemaFingerprintSnapshot,
) {
    match value {
        recite_runtime::DialogueSchemaFingerprintSnapshot::Fingerprint(fingerprint) => {
            tag(hasher, 0);
            hash_text(hasher, &fingerprint.algorithm);
            hash_bytes(hasher, &fingerprint.digest);
        }
        recite_runtime::DialogueSchemaFingerprintSnapshot::NoSchema => tag(hasher, 1),
    }
}
