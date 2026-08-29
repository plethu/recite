use crate::buffer::ReciteBuffer;
use crate::error::{ReciteStatus, set_last_error};

/// Encodes the current session state as an opaque msgpack snapshot.
///
/// The snapshot bytes are written to a freshly allocated `ReciteBuffer`; the
/// host must call `recite_buffer_free` after consuming them.
///
/// # Safety
/// `snapshot_out` must be a valid non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_snapshot(
    session_handle: u64,
    snapshot_out: *mut ReciteBuffer,
) -> ReciteStatus {
    if snapshot_out.is_null() {
        set_last_error("null pointer argument");
        return ReciteStatus::Validation;
    }
    let guard = super::lock_sessions();
    let ffi_session = match guard.get(&session_handle) {
        Some(session) => session,
        None => {
            set_last_error("unknown session handle");
            return ReciteStatus::InvalidHandle;
        }
    };
    if let Err(status) = super::ensure_session_thread(ffi_session) {
        return status;
    }
    match recite_runtime::encode_session_messagepack(&ffi_session.session) {
        Ok(bytes) => {
            unsafe { *snapshot_out = ReciteBuffer::from_bytes(bytes) };
            ReciteStatus::Ok
        }
        Err(error) => {
            set_last_error(&error.to_string());
            ReciteStatus::from(error)
        }
    }
}
