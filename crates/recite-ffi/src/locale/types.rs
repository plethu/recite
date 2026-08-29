use std::ffi::{c_char, c_void};

use recite_runtime::{PluralResolutionOutcome, TextDomain};

/// Operation requested from a locale callback.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReciteLocaleRequestKind {
    Singular = 0,
    Plural = 1,
}

/// Localisable text domain passed to a locale callback.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReciteLocaleTextDomain {
    Line = 0,
    Choice = 1,
    AvailabilityReason = 2,
    PresentationLabel = 3,
}

// Keep the C convenience aliases as literals: cbindgen can emit these in C
// and C++ without relying on Rust variant names that do not exist in either
// generated enum spelling.
pub const RECITE_LOCALE_REQUEST_SINGULAR: u32 = 0;
pub const RECITE_LOCALE_REQUEST_PLURAL: u32 = 1;
pub const RECITE_LOCALE_DOMAIN_LINE: u32 = 0;
pub const RECITE_LOCALE_DOMAIN_CHOICE: u32 = 1;
pub const RECITE_LOCALE_DOMAIN_AVAILABILITY_REASON: u32 = 2;
pub const RECITE_LOCALE_DOMAIN_PRESENTATION_LABEL: u32 = 3;

impl TryFrom<u32> for ReciteLocaleTextDomain {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Line),
            1 => Ok(Self::Choice),
            2 => Ok(Self::AvailabilityReason),
            3 => Ok(Self::PresentationLabel),
            _ => Err(()),
        }
    }
}

impl From<ReciteLocaleTextDomain> for TextDomain {
    fn from(value: ReciteLocaleTextDomain) -> Self {
        match value {
            ReciteLocaleTextDomain::Line => Self::Line,
            ReciteLocaleTextDomain::Choice => Self::Choice,
            ReciteLocaleTextDomain::AvailabilityReason => Self::AvailabilityReason,
            ReciteLocaleTextDomain::PresentationLabel => Self::PresentationLabel,
        }
    }
}

/// Outcome for one locale-catalog candidate attempt.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReciteLocaleAttemptOutcome {
    MissingPluralForms = 0,
    MissingEntry = 1,
    MissingTranslation = 2,
    Matched = 3,
}

pub const RECITE_LOCALE_ATTEMPT_MISSING_PLURAL_FORMS: u32 = 0;
pub const RECITE_LOCALE_ATTEMPT_MISSING_ENTRY: u32 = 1;
pub const RECITE_LOCALE_ATTEMPT_MISSING_TRANSLATION: u32 = 2;
pub const RECITE_LOCALE_ATTEMPT_MATCHED: u32 = 3;

impl TryFrom<u32> for ReciteLocaleAttemptOutcome {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::MissingPluralForms),
            1 => Ok(Self::MissingEntry),
            2 => Ok(Self::MissingTranslation),
            3 => Ok(Self::Matched),
            _ => Err(()),
        }
    }
}

impl From<ReciteLocaleAttemptOutcome> for PluralResolutionOutcome {
    fn from(value: ReciteLocaleAttemptOutcome) -> Self {
        match value {
            ReciteLocaleAttemptOutcome::MissingPluralForms => Self::MissingPluralForms,
            ReciteLocaleAttemptOutcome::MissingEntry => Self::MissingEntry,
            ReciteLocaleAttemptOutcome::MissingTranslation => Self::MissingTranslation,
            ReciteLocaleAttemptOutcome::Matched => Self::Matched,
        }
    }
}

/// Query passed synchronously to a locale callback.
///
/// All pointers are Recite-owned borrows valid until the callback returns.
#[repr(C)]
pub struct ReciteLocaleQuery {
    pub kind: u32,
    pub id: *const c_char,
    pub source_text: *const c_char,
    pub plural_source_text: *const c_char,
    pub count: i64,
    pub domain: u32,
    pub locale: *const c_char,
    pub variant: *const c_char,
}

/// One callback-provided plural candidate attempt.
///
/// Strings are host-owned and must remain immutable and valid until the
/// enclosing Recite API call returns. A negative `selected_arm` means that the
/// candidate had no selected arm.
///
/// Hosts must enumerate candidates in this exact order: first the requested
/// variant context (`context&variant`) across the locale's most-specific to
/// least-specific fallback chain, then the base context across that same
/// chain. A missing plural rule, missing entry, empty translation, or fuzzy
/// translation continues to the next candidate. Fuzzy and empty catalogue
/// records use `RECITE_LOCALE_ATTEMPT_MISSING_TRANSLATION`; a catalogue
/// conflict must not be reported as a match. `RECITE_LOCALE_ATTEMPT_MATCHED`
/// terminates the sequence. The selected arm and matched provenance must come
/// from the validated rule and candidate that produced the match. Violating
/// this order is a host contract violation; Recite records the supplied
/// sequence but cannot enforce a custom provider's catalogue lookup.
#[repr(C)]
pub struct ReciteLocaleAttempt {
    pub locale: *const c_char,
    pub context: *const c_char,
    pub key: *const c_char,
    pub selected_arm: i32,
    pub outcome: u32,
}

/// Result returned synchronously by a locale callback.
///
/// `text` is nullable to request the runtime's authored source fallback. For
/// plural requests, a non-null `text` requires a non-negative `selected_arm`.
/// The complete returned pointer tree is host-owned and must remain valid and
/// immutable from callback return until the enclosing Recite API call returns:
/// this includes `text`, `error_message`, every `matched_*` string, the
/// `attempts` array, and every string in every attempt. Stack or callback-local
/// storage is invalid. Recite copies the tree while processing the enclosing
/// call, then the host may release its owner storage. For a plural match,
/// `attempts` must end with the matching candidate and the three `matched_*`
/// fields must describe that same candidate. For an unresolved plural lookup,
/// return `text = NULL`, `selected_arm = -1`, and null matching provenance
/// after reporting every attempted candidate; traversal then uses the
/// authored English source form and rule. An attempt with an empty or fuzzy
/// translation must continue rather than terminate resolution.
#[repr(C)]
pub struct ReciteLocaleResult {
    pub ok: u8,
    pub text: *const c_char,
    pub selected_arm: i32,
    pub matched_locale: *const c_char,
    pub matched_context: *const c_char,
    pub matched_key: *const c_char,
    pub attempts: *const ReciteLocaleAttempt,
    pub attempts_len: usize,
    pub error_message: *const c_char,
}

/// Host-provided locale callback.
///
/// The callback runs synchronously on the session owner thread and must obey
/// the strict non-null, no-throw/no-panic/no-unwind, and no-re-entry contract.
/// It must return `ok = 0` for a failed lookup. A Rust panic from an
/// `extern "C"` callback aborts before Recite can catch it; C++ hosts must
/// catch exceptions in their own wrapper before entering Recite.
/// The host owns the complete result pointer tree until the enclosing Recite
/// API call returns, not merely until this callback returns. It must not use
/// stack or callback-local storage, and must not mutate or release the result
/// tree before that call returns.
pub type ReciteLocaleFn = unsafe extern "C" fn(
    query: *const ReciteLocaleQuery,
    userdata: *mut c_void,
) -> ReciteLocaleResult;
