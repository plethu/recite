//! C ABI surface for non-Rust engine adapters.
//!
//! Design decisions are documented in `docs/c-abi-boundary-design.md`.
//! Normative adapter semantics are in `docs/engine-adapter-contract.md`.

mod asset;
mod buffer;
mod catalog;
mod condition;
mod condition_codec;
mod error;
mod interpolation;
mod locale;
mod output;
mod session;
mod tagged_value;

pub use asset::{recite_asset_free, recite_asset_info, recite_asset_load};
pub use buffer::{ReciteBuffer, recite_buffer_free};
pub use catalog::{
    recite_catalog_add_po, recite_catalog_create, recite_catalog_free, recite_session_set_catalog,
};
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
    recite_session_clear_locale_provider, recite_session_create, recite_session_free,
    recite_session_prepare_restore, recite_session_register_condition, recite_session_restore,
    recite_session_set_interpolation_values, recite_session_set_locale_provider,
    recite_session_set_locale_variant, recite_session_snapshot, recite_session_start,
};

/// ABI major version for the generated C header.
///
/// Stable ABI families use major bumps for breaking changes. The current
/// unreleased 0.x family versions interface changes through its minor revision.
pub const RECITE_FFI_VERSION_MAJOR: u32 = 0;
/// Pre-release ABI revision; hosts must ship matching headers and libraries.
pub const RECITE_FFI_VERSION_MINOR: u32 = 7;
/// ABI patch version for documentation-only or implementation-only releases.
pub const RECITE_FFI_VERSION_PATCH: u32 = 0;
