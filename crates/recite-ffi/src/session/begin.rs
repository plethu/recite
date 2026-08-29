use crate::buffer::ReciteBuffer;
use crate::condition::{ConditionEntry, FfiContext, SendPtr};
use crate::error::{ReciteStatus, clear_condition_status, set_last_error};

use super::drain_to_batch;

/// Runs the initial traversal drain for a session created with
/// `recite_session_create`.
///
/// Must be called exactly once per session, after all condition handlers have
/// been registered with `recite_session_register_condition`. On success writes
/// the first output batch to `*batch_out`.
///
/// # Safety
/// `batch_out` must be a valid non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_begin(
    session_handle: u64,
    batch_out: *mut ReciteBuffer,
) -> ReciteStatus {
    if batch_out.is_null() {
        set_last_error("null pointer argument");
        return ReciteStatus::Validation;
    }

    let mut guard = super::lock_sessions();
    let ffi_session = match guard.get_mut(&session_handle) {
        Some(session) => session,
        None => {
            set_last_error("unknown session handle");
            return ReciteStatus::InvalidHandle;
        }
    };
    if let Err(status) = super::ensure_session_thread(ffi_session) {
        return status;
    }

    if ffi_session.begun {
        set_last_error("recite_session_begin called twice on the same handle");
        return ReciteStatus::SessionAlreadyActive;
    }
    let context = FfiContext {
        handlers: &ffi_session.handlers,
    };
    let session_checkpoint = ffi_session.session.clone();
    clear_condition_status();
    match drain_to_batch(
        &ffi_session.dialogue,
        &mut ffi_session.session,
        &context,
        &ffi_session.interpolation_values,
        ffi_session.locale_provider.as_ref(),
        ffi_session.locale_variant.as_deref(),
    ) {
        Ok(batch) => {
            ffi_session.begun = true;
            unsafe { *batch_out = batch };
            ReciteStatus::Ok
        }
        Err((status, message)) => {
            ffi_session.session = session_checkpoint;
            set_last_error(&message);
            status
        }
    }
}

/// Registers a condition handler on an existing session handle.
///
/// For conditions that appear in the opening block of a scene, call this
/// after `recite_session_create` and before `recite_session_begin`. For
/// conditions that only appear in later blocks (e.g. after a choice), it is
/// also safe to register after `recite_session_start` returns.
///
/// `name` is a UTF-8 NUL-terminated condition function name. `userdata` is
/// passed back to the handler on each invocation.
///
/// # Safety
/// `name` must be valid NUL-terminated UTF-8 for the duration of the call.
/// `handler` must be a valid non-null callback function pointer. Passing NULL
/// returns `RECITE_STATUS_VALIDATION` before the session or handler table is
/// accessed. `userdata` must remain valid and accessible from the calling
/// thread for the session's lifetime.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_register_condition(
    session_handle: u64,
    name: *const std::ffi::c_char,
    handler: Option<
        unsafe extern "C" fn(
            *const crate::condition::ReciteConditionQuery,
            *mut std::ffi::c_void,
        ) -> crate::condition::ReciteConditionResult,
    >,
    userdata: *mut std::ffi::c_void,
) -> ReciteStatus {
    if name.is_null() {
        set_last_error("null name argument");
        return ReciteStatus::Validation;
    }
    let Some(handler) = handler else {
        set_last_error("null condition handler argument");
        return ReciteStatus::Validation;
    };
    let name_str = match unsafe { std::ffi::CStr::from_ptr(name) }.to_str() {
        Ok(name) => name.to_owned(),
        Err(_) => {
            set_last_error("name is not valid UTF-8");
            return ReciteStatus::Validation;
        }
    };
    let mut guard = super::lock_sessions();
    match guard.get_mut(&session_handle) {
        Some(ffi_session) => {
            if let Err(status) = super::ensure_session_thread(ffi_session) {
                return status;
            }
            ffi_session.handlers.insert(
                name_str,
                ConditionEntry {
                    handler,
                    userdata: SendPtr(userdata),
                },
            );
            ReciteStatus::Ok
        }
        None => {
            set_last_error("unknown session handle");
            ReciteStatus::InvalidHandle
        }
    }
}
