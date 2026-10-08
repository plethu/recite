use crate::buffer::ReciteBuffer;
use crate::error::{ReciteStatus, set_last_error};

/// Restores and begins a checkpoint needing no host configuration.
/// Use prepare_restore, the session setters, then begin when resumption needs
/// condition handlers, interpolation values, a catalogue or locale provider.
///
/// Pending prompts return an empty batch; blocking effects are re-emitted
/// with the original request ID. Ended checkpoints return NoActiveSession.
/// Failure publishes neither a handle nor a batch.
///
/// # Safety
/// Both output pointers must be non-null and valid for the call. Snapshot
/// bytes must be non-null and valid for snapshot_len bytes when the length
/// fits Rust isize; larger lengths are rejected before reading.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_restore(
    asset_handle: u64,
    snapshot_bytes: *const u8,
    snapshot_len: usize,
    session_handle_out: *mut u64,
    batch_out: *mut ReciteBuffer,
) -> ReciteStatus {
    if session_handle_out.is_null() || batch_out.is_null() {
        set_last_error("null pointer argument");
        return ReciteStatus::Validation;
    }
    let mut handle = 0;
    let status = unsafe {
        super::recite_session_prepare_restore(
            asset_handle,
            snapshot_bytes,
            snapshot_len,
            &raw mut handle,
        )
    };
    if status != ReciteStatus::Ok {
        return status;
    }
    unsafe { super::begin_and_publish(handle, session_handle_out, batch_out) }
}
