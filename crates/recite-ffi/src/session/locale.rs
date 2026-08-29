use std::ffi::c_void;

use crate::error::{ReciteStatus, set_last_error};
use crate::locale::FfiLocaleProvider;

/// Installs the typed host locale callback used by subsequent traversal.
///
/// The callback and userdata are copied into the session. The callback is
/// invoked synchronously on the session owner thread and must not panic,
/// unwind, re-enter Recite, or throw across the ABI boundary. Passing a
/// callback does not change the session's explicit locale; a null locale
/// remains source-text-only and bypasses the callback. Null callbacks are
/// rejected before they are stored.
///
/// # Safety
/// `callback` must remain a valid non-null function pointer and `userdata` must
/// remain valid and accessible on the session owner thread for the session
/// lifetime.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_set_locale_provider(
    session_handle: u64,
    callback: Option<
        unsafe extern "C" fn(
            *const crate::locale::ReciteLocaleQuery,
            *mut c_void,
        ) -> crate::locale::ReciteLocaleResult,
    >,
    userdata: *mut c_void,
) -> ReciteStatus {
    let Some(callback) = callback else {
        set_last_error("locale callback is null");
        return ReciteStatus::Validation;
    };
    let mut guard = super::lock_sessions();
    let Some(session) = guard.get_mut(&session_handle) else {
        set_last_error("unknown session handle");
        return ReciteStatus::InvalidHandle;
    };
    if let Err(status) = super::ensure_session_thread(session) {
        return status;
    }
    session.locale_provider = Some(FfiLocaleProvider::new(callback, userdata));
    ReciteStatus::Ok
}

/// Removes the locale callback from a session. A session with an explicit
/// locale then uses the runtime's source-text fallback.
///
/// # Safety
/// The session handle must be used only from the thread that created it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_clear_locale_provider(session_handle: u64) -> ReciteStatus {
    let mut guard = super::lock_sessions();
    let Some(session) = guard.get_mut(&session_handle) else {
        set_last_error("unknown session handle");
        return ReciteStatus::InvalidHandle;
    };
    if let Err(status) = super::ensure_session_thread(session) {
        return status;
    }
    session.locale_provider = None;
    ReciteStatus::Ok
}

/// Sets or clears the explicit grammatical variant used by subsequent locale
/// lookups. The value is copied into the session and is not serialized; a
/// restored session must receive it again before its first resumption drain.
///
/// # Safety
/// `variant`, when non-null, must point to a valid NUL-terminated UTF-8 string
/// for the duration of the call. The session handle must be used only from
/// the thread that created it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_set_locale_variant(
    session_handle: u64,
    variant: *const std::ffi::c_char,
) -> ReciteStatus {
    let variant = match unsafe { super::parse_optional_session_string(variant, "locale variant") } {
        Ok(variant) => variant,
        Err(status) => return status,
    };
    set_locale_variant_value(session_handle, variant)
}

pub(crate) fn set_locale_variant_value(
    session_handle: u64,
    variant: Option<String>,
) -> ReciteStatus {
    let mut guard = super::lock_sessions();
    let Some(session) = guard.get_mut(&session_handle) else {
        set_last_error("unknown session handle");
        return ReciteStatus::InvalidHandle;
    };
    if let Err(status) = super::ensure_session_thread(session) {
        return status;
    }
    session.locale_variant = variant;
    ReciteStatus::Ok
}
