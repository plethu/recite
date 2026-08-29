mod plural;
mod provider;
mod types;

pub use plural::{
    recite_locale_evaluate_plural_rule, recite_locale_validate_plural_rule,
    recite_locale_validate_translation_placeholders,
};
pub(crate) use provider::FfiLocaleProvider;
pub use types::{
    RECITE_LOCALE_ATTEMPT_MATCHED, RECITE_LOCALE_ATTEMPT_MISSING_ENTRY,
    RECITE_LOCALE_ATTEMPT_MISSING_PLURAL_FORMS, RECITE_LOCALE_ATTEMPT_MISSING_TRANSLATION,
    RECITE_LOCALE_DOMAIN_AVAILABILITY_REASON, RECITE_LOCALE_DOMAIN_CHOICE,
    RECITE_LOCALE_DOMAIN_LINE, RECITE_LOCALE_DOMAIN_PRESENTATION_LABEL,
    RECITE_LOCALE_REQUEST_PLURAL, RECITE_LOCALE_REQUEST_SINGULAR, ReciteLocaleAttempt,
    ReciteLocaleAttemptOutcome, ReciteLocaleFn, ReciteLocaleQuery, ReciteLocaleRequestKind,
    ReciteLocaleResult, ReciteLocaleTextDomain,
};
