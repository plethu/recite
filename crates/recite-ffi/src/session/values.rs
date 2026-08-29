use crate::error::{ReciteStatus, set_last_error};
use crate::interpolation::{ReciteInterpolationValue, parse_interpolation_values};

/// Replaces the typed interpolation values attached to a session.
///
/// Values are copied before this function returns and are therefore safe for
/// a host to keep in temporary input buffers. Passing a null pointer with a
/// length of zero clears the map. Updating values never changes serialised
/// session state; the next traversal operation observes the replacement map.
///
/// # Safety
/// All non-null pointer arguments, including each record's string pointers,
/// must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_set_interpolation_values(
    session_handle: u64,
    values: *const ReciteInterpolationValue,
    values_len: usize,
) -> ReciteStatus {
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

    let interpolation_values = match unsafe { parse_interpolation_values(values, values_len) } {
        Ok(values) => values,
        Err(error) => {
            set_last_error(&error);
            return ReciteStatus::Validation;
        }
    };
    ffi_session.interpolation_values = interpolation_values;
    ReciteStatus::Ok
}

/// Frees a session handle. Does nothing if the handle is unknown.
#[unsafe(no_mangle)]
pub extern "C" fn recite_session_free(session_handle: u64) {
    super::lock_sessions().remove(&session_handle);
}
