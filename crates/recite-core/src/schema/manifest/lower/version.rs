use super::super::raw::RawValue;
use super::super::spans::{TomlSpanIndex, top_level_number_token, top_level_toml_number_token};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SchemaVersion<'a> {
    One,
    Unsupported(&'a str),
    Malformed,
}

pub(super) fn schema_version<'a>(source: &'a str, value: &RawValue) -> SchemaVersion<'a> {
    if !matches!(value, RawValue::Number(_)) {
        return SchemaVersion::Malformed;
    }

    let Some(token) = top_level_number_token(source, "schema_version") else {
        return SchemaVersion::Malformed;
    };

    if number_token_equals_one(token) {
        SchemaVersion::One
    } else {
        SchemaVersion::Unsupported(token)
    }
}

pub(super) fn toml_schema_version<'a>(
    source: &'a str,
    value: &RawValue,
    spans: Option<&TomlSpanIndex>,
) -> SchemaVersion<'a> {
    if !matches!(value, RawValue::Number(_)) {
        return SchemaVersion::Malformed;
    }
    let Some(token) = top_level_toml_number_token(source, "schema_version", spans) else {
        return SchemaVersion::Malformed;
    };
    if number_token_equals_one(token) {
        SchemaVersion::One
    } else {
        SchemaVersion::Unsupported(token)
    }
}

fn number_token_equals_one(token: &str) -> bool {
    let Some((significand, exponent)) = split_decimal_exponent(token) else {
        return false;
    };
    if significand.starts_with('-') {
        return false;
    }

    let Some((integer, fraction)) = significand.split_once('.').or(Some((significand, ""))) else {
        return false;
    };
    let coefficient = format!("{integer}{fraction}");
    if coefficient.is_empty()
        || !coefficient
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        return false;
    }

    let coefficient = coefficient.trim_start_matches('0');
    if coefficient.is_empty() {
        return false;
    }

    let decimal_places = i64::try_from(fraction.len()).unwrap_or(i64::MAX);
    let scale = decimal_places - exponent;
    if scale < 0 {
        return false;
    };
    let Ok(scale) = usize::try_from(scale) else {
        return false;
    };

    let Some(expected_len) = scale.checked_add(1) else {
        return false;
    };
    let mut bytes = coefficient.bytes();
    coefficient.len() == expected_len
        && bytes.next() == Some(b'1')
        && bytes.all(|byte| byte == b'0')
}

fn split_decimal_exponent(token: &str) -> Option<(&str, i64)> {
    let Some(index) = token.find(['e', 'E']) else {
        return Some((token, 0));
    };
    let significand = &token[..index];
    let exponent = token[index + 1..].parse::<i64>().ok()?;
    Some((significand, exponent))
}
