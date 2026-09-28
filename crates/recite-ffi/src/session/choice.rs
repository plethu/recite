use crate::buffer::ReciteBuffer;
use crate::condition::FfiContext;
use crate::error::{ReciteStatus, set_last_error};
use crate::output::{encode_batch, encode_batch_output};

use super::{driver_failure, locale_resolution};

/// Selects a pending prompt choice.
///
/// `choice_id` is a UTF-8 NUL-terminated string. On success writes the
/// subsequent output batch to `*batch_out`.
///
/// # Safety
/// All non-null pointer arguments must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_choose(
    session_handle: u64,
    choice_id: *const std::ffi::c_char,
    batch_out: *mut ReciteBuffer,
) -> ReciteStatus {
    if choice_id.is_null() || batch_out.is_null() {
        set_last_error("null pointer argument");
        return ReciteStatus::Validation;
    }
    let choice_str = match unsafe { std::ffi::CStr::from_ptr(choice_id) }.to_str() {
        Ok(choice) => choice,
        Err(_) => {
            set_last_error("choice_id is not valid UTF-8");
            return ReciteStatus::Validation;
        }
    };
    let choice_id = match recite_core::ChoiceId::new(choice_str) {
        Ok(choice_id) => choice_id,
        Err(error) => {
            set_last_error(&error.to_string());
            return ReciteStatus::InvalidChoice;
        }
    };

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

    let context = FfiContext {
        handlers: &ffi_session.handlers,
    };
    let resolution = locale_resolution(
        &ffi_session.interpolation_values,
        ffi_session.locale_source.provider(),
        ffi_session.locale_variant.as_deref(),
    );
    match ffi_session
        .driver
        .select_choice_with(choice_id, &context, resolution, |events| {
            encode_batch_output(events, encode_batch)
        }) {
        Ok(batch) => {
            unsafe { *batch_out = batch };
            ReciteStatus::Ok
        }
        Err(error) => {
            let (status, message) = driver_failure(error);
            set_last_error(&message);
            status
        }
    }
}
