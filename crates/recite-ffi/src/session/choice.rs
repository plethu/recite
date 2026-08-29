use crate::buffer::ReciteBuffer;
use crate::condition::FfiContext;
use crate::error::{ReciteStatus, clear_condition_status, set_last_error};

use recite_runtime::{LocaleResolution, choose_with};

use super::drain_after_event;

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
    let session_checkpoint = ffi_session.session.clone();
    clear_condition_status();
    let resolution = LocaleResolution::new().with_values(&ffi_session.interpolation_values);
    let resolution = ffi_session
        .locale_provider
        .as_ref()
        .map_or(resolution, |provider| resolution.with_provider(provider));
    let resolution = ffi_session
        .locale_variant
        .as_deref()
        .map_or(resolution, |variant| resolution.with_variant(variant));
    let result = match choose_with(
        &ffi_session.dialogue,
        &mut ffi_session.session,
        choice_id,
        &context,
        resolution,
    ) {
        Ok(first_event) => drain_after_event(
            &ffi_session.dialogue,
            &mut ffi_session.session,
            &context,
            first_event,
            &ffi_session.interpolation_values,
            ffi_session.locale_provider.as_ref(),
            ffi_session.locale_variant.as_deref(),
        ),
        Err(error) => {
            set_last_error(&error.to_string());
            Err((ReciteStatus::from(error), String::new()))
        }
    };

    match result {
        Ok(batch) => {
            unsafe { *batch_out = batch };
            ReciteStatus::Ok
        }
        Err((status, message)) => {
            ffi_session.session = session_checkpoint;
            if !message.is_empty() {
                set_last_error(&message);
            }
            status
        }
    }
}
