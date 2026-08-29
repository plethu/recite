use std::collections::BTreeMap;
use std::thread;

use crate::asset::{alloc_handle, lock_assets};
use crate::buffer::ReciteBuffer;
use crate::condition::FfiContext;
use crate::error::{ReciteStatus, clear_condition_status, restore_status, set_last_error};
use crate::interpolation::{ReciteInterpolationValue, parse_interpolation_values};
use crate::locale::FfiLocaleProvider;

use super::FfiSession;
use super::drain_restored;

struct RestoreRequest {
    asset_handle: u64,
    snapshot_bytes: *const u8,
    snapshot_len: usize,
    values: *const ReciteInterpolationValue,
    values_len: usize,
    locale_provider: Option<FfiLocaleProvider>,
    locale_variant: Option<String>,
}

struct RestoreOutputs {
    session_handle_out: *mut u64,
    batch_out: *mut ReciteBuffer,
}

/// Restores a session from a snapshot previously produced by
/// `recite_session_snapshot`.
///
/// The snapshot must have been produced against the same compiled asset
/// identified by `asset_handle`. On success writes a new session handle to
/// `*session_handle_out` and a resumption output batch to `*batch_out`. The
/// batch is empty when the restored session is at a pending-prompt boundary.
/// A pending blocking effect is re-emitted once in the resumption batch with
/// the same request ID so the host can reconcile or re-present it.
/// If the snapshot encoded an ended session, `recite_session_restore` returns
/// `RECITE_ERR_NO_ACTIVE_SESSION`.
///
/// # Safety
/// All non-null pointer arguments must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_restore(
    asset_handle: u64,
    snapshot_bytes: *const u8,
    snapshot_len: usize,
    session_handle_out: *mut u64,
    batch_out: *mut ReciteBuffer,
) -> ReciteStatus {
    unsafe {
        super::recite_session_restore_with_values(
            asset_handle,
            snapshot_bytes,
            snapshot_len,
            std::ptr::null(),
            0,
            session_handle_out,
            batch_out,
        )
    }
}

/// Restores a session and supplies typed interpolation values for its first
/// resumption drain.
///
/// Input records are borrowed only for this call and copied into the restored
/// session. Use `recite_session_set_interpolation_values` to replace them for a
/// later traversal operation.
///
/// # Safety
/// All non-null pointer arguments, including each record's string pointers,
/// must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_restore_with_values(
    asset_handle: u64,
    snapshot_bytes: *const u8,
    snapshot_len: usize,
    values: *const ReciteInterpolationValue,
    values_len: usize,
    session_handle_out: *mut u64,
    batch_out: *mut ReciteBuffer,
) -> ReciteStatus {
    unsafe {
        restore_impl(
            RestoreRequest {
                asset_handle,
                snapshot_bytes,
                snapshot_len,
                values,
                values_len,
                locale_provider: None,
                locale_variant: None,
            },
            RestoreOutputs {
                session_handle_out,
                batch_out,
            },
        )
    }
}

/// Restores a session and supplies both interpolation values and a typed
/// locale callback before the first resumption drain.
///
/// The callback is copied into the new session. Its complete result pointer
/// tree must remain immutable and valid until this restore call returns;
/// Recite copies it before returning the resumption batch.
///
/// # Safety
/// All non-null pointers must be valid for the duration of the call. The
/// callback must be a valid non-null function pointer, and `userdata` must
/// remain valid for the restored session lifetime. Passing NULL as `callback`
/// returns `RECITE_STATUS_VALIDATION` before a session is created.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_restore_with_values_and_locale_provider(
    asset_handle: u64,
    snapshot_bytes: *const u8,
    snapshot_len: usize,
    values: *const ReciteInterpolationValue,
    values_len: usize,
    callback: Option<
        unsafe extern "C" fn(
            *const crate::locale::ReciteLocaleQuery,
            *mut std::ffi::c_void,
        ) -> crate::locale::ReciteLocaleResult,
    >,
    userdata: *mut std::ffi::c_void,
    session_handle_out: *mut u64,
    batch_out: *mut ReciteBuffer,
) -> ReciteStatus {
    let Some(callback) = callback else {
        set_last_error("locale callback is null");
        return ReciteStatus::Validation;
    };
    unsafe {
        restore_impl(
            RestoreRequest {
                asset_handle,
                snapshot_bytes,
                snapshot_len,
                values,
                values_len,
                locale_provider: Some(FfiLocaleProvider::new(callback, userdata)),
                locale_variant: None,
            },
            RestoreOutputs {
                session_handle_out,
                batch_out,
            },
        )
    }
}

/// Restores a session with interpolation values, a locale callback, and an
/// explicit grammatical variant before the first resumption drain.
///
/// The variant is copied into the restored session and is not part of the
/// serialized snapshot. Callers must supply it again whenever restoring a
/// snapshot that needs a variant-specific catalog entry.
///
/// # Safety
/// All non-null pointers must be valid for the duration of the call. The
/// callback must be a valid non-null function pointer, and `userdata` must
/// remain valid for the restored session lifetime. Passing NULL as `callback`
/// returns `RECITE_STATUS_VALIDATION` before a session is created.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_restore_with_values_and_locale_provider_and_variant(
    asset_handle: u64,
    snapshot_bytes: *const u8,
    snapshot_len: usize,
    values: *const ReciteInterpolationValue,
    values_len: usize,
    locale_variant: *const std::ffi::c_char,
    callback: Option<
        unsafe extern "C" fn(
            *const crate::locale::ReciteLocaleQuery,
            *mut std::ffi::c_void,
        ) -> crate::locale::ReciteLocaleResult,
    >,
    userdata: *mut std::ffi::c_void,
    session_handle_out: *mut u64,
    batch_out: *mut ReciteBuffer,
) -> ReciteStatus {
    let Some(callback) = callback else {
        set_last_error("locale callback is null");
        return ReciteStatus::Validation;
    };
    let locale_variant =
        match unsafe { super::parse_optional_session_string(locale_variant, "locale variant") } {
            Ok(locale_variant) => locale_variant,
            Err(status) => return status,
        };
    unsafe {
        restore_impl(
            RestoreRequest {
                asset_handle,
                snapshot_bytes,
                snapshot_len,
                values,
                values_len,
                locale_provider: Some(FfiLocaleProvider::new(callback, userdata)),
                locale_variant,
            },
            RestoreOutputs {
                session_handle_out,
                batch_out,
            },
        )
    }
}

unsafe fn restore_impl(request: RestoreRequest, outputs: RestoreOutputs) -> ReciteStatus {
    if request.snapshot_bytes.is_null()
        || outputs.session_handle_out.is_null()
        || outputs.batch_out.is_null()
    {
        set_last_error("null pointer argument");
        return ReciteStatus::Validation;
    }
    let dialogue = {
        let guard = lock_assets();
        match guard.get(&request.asset_handle).cloned() {
            Some(dialogue) => dialogue,
            None => {
                set_last_error("unknown asset handle");
                return ReciteStatus::InvalidHandle;
            }
        }
    };
    let bytes = unsafe { std::slice::from_raw_parts(request.snapshot_bytes, request.snapshot_len) };
    let mut session = match recite_runtime::decode_session_messagepack(&dialogue, bytes) {
        Ok(session) => session,
        Err(error) => {
            set_last_error(&error.to_string());
            return restore_status(&error);
        }
    };

    let interpolation_values =
        match unsafe { parse_interpolation_values(request.values, request.values_len) } {
            Ok(values) => values,
            Err(error) => {
                set_last_error(&error);
                return ReciteStatus::Validation;
            }
        };
    let context = FfiContext {
        handlers: &BTreeMap::new(),
    };
    clear_condition_status();
    let batch = match drain_restored(
        &dialogue,
        &mut session,
        &context,
        &interpolation_values,
        request.locale_provider.as_ref(),
        request.locale_variant.as_deref(),
    ) {
        Ok(batch) => batch,
        Err((status, message)) => {
            set_last_error(&message);
            return status;
        }
    };

    let handle = alloc_handle();
    super::lock_sessions().insert(
        handle,
        FfiSession {
            dialogue,
            session,
            handlers: BTreeMap::new(),
            interpolation_values,
            locale_provider: request.locale_provider,
            locale_variant: request.locale_variant,
            owner_thread: thread::current().id(),
            begun: true,
        },
    );
    unsafe { *outputs.session_handle_out = handle };
    unsafe { *outputs.batch_out = batch };
    ReciteStatus::Ok
}
