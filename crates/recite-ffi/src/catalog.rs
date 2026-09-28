use std::collections::BTreeMap;
use std::ffi::{CStr, c_char};
use std::sync::{Arc, Mutex, OnceLock};

use recite_adapter::ReciteDialogueCatalog;

use crate::asset::alloc_handle;
use crate::buffer::checked_bytes;
use crate::error::{ReciteStatus, set_last_error};
use crate::session::{FfiLocaleSource, ensure_session_thread, lock_sessions};

type CatalogMap = Mutex<BTreeMap<u64, Arc<ReciteDialogueCatalog>>>;

fn catalogs() -> &'static CatalogMap {
    static CATALOGS: OnceLock<CatalogMap> = OnceLock::new();
    CATALOGS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn lock_catalogs()
-> Result<std::sync::MutexGuard<'static, BTreeMap<u64, Arc<ReciteDialogueCatalog>>>, ReciteStatus> {
    catalogs().lock().map_err(|_| {
        set_last_error("catalogue registry lock is poisoned");
        ReciteStatus::DialogueFault
    })
}

pub(crate) fn catalog_for_handle(handle: u64) -> Result<Arc<ReciteDialogueCatalog>, ReciteStatus> {
    lock_catalogs()?.get(&handle).cloned().ok_or_else(|| {
        set_last_error("unknown catalogue handle");
        ReciteStatus::InvalidHandle
    })
}

/// Creates an empty owned dialogue catalogue handle.
///
/// # Safety
/// `catalog_handle_out` must be a valid non-null pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_catalog_create(catalog_handle_out: *mut u64) -> ReciteStatus {
    if catalog_handle_out.is_null() {
        set_last_error("null pointer argument");
        return ReciteStatus::Validation;
    }
    let handle = alloc_handle();
    let mut guard = match lock_catalogs() {
        Ok(guard) => guard,
        Err(status) => return status,
    };
    guard.insert(handle, Arc::new(ReciteDialogueCatalog::new()));
    unsafe { *catalog_handle_out = handle };
    ReciteStatus::Ok
}

/// Atomically merges one writer-owned gettext PO document into a catalogue.
/// Existing sessions retain their attached catalogue revision.
///
/// # Safety
/// `locale` must point to a valid NUL-terminated UTF-8 string. `po_bytes`
/// must be valid for `po_len` bytes when non-null and within `isize::MAX`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_catalog_add_po(
    catalog_handle: u64,
    locale: *const c_char,
    po_bytes: *const u8,
    po_len: usize,
) -> ReciteStatus {
    if locale.is_null() || po_bytes.is_null() {
        set_last_error("null pointer argument");
        return ReciteStatus::Validation;
    }
    let locale = match unsafe { CStr::from_ptr(locale) }.to_str() {
        Ok(locale) => locale,
        Err(_) => {
            set_last_error("locale is not valid UTF-8");
            return ReciteStatus::Validation;
        }
    };
    let bytes = match unsafe { checked_bytes(po_bytes, po_len, "PO bytes") } {
        Ok(bytes) => bytes,
        Err(error) => {
            set_last_error(&error);
            return ReciteStatus::Validation;
        }
    };
    let source = match std::str::from_utf8(bytes) {
        Ok(source) => source,
        Err(_) => {
            set_last_error("PO bytes are not valid UTF-8");
            return ReciteStatus::Localisation;
        }
    };
    let mut guard = match lock_catalogs() {
        Ok(guard) => guard,
        Err(status) => return status,
    };
    let Some(current) = guard.get_mut(&catalog_handle) else {
        set_last_error("unknown catalogue handle");
        return ReciteStatus::InvalidHandle;
    };
    match Arc::make_mut(current).import_po(locale, "<ffi catalog>", source) {
        Ok(()) => ReciteStatus::Ok,
        Err(error) => {
            set_last_error(&error.to_string());
            ReciteStatus::from(error)
        }
    }
}

/// Releases a catalogue handle. Sessions retain their attached revision.
#[unsafe(no_mangle)]
pub extern "C" fn recite_catalog_free(catalog_handle: u64) {
    if let Ok(mut guard) = lock_catalogs() {
        guard.remove(&catalog_handle);
    }
}

/// Attaches one owned catalogue revision to a session, replacing its locale
/// callback. An absent session locale still uses authored source text.
#[unsafe(no_mangle)]
pub extern "C" fn recite_session_set_catalog(
    session_handle: u64,
    catalog_handle: u64,
) -> ReciteStatus {
    let mut guard = lock_sessions();
    let Some(session) = guard.get_mut(&session_handle) else {
        set_last_error("unknown session handle");
        return ReciteStatus::InvalidHandle;
    };
    if let Err(status) = ensure_session_thread(session) {
        return status;
    }
    let catalog = match catalog_for_handle(catalog_handle) {
        Ok(catalog) => catalog,
        Err(status) => return status,
    };
    session.locale_source = FfiLocaleSource::Catalog(catalog);
    ReciteStatus::Ok
}
