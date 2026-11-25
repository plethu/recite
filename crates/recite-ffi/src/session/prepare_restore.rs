use crate::error::{ReciteStatus, set_last_error};

/// Restores a checkpoint against the exact asset it was saved against,
/// without running traversal. Any change to the compiled asset returns
/// `SaveLoadIncompatibility`; use `recite_session_prepare_resume` to resume a
/// save in a patched asset.
///
/// Configure condition handlers, interpolation values and locale providers on
/// the returned handle, then call `recite_session_begin`, which presents a
/// pending prompt again or re-emits a pending blocking effect with its
/// original ID. Failed preparation does not allocate a session handle.
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
    let handle =
        match unsafe { super::resume::prepare_exact(asset_handle, snapshot_bytes, snapshot_len) } {
            Ok(handle) => handle,
            Err(status) => return status,
        };
    unsafe { *session_handle_out = handle };
    ReciteStatus::Ok
}
