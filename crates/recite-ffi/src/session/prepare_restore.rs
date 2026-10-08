use crate::asset::{alloc_handle, lock_assets};
use crate::buffer::checked_bytes;
use crate::error::{ReciteStatus, set_last_error};
use recite_adapter::{LoadedDialogue, SessionDriver};

/// Restores a checkpoint without running traversal.
///
/// Configure condition handlers, interpolation values and locale providers on
/// the returned handle, then call `recite_session_begin`. A pending prompt
/// resumes with an empty batch; a pending blocking effect is re-emitted with
/// its original ID. Failed preparation does not allocate a session handle.
///
/// # Safety
/// `session_handle_out` must be a valid non-null pointer. `snapshot_bytes`
/// must be non-null and valid for `snapshot_len` bytes when the length fits
/// Rust `isize`; larger lengths are rejected before reading the snapshot.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_prepare_restore(
    asset_handle: u64,
    snapshot_bytes: *const u8,
    snapshot_len: usize,
    session_handle_out: *mut u64,
) -> ReciteStatus {
    if session_handle_out.is_null() {
        set_last_error("null session handle output");
        return ReciteStatus::Validation;
    }
    let bytes = match unsafe { checked_bytes(snapshot_bytes, snapshot_len, "snapshot bytes") } {
        Ok(bytes) => bytes,
        Err(error) => {
            set_last_error(&error);
            return ReciteStatus::Validation;
        }
    };
    let Some(dialogue) = lock_assets().get(&asset_handle).cloned() else {
        set_last_error("unknown asset handle");
        return ReciteStatus::InvalidHandle;
    };
    let mut driver = SessionDriver::new();
    let result = LoadedDialogue::from_shared(dialogue)
        .and_then(|asset| driver.prepare_restore(&asset, bytes));
    if let Err(error) = result {
        set_last_error(&error.to_string());
        return ReciteStatus::from(error);
    }
    let handle = alloc_handle();
    super::lock_sessions().insert(handle, super::FfiSession::prepared(driver));
    unsafe { *session_handle_out = handle };
    ReciteStatus::Ok
}
