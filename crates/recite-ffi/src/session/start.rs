use crate::buffer::ReciteBuffer;
use crate::error::{ReciteStatus, set_last_error};
use crate::interpolation::ReciteInterpolationValue;

struct StartRequest {
    asset_handle: u64,
    start_block: *const std::ffi::c_char,
    locale: *const std::ffi::c_char,
    locale_variant: *const std::ffi::c_char,
    values: *const ReciteInterpolationValue,
    values_len: usize,
    callback: Option<
        unsafe extern "C" fn(
            *const crate::locale::ReciteLocaleQuery,
            *mut std::ffi::c_void,
        ) -> crate::locale::ReciteLocaleResult,
    >,
    userdata: *mut std::ffi::c_void,
}

struct StartOutputs {
    session_handle_out: *mut u64,
    batch_out: *mut ReciteBuffer,
}

/// Convenience that combines `recite_session_create` and `recite_session_begin`.
///
/// Use this when no conditions appear in the opening block of the scene. For
/// scenes that evaluate conditions at scene start, use `recite_session_create`
/// + `recite_session_register_condition` + `recite_session_begin` instead.
///
/// `start_block` and `locale` are nullable UTF-8 NUL-terminated strings.
/// On success writes a non-zero handle to `*session_handle_out` and the initial
/// output batch to `*batch_out`.
///
/// # Safety
/// All non-null pointer arguments must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_start(
    asset_handle: u64,
    start_block: *const std::ffi::c_char,
    locale: *const std::ffi::c_char,
    session_handle_out: *mut u64,
    batch_out: *mut ReciteBuffer,
) -> ReciteStatus {
    unsafe {
        super::recite_session_start_with_values(
            asset_handle,
            start_block,
            locale,
            std::ptr::null(),
            0,
            session_handle_out,
            batch_out,
        )
    }
}

/// Convenience that combines session creation and the initial traversal drain
/// while supplying typed interpolation values.
///
/// Input records are borrowed only for this call and copied into session-owned
/// storage. Use `recite_session_set_interpolation_values` to replace them for a
/// later traversal operation.
///
/// # Safety
/// All non-null pointer arguments, including each record's string pointers,
/// must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_start_with_values(
    asset_handle: u64,
    start_block: *const std::ffi::c_char,
    locale: *const std::ffi::c_char,
    values: *const ReciteInterpolationValue,
    values_len: usize,
    session_handle_out: *mut u64,
    batch_out: *mut ReciteBuffer,
) -> ReciteStatus {
    if session_handle_out.is_null() || batch_out.is_null() {
        set_last_error("null pointer argument");
        return ReciteStatus::Validation;
    }

    let status = unsafe {
        super::recite_session_create_with_values(
            asset_handle,
            start_block,
            locale,
            values,
            values_len,
            session_handle_out,
        )
    };
    if status != ReciteStatus::Ok {
        return status;
    }

    let handle = unsafe { *session_handle_out };
    let begin_status = unsafe { super::recite_session_begin(handle, batch_out) };
    if begin_status != ReciteStatus::Ok {
        // Remove the created (but unbegun) session so the caller doesn't hold
        // a handle that can't be used.
        super::lock_sessions().remove(&handle);
        unsafe { *session_handle_out = 0 };
    }
    begin_status
}

/// Convenience that creates a session, installs a locale callback, and runs
/// the initial traversal drain.
///
/// The callback result is copied during the enclosing synchronous call. Every
/// result pointer must remain immutable and valid until that call returns; a
/// null locale still selects source-text-only mode and bypasses the callback.
///
/// # Safety
/// All non-null pointers must be valid for the duration of the call. The
/// callback must be a valid non-null function pointer, and `userdata` must
/// remain valid for the session lifetime. Passing NULL as `callback` returns
/// `RECITE_STATUS_VALIDATION` before a session is created.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_start_with_locale_provider(
    asset_handle: u64,
    start_block: *const std::ffi::c_char,
    locale: *const std::ffi::c_char,
    callback: Option<
        unsafe extern "C" fn(
            *const crate::locale::ReciteLocaleQuery,
            *mut std::ffi::c_void,
        ) -> crate::locale::ReciteLocaleResult,
    >,
    userdata: *mut std::ffi::c_void,
    session_handle_out: *mut u64,
    batch_out: *mut ReciteBuffer,
) -> ReciteStatus {
    unsafe {
        start_with_provider(
            StartRequest {
                asset_handle,
                start_block,
                locale,
                locale_variant: std::ptr::null(),
                values: std::ptr::null(),
                values_len: 0,
                callback,
                userdata,
            },
            StartOutputs {
                session_handle_out,
                batch_out,
            },
        )
    }
}

/// Convenience that creates a session, installs a locale callback, stores
/// typed interpolation values, and runs the initial traversal drain.
///
/// # Safety
/// All non-null pointers must be valid for the duration of the call. The
/// callback must be a valid non-null function pointer, and `userdata` must
/// remain valid for the session lifetime. Passing NULL as `callback` returns
/// `RECITE_STATUS_VALIDATION` before a session is created.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_start_with_values_and_locale_provider(
    asset_handle: u64,
    start_block: *const std::ffi::c_char,
    locale: *const std::ffi::c_char,
    values: *const ReciteInterpolationValue,
    values_len: usize,
    callback: Option<
        unsafe extern "C" fn(
            *const crate::locale::ReciteLocaleQuery,
            *mut std::ffi::c_void,
        ) -> crate::locale::ReciteLocaleResult,
    >,
    userdata: *mut std::ffi::c_void,
    session_handle_out: *mut u64,
    batch_out: *mut ReciteBuffer,
) -> ReciteStatus {
    unsafe {
        start_with_provider(
            StartRequest {
                asset_handle,
                start_block,
                locale,
                locale_variant: std::ptr::null(),
                values,
                values_len,
                callback,
                userdata,
            },
            StartOutputs {
                session_handle_out,
                batch_out,
            },
        )
    }
}

/// Convenience that creates a session, installs a locale callback and
/// grammatical variant, and runs the initial traversal drain.
///
/// The variant is copied into the session and is not part of serialized state.
/// It therefore has to be supplied again when restoring a snapshot.
///
/// # Safety
/// All non-null pointers must be valid for the duration of the call. The
/// callback must be a valid non-null function pointer, and `userdata` must
/// remain valid for the session lifetime. Passing NULL as `callback` returns
/// `RECITE_STATUS_VALIDATION` before a session is created.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_start_with_locale_provider_and_variant(
    asset_handle: u64,
    start_block: *const std::ffi::c_char,
    locale: *const std::ffi::c_char,
    locale_variant: *const std::ffi::c_char,
    callback: Option<
        unsafe extern "C" fn(
            *const crate::locale::ReciteLocaleQuery,
            *mut std::ffi::c_void,
        ) -> crate::locale::ReciteLocaleResult,
    >,
    userdata: *mut std::ffi::c_void,
    session_handle_out: *mut u64,
    batch_out: *mut ReciteBuffer,
) -> ReciteStatus {
    unsafe {
        start_with_provider(
            StartRequest {
                asset_handle,
                start_block,
                locale,
                locale_variant,
                values: std::ptr::null(),
                values_len: 0,
                callback,
                userdata,
            },
            StartOutputs {
                session_handle_out,
                batch_out,
            },
        )
    }
}

/// Convenience that creates a session, installs typed interpolation values,
/// a locale callback, and a grammatical variant before the initial drain.
///
/// # Safety
/// All non-null pointers must be valid for the duration of the call. The
/// callback must be a valid non-null function pointer, and `userdata` must
/// remain valid for the session lifetime. Passing NULL as `callback` returns
/// `RECITE_STATUS_VALIDATION` before a session is created.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_session_start_with_values_and_locale_provider_and_variant(
    asset_handle: u64,
    start_block: *const std::ffi::c_char,
    locale: *const std::ffi::c_char,
    locale_variant: *const std::ffi::c_char,
    values: *const ReciteInterpolationValue,
    values_len: usize,
    callback: Option<
        unsafe extern "C" fn(
            *const crate::locale::ReciteLocaleQuery,
            *mut std::ffi::c_void,
        ) -> crate::locale::ReciteLocaleResult,
    >,
    userdata: *mut std::ffi::c_void,
    session_handle_out: *mut u64,
    batch_out: *mut ReciteBuffer,
) -> ReciteStatus {
    unsafe {
        start_with_provider(
            StartRequest {
                asset_handle,
                start_block,
                locale,
                locale_variant,
                values,
                values_len,
                callback,
                userdata,
            },
            StartOutputs {
                session_handle_out,
                batch_out,
            },
        )
    }
}

unsafe fn start_with_provider(request: StartRequest, outputs: StartOutputs) -> ReciteStatus {
    if outputs.session_handle_out.is_null() || outputs.batch_out.is_null() {
        set_last_error("null pointer argument");
        return ReciteStatus::Validation;
    }
    let Some(callback) = request.callback else {
        set_last_error("locale callback is null");
        return ReciteStatus::Validation;
    };
    let locale_variant = match unsafe {
        super::parse_optional_session_string(request.locale_variant, "locale variant")
    } {
        Ok(locale_variant) => locale_variant,
        Err(status) => return status,
    };
    let status = unsafe {
        super::recite_session_create_with_values(
            request.asset_handle,
            request.start_block,
            request.locale,
            request.values,
            request.values_len,
            outputs.session_handle_out,
        )
    };
    if status != ReciteStatus::Ok {
        return status;
    }
    let handle = unsafe { *outputs.session_handle_out };
    let status = unsafe {
        super::recite_session_set_locale_provider(handle, Some(callback), request.userdata)
    };
    if status != ReciteStatus::Ok {
        super::lock_sessions().remove(&handle);
        unsafe { *outputs.session_handle_out = 0 };
        return status;
    }
    let status = super::set_locale_variant_value(handle, locale_variant);
    if status != ReciteStatus::Ok {
        super::lock_sessions().remove(&handle);
        unsafe { *outputs.session_handle_out = 0 };
        return status;
    }
    let status = unsafe { super::recite_session_begin(handle, outputs.batch_out) };
    if status != ReciteStatus::Ok {
        super::lock_sessions().remove(&handle);
        unsafe { *outputs.session_handle_out = 0 };
    }
    status
}
