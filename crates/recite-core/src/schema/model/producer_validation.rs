/// Return whether a producer extension key has the canonical namespace shape.
pub(crate) fn is_namespaced_extension_key(key: &str) -> bool {
    let mut segments = key.split(':');
    segments.next().is_some_and(is_extension_segment)
        && segments.next().is_some_and(is_extension_segment)
        && segments.next().is_none()
}

fn is_extension_segment(segment: &str) -> bool {
    let mut characters = segment.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    first.is_ascii_alphabetic()
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
}

/// Return whether a producer number is a TOML-preservable JSON number lexeme.
///
/// The source editor writes these lexemes into TOML and the source loader
/// restores them from the concrete syntax tree. Restricting the input to the
/// shared JSON number grammar and serde_json's canonical spelling avoids
/// TOML-only spellings and loader-induced variant changes.
pub(crate) fn is_json_number_lexeme(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0;
    let negative = bytes.first() == Some(&b'-');
    if negative {
        index += 1;
    }
    if bytes.get(index) == Some(&b'+') || index >= bytes.len() {
        return false;
    }

    let integer_start = index;
    if bytes[index] == b'0' {
        index += 1;
        if bytes.get(index).is_some_and(u8::is_ascii_digit) {
            return false;
        }
    } else if bytes[index].is_ascii_digit() {
        index += 1;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
    } else {
        return false;
    }

    let mut has_nonzero_digit = bytes[integer_start..index]
        .iter()
        .any(|digit| *digit != b'0');
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        let fraction_start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if fraction_start == index {
            return false;
        }
        has_nonzero_digit |= bytes[fraction_start..index]
            .iter()
            .any(|digit| *digit != b'0');
    }
    if bytes
        .get(index)
        .is_some_and(|character| matches!(character, b'e' | b'E'))
    {
        index += 1;
        if bytes
            .get(index)
            .is_some_and(|character| matches!(character, b'+' | b'-'))
        {
            index += 1;
        }
        let exponent_start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if exponent_start == index {
            return false;
        }
    }

    if index != bytes.len() || (negative && !has_nonzero_digit) {
        return false;
    }
    serde_json::Number::from_str(value).is_ok_and(|number| number.to_string() == value)
}
use std::str::FromStr;
