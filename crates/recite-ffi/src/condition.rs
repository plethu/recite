use std::collections::BTreeMap;
use std::ffi::{CStr, c_char, c_void};

use recite_runtime::{ConditionEvaluationError, ConditionQuery, ConditionValue, DialogueContext};

use crate::condition_codec::{decode_condition_value, encode_condition_args};
use crate::error::{ReciteStatus, set_condition_status};

/// Query passed to a `ReciteConditionFn` callback.
///
/// All pointers are borrowed for the duration of the callback and must not be
/// stored by the host.
#[repr(C)]
pub struct ReciteConditionQuery {
    /// Recite-owned UTF-8 NUL-terminated condition function name. Borrowed by
    /// the host only for the callback call.
    pub function_name: *const c_char,
    /// Recite-owned msgpack-encoded array of `FfiConditionArg` values. Borrowed
    /// by the host only for the callback call.
    pub args_msgpack: *const u8,
    pub args_len: usize,
}

/// Result returned by a `ReciteConditionFn` callback.
///
/// `ok` must be exactly 0 or 1. When `ok == 1`, `value_msgpack` must point to a
/// complete msgpack-encoded `FfiConditionValue` valid for the duration of the
/// callback frame. When `ok == 0`, `error_message` may be null (the runtime
/// uses a stable fallback) or point to a UTF-8 NUL-terminated string valid for
/// the duration of the callback frame.
#[repr(C)]
pub struct ReciteConditionResult {
    /// Exactly 1 = success, 0 = failure.
    pub ok: u8,
    /// Host-owned msgpack bytes encoding a `FfiConditionValue`. Borrowed by
    /// Recite only until callback return; valid when `ok == 1`.
    pub value_msgpack: *const u8,
    pub value_len: usize,
    /// Host-owned UTF-8 NUL-terminated error message. Borrowed by Recite only
    /// until callback return; valid when `ok == 0`.
    pub error_message: *const c_char,
}

/// Host-provided condition handler function pointer.
///
/// Invoked synchronously on the same thread as the `recite_session_*` call
/// that triggers condition evaluation. Must not call back into `recite-ffi`.
pub type ReciteConditionFn = unsafe extern "C" fn(
    query: *const ReciteConditionQuery,
    userdata: *mut c_void,
) -> ReciteConditionResult;

/// Wraps `*mut c_void` so it can be stored in a `Send`-able map.
///
/// # Safety
/// `FfiSession` records its owner thread and rejects session operations from
/// other threads before callbacks can observe the pointer.
pub(crate) struct SendPtr(pub *mut c_void);
// SAFETY: condition callbacks only run after the owning session verifies that
// the current thread matches the thread that created or restored the session.
unsafe impl Send for SendPtr {}

pub(crate) struct ConditionEntry {
    pub handler: ReciteConditionFn,
    pub userdata: SendPtr,
}

/// Implements `DialogueContext` via registered `ReciteConditionFn` callbacks.
pub(crate) struct FfiContext<'a> {
    pub handlers: &'a BTreeMap<String, ConditionEntry>,
}

impl DialogueContext for FfiContext<'_> {
    fn evaluate_condition(
        &self,
        query: ConditionQuery<'_>,
    ) -> Result<ConditionValue, ConditionEvaluationError> {
        let Some(entry) = self.handlers.get(query.function()) else {
            return Err(condition_error(
                ReciteStatus::MissingConditionHandler,
                format!("no handler registered for `{}`", query.function()),
            ));
        };

        let args_bytes = encode_condition_args(query).map_err(|error| {
            condition_error(
                ReciteStatus::ConditionEvaluation,
                format!("failed to encode condition args: {error}"),
            )
        })?;

        // Build the NUL-terminated function name.
        let function_cstring = std::ffi::CString::new(query.function().replace('\0', "?"))
            .map_err(|_| {
                condition_error(
                    ReciteStatus::ConditionEvaluation,
                    "condition function name contains NUL",
                )
            })?;

        let c_query = ReciteConditionQuery {
            function_name: function_cstring.as_ptr(),
            args_msgpack: args_bytes.as_ptr(),
            args_len: args_bytes.len(),
        };

        // Call the host handler.
        // SAFETY: The host registered a valid function pointer. Lifetimes of
        // c_query fields are valid for this call's stack frame.
        let result = unsafe { (entry.handler)(&c_query, entry.userdata.0) };

        match result.ok {
            1 => {
                if result.value_msgpack.is_null()
                    || result.value_len == 0
                    || result.value_len > isize::MAX as usize
                {
                    return Err(condition_error(
                        ReciteStatus::InvalidConditionResult,
                        "condition handler returned an invalid value payload",
                    ));
                }
                // SAFETY: The host guarantees the bytes are valid for this call.
                let bytes =
                    unsafe { std::slice::from_raw_parts(result.value_msgpack, result.value_len) };
                let ffi_value = decode_condition_value(bytes).map_err(|error| {
                    condition_error(
                        ReciteStatus::InvalidConditionResult,
                        format!("failed to decode condition result: {error}"),
                    )
                })?;
                Ok(match ffi_value {
                    crate::condition_codec::FfiConditionValue::Bool { value } => {
                        ConditionValue::Bool(value)
                    }
                    crate::condition_codec::FfiConditionValue::Enum { variant } => {
                        ConditionValue::EnumVariant(variant)
                    }
                })
            }
            0 => {
                let msg = if result.error_message.is_null() {
                    "condition handler failed with no message".to_owned()
                } else {
                    // SAFETY: The host guarantees the string is valid NUL-terminated UTF-8.
                    unsafe { CStr::from_ptr(result.error_message) }
                        .to_string_lossy()
                        .into_owned()
                };
                Err(condition_error(ReciteStatus::ConditionEvaluation, msg))
            }
            _ => Err(condition_error(
                ReciteStatus::InvalidConditionResult,
                format!("condition handler returned invalid ok value {}", result.ok),
            )),
        }
    }
}

fn condition_error(status: ReciteStatus, message: impl Into<String>) -> ConditionEvaluationError {
    set_condition_status(status);
    ConditionEvaluationError::new(message)
}
