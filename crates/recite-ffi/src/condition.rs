use std::collections::BTreeMap;
use std::ffi::{CStr, c_char, c_void};

use recite_runtime::{ConditionEvaluationError, ConditionQuery, ConditionValue, DialogueContext};
use serde::{Deserialize, Serialize};

use crate::error::{ReciteStatus, encode_condition_status};

/// Msgpack-encoded condition argument passed to a host condition handler.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum FfiConditionArg {
    Identifier { value: String },
    String { value: String },
    Integer { value: i64 },
    Float { value: f64 },
    Boolean { value: bool },
}

/// Msgpack-encoded condition result value returned by a host condition handler.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum FfiConditionValue {
    Bool { value: bool },
    Enum { variant: String },
}

/// Query passed to a `ReciteConditionFn` callback.
///
/// All pointers are borrowed for the duration of the callback and must not be
/// stored by the host.
#[repr(C)]
pub struct ReciteConditionQuery {
    /// UTF-8 NUL-terminated condition function name. Borrowed for the call.
    pub function_name: *const c_char,
    /// Msgpack-encoded array of `FfiConditionArg` values. Borrowed for the call.
    pub args_msgpack: *const u8,
    pub args_len: usize,
}

/// Result returned by a `ReciteConditionFn` callback.
///
/// When `ok != 0`, `value_msgpack` must point to a msgpack-encoded
/// `FfiConditionValue` valid for the duration of the callback frame.
/// When `ok == 0`, `error_message` must point to a UTF-8 NUL-terminated string
/// valid for the duration of the callback frame.
#[repr(C)]
pub struct ReciteConditionResult {
    /// 1 = success, 0 = failure.
    pub ok: u8,
    /// Host-owned msgpack bytes encoding a `FfiConditionValue`. Valid when `ok != 0`.
    pub value_msgpack: *const u8,
    pub value_len: usize,
    /// Host-owned UTF-8 NUL-terminated error message. Valid when `ok == 0`.
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
/// The host guarantees single-threaded access per session handle, and that
/// the userdata pointer remains valid for the session's lifetime.
pub(crate) struct SendPtr(pub *mut c_void);
// SAFETY: recite-ffi documents single-threaded access per session handle.
// Condition callbacks are invoked on the calling thread only.
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
            let msg = encode_condition_status(
                ReciteStatus::MissingConditionHandler,
                &format!("no handler registered for `{}`", query.function()),
            );
            return Err(ConditionEvaluationError::new(msg));
        };

        // Encode arguments as msgpack.
        let args: Vec<FfiConditionArg> = query
            .arguments()
            .iter()
            .map(|arg| match arg {
                recite_runtime::ConditionArgument::Identifier(v) => FfiConditionArg::Identifier {
                    value: v.to_owned(),
                },
                recite_runtime::ConditionArgument::String(v) => FfiConditionArg::String {
                    value: v.to_owned(),
                },
                recite_runtime::ConditionArgument::Integer(v) => {
                    FfiConditionArg::Integer { value: v }
                }
                recite_runtime::ConditionArgument::Float(v) => FfiConditionArg::Float { value: v },
                recite_runtime::ConditionArgument::Boolean(v) => {
                    FfiConditionArg::Boolean { value: v }
                }
            })
            .collect();

        let args_bytes = rmp_serde::to_vec_named(&args).map_err(|e| {
            ConditionEvaluationError::new(encode_condition_status(
                ReciteStatus::ConditionEvaluation,
                &format!("failed to encode condition args: {e}"),
            ))
        })?;

        // Build the NUL-terminated function name.
        let function_cstring = std::ffi::CString::new(query.function().replace('\0', "?"))
            .map_err(|_| {
                ConditionEvaluationError::new(encode_condition_status(
                    ReciteStatus::ConditionEvaluation,
                    "condition function name contains NUL",
                ))
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

        if result.ok != 0 {
            if result.value_msgpack.is_null() {
                return Err(ConditionEvaluationError::new(encode_condition_status(
                    ReciteStatus::ConditionEvaluation,
                    "condition handler returned ok but null value pointer",
                )));
            }
            // SAFETY: The host guarantees the bytes are valid for this call.
            let bytes =
                unsafe { std::slice::from_raw_parts(result.value_msgpack, result.value_len) };
            let ffi_value: FfiConditionValue = rmp_serde::from_slice(bytes).map_err(|e| {
                ConditionEvaluationError::new(encode_condition_status(
                    ReciteStatus::InvalidConditionResult,
                    &format!("failed to decode condition result: {e}"),
                ))
            })?;
            Ok(match ffi_value {
                FfiConditionValue::Bool { value } => ConditionValue::Bool(value),
                FfiConditionValue::Enum { variant } => ConditionValue::EnumVariant(variant),
            })
        } else {
            let msg = if result.error_message.is_null() {
                "condition handler failed with no message".to_owned()
            } else {
                // SAFETY: The host guarantees the string is valid NUL-terminated UTF-8.
                unsafe { CStr::from_ptr(result.error_message) }
                    .to_string_lossy()
                    .into_owned()
            };
            Err(ConditionEvaluationError::new(encode_condition_status(
                ReciteStatus::ConditionEvaluation,
                &msg,
            )))
        }
    }
}
