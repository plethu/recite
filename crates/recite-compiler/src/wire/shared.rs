use recite_core::{ScalarValue, TableRange, Value};

pub(in crate::wire) fn range_to_u32<I: Copy>(
    range: TableRange<I>,
    index: impl Fn(I) -> u32,
) -> (u32, u32) {
    (index(range.start), range.len)
}

pub(in crate::wire) fn scalar_value_tag(value: &ScalarValue) -> u8 {
    match value {
        ScalarValue::String(_) => recite_core::V0_SCALAR_TAG_STRING,
        ScalarValue::Integer(_) => recite_core::V0_SCALAR_TAG_INTEGER,
        ScalarValue::Float(_) => recite_core::V0_SCALAR_TAG_FLOAT,
        ScalarValue::Boolean(_) => recite_core::V0_SCALAR_TAG_BOOLEAN,
    }
}

pub(in crate::wire) fn value_tag(value: &Value) -> u8 {
    match value {
        Value::Scalar(_) => recite_core::V0_VALUE_TAG_SCALAR,
        Value::Array(_) => recite_core::V0_VALUE_TAG_ARRAY,
    }
}

pub(in crate::wire) fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}
