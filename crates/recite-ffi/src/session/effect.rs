use crate::buffer::ReciteBuffer;
use crate::condition::FfiContext;
use crate::error::{ReciteStatus, clear_condition_status, set_last_error};

use recite_runtime::{EffectAck, acknowledge_effect};

use super::drain_to_batch;

/// Acknowledges the currently pending blocking effect.
///
/// `effect_request_id` is a UTF-8 NUL-terminated string matching the effect ID
/// emitted in the output batch. `ack_completed` is 1 for `EffectAck::Completed`
/// or 0 for `EffectAck::Failed`. `failure_reason` is a nullable UTF-8
/// NUL-terminated string used when `ack_completed == 0`.
///
/// # Safety
/// All non-null pointer arguments must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_acknowledge_effect(
    session_handle: u64,
    effect_request_id: *const std::ffi::c_char,
    ack_completed: u8,
    failure_reason: *const std::ffi::c_char,
    batch_out: *mut ReciteBuffer,
) -> ReciteStatus {
    if effect_request_id.is_null() || batch_out.is_null() {
        set_last_error("null pointer argument");
        return ReciteStatus::Validation;
    }
    let id_str = match unsafe { std::ffi::CStr::from_ptr(effect_request_id) }.to_str() {
        Ok(id) => id,
        Err(_) => {
            set_last_error("effect_request_id is not valid UTF-8");
            return ReciteStatus::Validation;
        }
    };
    let effect_id = match recite_core::EffectId::new(id_str) {
        Ok(effect_id) => effect_id,
        Err(error) => {
            set_last_error(&error.to_string());
            return ReciteStatus::EffectAcknowledgement;
        }
    };

    let ack = if ack_completed != 0 {
        EffectAck::Completed
    } else {
        let reason = if failure_reason.is_null() {
            String::new()
        } else {
            match unsafe { std::ffi::CStr::from_ptr(failure_reason) }.to_str() {
                Ok(reason) => reason.to_owned(),
                Err(_) => {
                    set_last_error("failure_reason is not valid UTF-8");
                    return ReciteStatus::Validation;
                }
            }
        };
        EffectAck::Failed { reason }
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
    let result = match acknowledge_effect(&mut ffi_session.session, effect_id, ack) {
        Ok(()) => drain_to_batch(
            &ffi_session.dialogue,
            &mut ffi_session.session,
            &context,
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
