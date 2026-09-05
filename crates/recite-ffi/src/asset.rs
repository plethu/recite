use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use recite_core::{CompiledDialogue, decode_compiled_dialogue_messagepack};

use crate::error::set_last_error;

type AssetMap = Mutex<BTreeMap<u64, std::sync::Arc<CompiledDialogue>>>;

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

pub(crate) fn alloc_handle() -> u64 {
    NEXT_HANDLE.fetch_add(1, Ordering::Relaxed)
}

pub(crate) fn assets() -> &'static AssetMap {
    static ASSETS: OnceLock<AssetMap> = OnceLock::new();
    ASSETS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

// Mutex poison is unrecoverable in a cdylib boundary; unwrap is intentional.
#[allow(
    clippy::unwrap_used,
    reason = "ffi: the process-global asset registry cannot recover a poisoned mutex"
)]
pub(crate) fn lock_assets()
-> std::sync::MutexGuard<'static, BTreeMap<u64, std::sync::Arc<CompiledDialogue>>> {
    assets().lock().unwrap()
}

/// Loads and decodes a compiled Recite asset from a byte slice.
///
/// On success writes a non-zero handle to `*asset_handle_out` and returns
/// `RECITE_OK`. On failure returns a negative status code and sets the
/// thread-local error message.
///
/// The asset handle is valid until `recite_asset_free` is called.
///
/// # Safety
/// `bytes` must be valid for `len` bytes. `asset_handle_out` must be a valid
/// non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_asset_load(
    bytes: *const u8,
    len: usize,
    asset_handle_out: *mut u64,
) -> crate::ReciteStatus {
    if bytes.is_null() || asset_handle_out.is_null() {
        set_last_error("null pointer argument");
        return crate::ReciteStatus::Validation;
    }
    let bytes_slice = unsafe { std::slice::from_raw_parts(bytes, len) };
    match decode_compiled_dialogue_messagepack(bytes_slice) {
        Ok(dialogue) => {
            let handle = alloc_handle();
            lock_assets().insert(handle, std::sync::Arc::new(dialogue));
            unsafe { *asset_handle_out = handle };
            crate::ReciteStatus::Ok
        }
        Err(e) => {
            set_last_error(&e.to_string());
            crate::ReciteStatus::AssetLoadOrDecode
        }
    }
}

/// Frees an asset handle.
///
/// If sessions that reference this asset are still alive, their internal
/// `Arc` reference keeps the underlying data alive until those sessions are
/// also freed.
#[unsafe(no_mangle)]
pub extern "C" fn recite_asset_free(asset_handle: u64) {
    lock_assets().remove(&asset_handle);
}
