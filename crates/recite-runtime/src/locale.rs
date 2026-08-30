use std::collections::BTreeMap;

use recite_core::{LocaleId, ScalarValue};

/// Localisable text category supplied to runtime locale providers.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TextDomain {
    Line,
    Choice,
    AvailabilityReason,
    PresentationLabel,
}

/// Structured failure from a locale provider.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("locale provider failed: {reason}")]
pub struct LocaleError {
    reason: String,
}

impl LocaleError {
    #[must_use]
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }

    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

/// Typed caller-owned values supplied to interpolation at delivery time.
pub type InterpolationValues = BTreeMap<String, ScalarValue>;

/// Outcome recorded for one candidate gettext lookup key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PluralResolutionOutcome {
    MissingPluralForms,
    MissingEntry,
    MissingTranslation,
    Matched,
}

/// Outcome for one ordinary singular lookup candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocaleLookupOutcome {
    MissingEntry,
    Matched,
}

/// One candidate considered by an ordinary locale lookup.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocaleLookupAttempt {
    pub locale: String,
    pub context: String,
    pub key: String,
    pub outcome: LocaleLookupOutcome,
}

impl LocaleLookupAttempt {
    #[must_use]
    pub fn new(
        locale: impl Into<String>,
        context: impl Into<String>,
        key: impl Into<String>,
        outcome: LocaleLookupOutcome,
    ) -> Self {
        Self {
            locale: locale.into(),
            context: context.into(),
            key: key.into(),
            outcome,
        }
    }
}

/// Structured ordinary lookup result. Existing providers can keep
/// implementing [`LocaleProvider::lookup`]; providers that know their real
/// fallback chain may override [`LocaleProvider::lookup_with_provenance`].
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocaleLookupProvenance {
    pub template: Option<String>,
    pub matched_locale: Option<String>,
    pub matched_context: Option<String>,
    pub matched_key: Option<String>,
    pub attempts: Vec<LocaleLookupAttempt>,
}

impl LocaleLookupProvenance {
    #[must_use]
    pub fn new(template: Option<String>) -> Self {
        Self {
            template,
            matched_locale: None,
            matched_context: None,
            matched_key: None,
            attempts: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_match(
        mut self,
        locale: impl Into<String>,
        context: impl Into<String>,
        key: impl Into<String>,
    ) -> Self {
        self.matched_locale = Some(locale.into());
        self.matched_context = Some(context.into());
        self.matched_key = Some(key.into());
        self
    }

    #[must_use]
    pub fn with_attempts(mut self, attempts: Vec<LocaleLookupAttempt>) -> Self {
        self.attempts = attempts;
        self
    }
}

/// One deterministic candidate considered while resolving a plural entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluralResolutionAttempt {
    pub locale: String,
    pub context: String,
    /// Stable source ID for the attempted entry. The context identifies the
    /// variant/base lookup form while this key remains independently usable by
    /// host diagnostics and adapters.
    pub key: String,
    pub selected_arm: Option<usize>,
    pub outcome: PluralResolutionOutcome,
}

/// Structured result of one plural lookup.
///
/// `template` is populated only for a matching catalogue entry. The selected
/// arm is likewise only authoritative when a translation matched; callers
/// must use their source-language fallback for an unresolved entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluralResolution {
    pub template: Option<String>,
    pub selected_arm: Option<usize>,
    pub matched_locale: Option<String>,
    pub matched_context: Option<String>,
    pub matched_key: Option<String>,
    pub attempts: Vec<PluralResolutionAttempt>,
}

/// Host-owned lookup for named interpolation values.
pub trait InterpolationValueProvider {
    fn lookup_value(&self, name: &str) -> Result<Option<ScalarValue>, LocaleError>;
}

impl InterpolationValueProvider for InterpolationValues {
    fn lookup_value(&self, name: &str) -> Result<Option<ScalarValue>, LocaleError> {
        Ok(self.get(name).cloned())
    }
}

/// Runtime translation lookup surface.
///
/// Recite passes both the stable localisable ID and source text to support
/// gettext-style providers. When `variant` is present, providers should
/// exhaust the locale fallback chain for the variant-specific lookup key
/// before trying their non-variant key, then return `None` when no translation
/// exists so the runtime can fall back to `source_text`.
pub trait LocaleProvider {
    fn lookup(
        &self,
        id: &str,
        source_text: &str,
        domain: TextDomain,
        locale: &LocaleId,
        variant: Option<&str>,
    ) -> Result<Option<String>, LocaleError>;

    /// Performs an ordinary lookup while optionally returning the provider's
    /// actual ordered fallback attempts. The default preserves the original
    /// lookup contract and intentionally leaves provenance empty rather than
    /// inventing a fallback chain.
    fn lookup_with_provenance(
        &self,
        id: &str,
        source_text: &str,
        domain: TextDomain,
        locale: &LocaleId,
        variant: Option<&str>,
    ) -> Result<LocaleLookupProvenance, LocaleError> {
        self.lookup(id, source_text, domain, locale, variant)
            .map(|template| LocaleLookupProvenance {
                template,
                matched_locale: None,
                matched_context: None,
                matched_key: None,
                attempts: Vec::new(),
            })
    }

    /// Resolves one gettext plural entry in a single provider call.
    /// Implementations must validate each candidate locale's `Plural-Forms`
    /// header and return ordered attempts, including the selected arm and
    /// outcome for each candidate key. `selected_arm` is authoritative only
    /// when `template` is present and the matching entry supplied it.
    // The explicit arguments mirror gettext's stable lookup tuple and keep
    // source forms, count, domain, locale, and variant independently visible
    // to host providers. This public shape is the §9.7 compatibility contract.
    #[expect(
        clippy::too_many_arguments,
        reason = "the public plural provider contract keeps gettext tuple fields explicit"
    )]
    fn resolve_plural(
        &self,
        id: &str,
        source_singular: &str,
        source_plural: &str,
        count: i64,
        domain: TextDomain,
        locale: &LocaleId,
        variant: Option<&str>,
    ) -> Result<PluralResolution, LocaleError>;
}
