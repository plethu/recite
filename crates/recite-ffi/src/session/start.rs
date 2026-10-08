use crate::buffer::ReciteBuffer;
use crate::error::{ReciteStatus, set_last_error};

/// Creates and begins a session that needs no host configuration before traversal.
/// For conditions, interpolation values or locale providers, use create, the
/// session setters, then begin instead.
///
/// On success publishes a handle and the initial batch. Failure frees the
/// prepared session and leaves both outputs unchanged.
///
/// # Safety
/// Both output pointers must be non-null and valid for the call. Non-null
/// start_block and locale pointers must be valid NUL-terminated UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_start(
    asset_handle: u64,
    start_block: *const std::ffi::c_char,
    locale: *const std::ffi::c_char,
    session_handle_out: *mut u64,
    batch_out: *mut ReciteBuffer,
) -> ReciteStatus {
    if session_handle_out.is_null() || batch_out.is_null() {
        set_last_error("null pointer argument");
        return ReciteStatus::Validation;
    }
    let mut handle = 0;
    let status =
        unsafe { super::recite_session_create(asset_handle, start_block, locale, &raw mut handle) };
    if status != ReciteStatus::Ok {
        return status;
    }
    unsafe { super::begin_and_publish(handle, session_handle_out, batch_out) }
}
