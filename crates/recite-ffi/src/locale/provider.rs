use std::ffi::{CStr, CString, c_char, c_void};

use recite_core::LocaleId;
use recite_runtime::{
    LocaleError, LocaleProvider, PluralResolution, PluralResolutionAttempt, TextDomain,
};

use super::types::{
    ReciteLocaleAttempt, ReciteLocaleAttemptOutcome, ReciteLocaleFn, ReciteLocaleQuery,
    ReciteLocaleRequestKind, ReciteLocaleResult, ReciteLocaleTextDomain,
};

/// A copied callback registration held by one FFI session.
pub(crate) struct FfiLocaleProvider {
    callback: ReciteLocaleFn,
    pub(crate) userdata: SendLocalePtr,
}

/// Raw callback userdata is safe to move with a session because the FFI layer
/// rejects operations from any thread other than the session owner.
pub(crate) struct SendLocalePtr(pub *mut c_void);

// SAFETY: the session owner-thread check runs before locale callbacks.
unsafe impl Send for SendLocalePtr {}

struct LocaleQuery<'a> {
    kind: ReciteLocaleRequestKind,
    id: &'a str,
    source_text: &'a str,
    plural_source_text: Option<&'a str>,
    count: i64,
    domain: TextDomain,
    locale: &'a LocaleId,
    variant: Option<&'a str>,
}

impl FfiLocaleProvider {
    pub(crate) fn new(callback: ReciteLocaleFn, userdata: *mut c_void) -> Self {
        Self {
            callback,
            userdata: SendLocalePtr(userdata),
        }
    }

    fn query(&self, request: LocaleQuery<'_>) -> Result<ReciteLocaleResult, LocaleError> {
        let id = cstring(request.id, "locale ID")?;
        let source_text = cstring(request.source_text, "locale source text")?;
        let plural_source_text = request
            .plural_source_text
            .map(|value| cstring(value, "locale plural source text"))
            .transpose()?;
        let locale_id = cstring(request.locale.as_str(), "locale")?;
        let variant = request
            .variant
            .map(|value| cstring(value, "locale variant"))
            .transpose()?;
        let query = ReciteLocaleQuery {
            kind: request.kind as u32,
            id: id.as_ptr(),
            source_text: source_text.as_ptr(),
            plural_source_text: plural_source_text
                .as_ref()
                .map_or(std::ptr::null(), |value| value.as_ptr()),
            count: request.count,
            domain: domain_number(request.domain),
            locale: locale_id.as_ptr(),
            variant: variant
                .as_ref()
                .map_or(std::ptr::null(), |value| value.as_ptr()),
        };
        // SAFETY: the callback and userdata were supplied by the host, and all
        // query pointers remain valid for the synchronous callback frame. The
        // returned pointer tree is host-owned and must remain valid and
        // immutable until the enclosing public Recite API call returns. The
        // callback's strict C ABI contract forbids panic, throw, and unwind;
        // Rust cannot recover a panic after it crosses this ABI.
        Ok(unsafe { (self.callback)(&raw const query, self.userdata.0) })
    }
}

impl LocaleProvider for FfiLocaleProvider {
    fn lookup(
        &self,
        id: &str,
        source_text: &str,
        domain: TextDomain,
        locale: &LocaleId,
        variant: Option<&str>,
    ) -> Result<Option<String>, LocaleError> {
        let result = self.query(LocaleQuery {
            kind: ReciteLocaleRequestKind::Singular,
            id,
            source_text,
            plural_source_text: None,
            count: -1,
            domain,
            locale,
            variant,
        })?;
        if result.ok == 0 {
            return Err(callback_error(result, "locale callback failed"));
        }
        if result.ok != 1 {
            return Err(LocaleError::new(format!(
                "locale callback returned invalid ok value {}",
                result.ok
            )));
        }
        validate_singular_result(&result)?;
        optional_utf8(result.text, "locale translation")
    }

    fn resolve_plural(
        &self,
        id: &str,
        source_singular: &str,
        source_plural: &str,
        count: i64,
        domain: TextDomain,
        locale: &LocaleId,
        variant: Option<&str>,
    ) -> Result<PluralResolution, LocaleError> {
        let result = self.query(LocaleQuery {
            kind: ReciteLocaleRequestKind::Plural,
            id,
            source_text: source_singular,
            plural_source_text: Some(source_plural),
            count,
            domain,
            locale,
            variant,
        })?;
        if result.ok == 0 {
            return Err(callback_error(result, "locale callback failed"));
        }
        if result.ok != 1 {
            return Err(LocaleError::new(format!(
                "locale callback returned invalid ok value {}",
                result.ok
            )));
        }
        let template = optional_utf8(result.text, "locale plural translation")?;
        let selected_arm = selected_arm(result.selected_arm)?;
        if template.is_some() && selected_arm.is_none() {
            return Err(LocaleError::new(
                "locale plural callback returned a template without a selected arm",
            ));
        }
        if template.is_none() && selected_arm.is_some() {
            return Err(LocaleError::new(
                "locale plural callback returned a selected arm without a template",
            ));
        }
        let attempts = attempts(result.attempts, result.attempts_len)?;
        Ok(PluralResolution {
            template,
            selected_arm,
            matched_locale: optional_utf8(result.matched_locale, "matched locale")?,
            matched_context: optional_utf8(result.matched_context, "matched context")?,
            matched_key: optional_utf8(result.matched_key, "matched key")?,
            attempts,
        })
    }
}

fn callback_error(result: ReciteLocaleResult, fallback: &str) -> LocaleError {
    let message = if result.error_message.is_null() {
        fallback.to_owned()
    } else {
        // SAFETY: callback result strings are host-owned and valid through the
        // enclosing public Recite API call. This copy occurs before that call
        // returns.
        unsafe { CStr::from_ptr(result.error_message) }
            .to_str()
            .map_or_else(
                |_| "locale callback error is not valid UTF-8".to_owned(),
                str::to_owned,
            )
    };
    LocaleError::new(message)
}

fn validate_singular_result(result: &ReciteLocaleResult) -> Result<(), LocaleError> {
    if result.selected_arm != -1 {
        return Err(LocaleError::new(
            "singular locale callback returned a selected arm",
        ));
    }
    if result.attempts_len != 0 {
        return Err(LocaleError::new(
            "singular locale callback returned plural attempts",
        ));
    }
    Ok(())
}

fn cstring(value: &str, label: &str) -> Result<CString, LocaleError> {
    CString::new(value).map_err(|_| LocaleError::new(format!("{label} contains NUL")))
}

fn optional_utf8(pointer: *const c_char, label: &str) -> Result<Option<String>, LocaleError> {
    if pointer.is_null() {
        return Ok(None);
    }
    // SAFETY: callback result strings are host-owned and valid through the
    // enclosing public Recite API call. This copy occurs before that call
    // returns.
    let value = unsafe { CStr::from_ptr(pointer) }
        .to_str()
        .map_err(|_| LocaleError::new(format!("{label} is not valid UTF-8")))?;
    Ok(Some(value.to_owned()))
}

fn selected_arm(value: i32) -> Result<Option<usize>, LocaleError> {
    if value < -1 {
        return Err(LocaleError::new(format!(
            "locale plural callback returned invalid selected arm {value}"
        )));
    }
    usize::try_from(value).map_or(Ok(None), |value| Ok(Some(value)))
}

fn attempts(
    pointer: *const ReciteLocaleAttempt,
    length: usize,
) -> Result<Vec<PluralResolutionAttempt>, LocaleError> {
    if length == 0 {
        return Ok(Vec::new());
    }
    if pointer.is_null() {
        return Err(LocaleError::new(
            "locale plural callback attempts pointer is null",
        ));
    }
    if length > isize::MAX as usize / std::mem::size_of::<ReciteLocaleAttempt>() {
        return Err(LocaleError::new(
            "locale plural callback attempts length is too large",
        ));
    }
    // SAFETY: the callback guarantees a valid immutable array through the
    // enclosing public Recite API call; this copy occurs before that call
    // returns.
    let records = unsafe { std::slice::from_raw_parts(pointer, length) };
    records
        .iter()
        .map(|attempt| {
            Ok(PluralResolutionAttempt {
                locale: required_utf8(attempt.locale, "locale plural attempt locale")?,
                context: required_utf8(attempt.context, "locale plural attempt context")?,
                key: required_utf8(attempt.key, "locale plural attempt key")?,
                selected_arm: selected_arm(attempt.selected_arm)?,
                outcome: ReciteLocaleAttemptOutcome::try_from(attempt.outcome)
                    .map_err(|_| {
                        LocaleError::new("locale plural callback returned unknown outcome")
                    })?
                    .into(),
            })
        })
        .collect()
}

fn required_utf8(pointer: *const c_char, label: &str) -> Result<String, LocaleError> {
    optional_utf8(pointer, label)?.ok_or_else(|| LocaleError::new(format!("{label} is null")))
}

fn domain_number(domain: TextDomain) -> u32 {
    match domain {
        TextDomain::Line => ReciteLocaleTextDomain::Line as u32,
        TextDomain::Choice => ReciteLocaleTextDomain::Choice as u32,
        TextDomain::AvailabilityReason => ReciteLocaleTextDomain::AvailabilityReason as u32,
        TextDomain::PresentationLabel => ReciteLocaleTextDomain::PresentationLabel as u32,
    }
}
