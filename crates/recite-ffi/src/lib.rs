//! C ABI surface for non-Rust engine adapters.
//!
//! Design decisions are documented in `docs/c-abi-boundary-design.md`.
//! Normative adapter semantics are in `docs/engine-adapter-contract.md`.

mod asset;
mod buffer;
mod condition;
mod condition_codec;
mod error;
mod interpolation;
mod locale;
mod output;
mod session;

pub use asset::{recite_asset_free, recite_asset_load};
pub use buffer::{ReciteBuffer, recite_buffer_free};
pub use condition::{ReciteConditionFn, ReciteConditionQuery, ReciteConditionResult};
pub use error::{ReciteStatus, recite_last_error_message};
pub use interpolation::{ReciteInterpolationValue, ReciteInterpolationValueKind};
pub use locale::{
    RECITE_LOCALE_ATTEMPT_MATCHED, RECITE_LOCALE_ATTEMPT_MISSING_ENTRY,
    RECITE_LOCALE_ATTEMPT_MISSING_PLURAL_FORMS, RECITE_LOCALE_ATTEMPT_MISSING_TRANSLATION,
    RECITE_LOCALE_DOMAIN_AVAILABILITY_REASON, RECITE_LOCALE_DOMAIN_CHOICE,
    RECITE_LOCALE_DOMAIN_LINE, RECITE_LOCALE_DOMAIN_PRESENTATION_LABEL,
    RECITE_LOCALE_REQUEST_PLURAL, RECITE_LOCALE_REQUEST_SINGULAR, ReciteLocaleAttempt,
    ReciteLocaleAttemptOutcome, ReciteLocaleFn, ReciteLocaleQuery, ReciteLocaleRequestKind,
    ReciteLocaleResult, ReciteLocaleTextDomain, recite_locale_evaluate_plural_rule,
    recite_locale_validate_plural_rule, recite_locale_validate_translation_placeholders,
};
pub use session::{
    recite_session_acknowledge_effect, recite_session_begin, recite_session_choose,
    recite_session_clear_locale_provider, recite_session_create, recite_session_create_with_values,
    recite_session_free, recite_session_register_condition, recite_session_restore,
    recite_session_restore_with_values, recite_session_restore_with_values_and_locale_provider,
    recite_session_restore_with_values_and_locale_provider_and_variant,
    recite_session_set_interpolation_values, recite_session_set_locale_provider,
    recite_session_set_locale_variant, recite_session_snapshot, recite_session_start,
    recite_session_start_with_locale_provider,
    recite_session_start_with_locale_provider_and_variant, recite_session_start_with_values,
    recite_session_start_with_values_and_locale_provider,
    recite_session_start_with_values_and_locale_provider_and_variant,
};

/// ABI major version for the generated C header.
///
/// Increment this for breaking C ABI changes.
pub const RECITE_FFI_VERSION_MAJOR: u32 = 0;
/// ABI minor version for additive, backwards-compatible C ABI changes.
pub const RECITE_FFI_VERSION_MINOR: u32 = 5;
/// ABI patch version for documentation-only or implementation-only releases.
pub const RECITE_FFI_VERSION_PATCH: u32 = 0;
