use crate::asset::{alloc_handle, lock_assets};
use crate::error::{ReciteStatus, set_last_error};

use super::{FfiSession, parse_session_params};
use recite_adapter::{LoadedDialogue, SessionDriver, StartRequest};

/// Creates a session handle without running any traversal.
///
/// Use this instead of `recite_session_start` when conditions appear in the
/// opening block of the scene: register handlers with
/// `recite_session_register_condition` after this call, then call
/// `recite_session_begin` to run the first traversal drain.
///
/// `start_block` and `locale` are nullable UTF-8 NUL-terminated strings.
/// On success writes a non-zero handle to `*session_handle_out`.
///
/// # Safety
/// All non-null pointer arguments must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_create(
    asset_handle: u64,
    start_block: *const std::ffi::c_char,
    locale: *const std::ffi::c_char,
    session_handle_out: *mut u64,
) -> ReciteStatus {
    if session_handle_out.is_null() {
        set_last_error("null pointer argument");
        return ReciteStatus::Validation;
    }

    let dialogue = {
        let guard = lock_assets();
        match guard.get(&asset_handle).cloned() {
            Some(dialogue) => dialogue,
            None => {
                set_last_error("unknown asset handle");
                return ReciteStatus::InvalidHandle;
            }
        }
    };

    let (block, options) = match unsafe { parse_session_params(start_block, locale) } {
        Ok(values) => values,
        Err((status, message)) => {
            set_last_error(&message);
            return status;
        }
    };

    let loaded = match LoadedDialogue::from_shared(dialogue) {
        Ok(loaded) => loaded,
        Err(error) => {
            set_last_error(&error.to_string());
            return ReciteStatus::from(error);
        }
    };
    let mut driver = SessionDriver::new();
    if let Err(error) = driver.prepare(StartRequest {
        asset: &loaded,
        block_id: block.as_deref(),
        options,
    }) {
        set_last_error(&error.to_string());
        return ReciteStatus::from(error);
    }

    let handle = alloc_handle();
    let mut guard = match super::lock_sessions() {
        Ok(guard) => guard,
        Err(status) => return status,
    };
    guard.insert(handle, FfiSession::prepared(driver));
    unsafe { *session_handle_out = handle };
    ReciteStatus::Ok
}
