use std::collections::BTreeMap;
use std::ffi::{CStr, c_char};

use recite_core::ScalarValue;
use recite_runtime::InterpolationValues;

/// Scalar type carried by one caller-provided interpolation value.
///
/// The numeric discriminants are part of the C ABI. The associated payload is
/// selected by [`ReciteInterpolationValue::kind`].
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReciteInterpolationValueKind {
    String = 0,
    Integer = 1,
    Float = 2,
    Boolean = 3,
}

/// One caller-provided typed interpolation value.
///
/// The record and any string it points to are borrowed only for the duration
/// of the `recite_session_*_with_values` or
/// `recite_session_set_interpolation_values` call. Recite copies every value
/// into its session-owned [`InterpolationValues`] map before returning, so a
/// host may release or reuse the input records afterwards.
#[repr(C)]
pub struct ReciteInterpolationValue {
    /// UTF-8 NUL-terminated binding name, such as `player_name`.
    pub name: *const c_char,
    /// Selects which payload field is read. Use the numeric discriminants from
    /// [`ReciteInterpolationValueKind`]; unknown values are rejected.
    pub kind: u32,
    /// UTF-8 NUL-terminated string payload; used for
    /// [`ReciteInterpolationValueKind::String`].
    pub string_value: *const c_char,
    /// Integer payload; used for [`ReciteInterpolationValueKind::Integer`].
    pub integer_value: i64,
    /// Finite floating-point payload; used for [`ReciteInterpolationValueKind::Float`].
    pub float_value: f64,
    /// Boolean payload, which must be exactly `0` or `1`; used for
    /// [`ReciteInterpolationValueKind::Boolean`].
    pub boolean_value: u8,
}

/// Copies a caller-owned C array into the canonical runtime interpolation map.
///
/// # Safety
/// `values` must be null when `values_len` is zero, or valid for `values_len`
/// consecutive [`ReciteInterpolationValue`] records. Every non-null string
/// pointer in those records must point to a valid NUL-terminated UTF-8 string
/// for the duration of this call.
pub(crate) unsafe fn parse_interpolation_values(
    values: *const ReciteInterpolationValue,
    values_len: usize,
) -> Result<InterpolationValues, String> {
    if values_len == 0 {
        return Ok(BTreeMap::new());
    }
    if values.is_null() {
        return Err("interpolation values pointer is null".to_owned());
    }
    if values_len > isize::MAX as usize / std::mem::size_of::<ReciteInterpolationValue>() {
        return Err("interpolation values length is too large".to_owned());
    }

    let records = unsafe { std::slice::from_raw_parts(values, values_len) };
    let mut parsed = BTreeMap::new();
    for record in records {
        if record.name.is_null() {
            return Err("interpolation value name is null".to_owned());
        }
        let name = unsafe { CStr::from_ptr(record.name) }
            .to_str()
            .map_err(|_| "interpolation value name is not valid UTF-8".to_owned())?
            .to_owned();
        if name.is_empty() {
            return Err("interpolation value name is empty".to_owned());
        }
        if parsed.contains_key(&name) {
            return Err(format!("duplicate interpolation value `{name}`"));
        }

        let value = match record.kind {
            kind if kind == ReciteInterpolationValueKind::String as u32 => {
                if record.string_value.is_null() {
                    return Err(format!("interpolation value `{name}` string is null"));
                }
                let value = unsafe { CStr::from_ptr(record.string_value) }
                    .to_str()
                    .map_err(|_| {
                        format!("interpolation value `{name}` string is not valid UTF-8")
                    })?;
                ScalarValue::String(value.to_owned())
            }
            kind if kind == ReciteInterpolationValueKind::Integer as u32 => {
                ScalarValue::Integer(record.integer_value)
            }
            kind if kind == ReciteInterpolationValueKind::Float as u32 => {
                if !record.float_value.is_finite() {
                    return Err(format!("interpolation value `{name}` float is not finite"));
                }
                ScalarValue::Float(record.float_value)
            }
            kind if kind == ReciteInterpolationValueKind::Boolean as u32 => {
                match record.boolean_value {
                    0 => ScalarValue::Boolean(false),
                    1 => ScalarValue::Boolean(true),
                    _ => {
                        return Err(format!(
                            "interpolation value `{name}` boolean must be 0 or 1"
                        ));
                    }
                }
            }
            _ => {
                return Err(format!(
                    "interpolation value `{name}` has an unknown kind {}",
                    record.kind
                ));
            }
        };
        parsed.insert(name, value);
    }
    Ok(parsed)
}
