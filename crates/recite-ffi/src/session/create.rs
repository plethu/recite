use std::collections::BTreeMap;
use std::thread;

use crate::asset::{alloc_handle, lock_assets};
use crate::error::{ReciteStatus, set_last_error};
use crate::interpolation::{ReciteInterpolationValue, parse_interpolation_values};

use super::{FfiSession, parse_session_params};

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
    unsafe {
        super::recite_session_create_with_values(
            asset_handle,
            start_block,
            locale,
            std::ptr::null(),
            0,
            session_handle_out,
        )
    }
}

/// Creates a session handle without running traversal and stores typed
/// interpolation values for the session.
///
/// The input records are borrowed only for this call. Recite copies them into
/// session-owned storage before returning. Call
/// `recite_session_set_interpolation_values` to replace the values later.
///
/// # Safety
/// All non-null pointer arguments, including each record's string pointers,
/// must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_create_with_values(
    asset_handle: u64,
    start_block: *const std::ffi::c_char,
    locale: *const std::ffi::c_char,
    values: *const ReciteInterpolationValue,
    values_len: usize,
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

    let interpolation_values = match unsafe { parse_interpolation_values(values, values_len) } {
        Ok(values) => values,
        Err(error) => {
            set_last_error(&error);
            return ReciteStatus::Validation;
        }
    };

    let (block, options) = match unsafe { parse_session_params(start_block, locale) } {
        Ok(values) => values,
        Err((status, message)) => {
            set_last_error(&message);
            return status;
        }
    };

    let session =
        match recite_runtime::start_scene_with_options(&dialogue, block.as_deref(), options) {
            Ok(session) => session,
            Err(error) => {
                set_last_error(&error.to_string());
                return ReciteStatus::from(error);
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
            locale_provider: None,
            locale_variant: None,
            owner_thread: thread::current().id(),
            begun: false,
        },
    );
    unsafe { *session_handle_out = handle };
    ReciteStatus::Ok
}
