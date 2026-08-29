use std::ffi::CStr;

use crate::error::{ReciteStatus, set_last_error};

/// Validates a complete gettext `Plural-Forms` header using Recite's shared
/// bounded expression validator and returns its declared arm count.
///
/// # Safety
/// `header` must point to a valid NUL-terminated UTF-8 string and
/// `nplurals_out` must be a valid non-null pointer for the duration of the
/// call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_locale_validate_plural_rule(
    header: *const std::ffi::c_char,
    nplurals_out: *mut usize,
) -> ReciteStatus {
    if header.is_null() || nplurals_out.is_null() {
        set_last_error("null pointer argument");
        return ReciteStatus::Validation;
    }
    let header = match unsafe { CStr::from_ptr(header) }.to_str() {
        Ok(header) => header,
        Err(_) => {
            set_last_error("plural rule header is not valid UTF-8");
            return ReciteStatus::Validation;
        }
    };
    match recite_core::validate_plural_rule(header) {
        Ok(nplurals) => {
            unsafe { *nplurals_out = nplurals };
            ReciteStatus::Ok
        }
        Err(error) => {
            set_last_error(&error.to_string());
            ReciteStatus::Localisation
        }
    }
}

/// Evaluates one count with a complete, previously validated gettext rule.
/// The native core remains the authority for arm selection; hosts must not
/// provide a competing selector.
///
/// # Safety
/// `header` must point to a valid NUL-terminated UTF-8 string and
/// `arm_out` must be a valid non-null pointer for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_locale_evaluate_plural_rule(
    header: *const std::ffi::c_char,
    count: i64,
    arm_out: *mut usize,
) -> ReciteStatus {
    if header.is_null() || arm_out.is_null() {
        set_last_error("null pointer argument");
        return ReciteStatus::Validation;
    }
    let header = match unsafe { CStr::from_ptr(header) }.to_str() {
        Ok(header) => header,
        Err(_) => {
            set_last_error("plural rule header is not valid UTF-8");
            return ReciteStatus::Validation;
        }
    };
    match recite_core::evaluate_plural_form(header, count) {
        Ok(arm) => {
            unsafe { *arm_out = arm };
            ReciteStatus::Ok
        }
        Err(error) => {
            set_last_error(&error.to_string());
            ReciteStatus::Localisation
        }
    }
}

/// Validates that a non-empty translated string preserves the source's
/// interpolation placeholder names and multiplicities.
///
/// # Safety
/// Both arguments must point to valid NUL-terminated UTF-8 strings for the
/// duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recite_locale_validate_translation_placeholders(
    source: *const std::ffi::c_char,
    translation: *const std::ffi::c_char,
) -> ReciteStatus {
    if source.is_null() || translation.is_null() {
        set_last_error("null pointer argument");
        return ReciteStatus::Validation;
    }
    let source = match unsafe { CStr::from_ptr(source) }.to_str() {
        Ok(source) => source,
        Err(_) => {
            set_last_error("source text is not valid UTF-8");
            return ReciteStatus::Validation;
        }
    };
    let translation = match unsafe { CStr::from_ptr(translation) }.to_str() {
        Ok(translation) => translation,
        Err(_) => {
            set_last_error("translation is not valid UTF-8");
            return ReciteStatus::Validation;
        }
    };
    match recite_core::validate_translation_placeholders(source, translation) {
        Ok(()) => ReciteStatus::Ok,
        Err(error) => {
            set_last_error(&format!("invalid translation placeholders: {error:?}"));
            ReciteStatus::Localisation
        }
    }
}
