#[path = "policy.rs"]
mod policy;

use recite_core::{LocaleId, PoDocument, PoEntry};

use super::CatalogSummaryError;
use super::coverage::CatalogEntryKey;
use super::types::{CatalogIdentity, CatalogInput};
pub use policy::{CatalogResolutionPolicy, CatalogVariant};

/// A concrete locale/variant candidate in policy order.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub struct CatalogFallbackCandidate {
    locale: LocaleId,
    variant: CatalogVariant,
}

impl CatalogFallbackCandidate {
    #[must_use]
    pub const fn locale(&self) -> &LocaleId {
        &self.locale
    }

    #[must_use]
    pub const fn variant(&self) -> &CatalogVariant {
        &self.variant
    }
}

/// The policy and resulting ordered candidates shared by every expected entry.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct CatalogResolution {
    requested_locale: Option<LocaleId>,
    default_locale: Option<LocaleId>,
    candidates: Vec<CatalogFallbackCandidate>,
}

impl CatalogResolution {
    pub(super) fn new(policy: &CatalogResolutionPolicy) -> Result<Self, CatalogSummaryError> {
        policy.validate()?;
        let mut locales = Vec::new();
        if let Some(locale) = &policy.requested_locale {
            locales.push(locale.clone());
            if let Some(default) = &policy.default_locale {
                locales.push(default.clone());
            }
            locales.extend(policy.fallback_locales.iter().cloned());
        }
        let candidates = locales
            .into_iter()
            .flat_map(|locale| {
                policy
                    .variants
                    .iter()
                    .cloned()
                    .map(move |variant| CatalogFallbackCandidate {
                        locale: locale.clone(),
                        variant,
                    })
            })
            .collect::<Vec<_>>();
        Ok(Self {
            requested_locale: policy.requested_locale.clone(),
            default_locale: policy.default_locale.clone(),
            candidates,
        })
    }

    #[must_use]
    pub const fn requested_locale(&self) -> Option<&LocaleId> {
        self.requested_locale.as_ref()
    }

    #[must_use]
    pub const fn default_locale(&self) -> Option<&LocaleId> {
        self.default_locale.as_ref()
    }

    #[must_use]
    pub fn candidates(&self) -> &[CatalogFallbackCandidate] {
        &self.candidates
    }

    #[must_use]
    pub fn fallback_candidates(&self) -> &[CatalogFallbackCandidate] {
        self.candidates()
    }

    #[must_use]
    pub const fn is_source_only(&self) -> bool {
        self.requested_locale.is_none()
    }
}

/// The catalogue selected for one expected entry, when one is usable.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct CatalogMatch {
    catalog: CatalogIdentity,
    candidate: CatalogFallbackCandidate,
}

impl CatalogMatch {
    #[must_use]
    pub const fn catalog(&self) -> &CatalogIdentity {
        &self.catalog
    }

    #[must_use]
    pub const fn catalog_identity(&self) -> &CatalogIdentity {
        self.catalog()
    }

    #[must_use]
    pub const fn candidate(&self) -> &CatalogFallbackCandidate {
        &self.candidate
    }
}

/// Resolution metadata for one expected POT entry.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct CatalogEntryResolution {
    key: CatalogEntryKey,
    candidates: Vec<CatalogFallbackCandidate>,
    matched: Option<CatalogMatch>,
    source_fallback: bool,
}

impl CatalogEntryResolution {
    pub(super) fn build(
        key: &CatalogEntryKey,
        resolution: &CatalogResolution,
        catalogs: &[CatalogInput],
    ) -> Self {
        let matched = resolution.candidates.iter().find_map(|candidate| {
            let catalog = catalogs.iter().find(|catalog| {
                catalog.identity().locale() == candidate.locale()
                    && catalogs_entry_is_translated(catalog.document(), key, candidate.variant())
            })?;
            Some(CatalogMatch {
                catalog: catalog.identity().clone(),
                candidate: candidate.clone(),
            })
        });
        Self {
            key: key.clone(),
            candidates: resolution.candidates.clone(),
            source_fallback: matched.is_none(),
            matched,
        }
    }

    #[must_use]
    pub const fn key(&self) -> &CatalogEntryKey {
        &self.key
    }

    #[must_use]
    pub fn candidates(&self) -> &[CatalogFallbackCandidate] {
        &self.candidates
    }

    #[must_use]
    pub const fn matched(&self) -> Option<&CatalogMatch> {
        self.matched.as_ref()
    }

    #[must_use]
    pub const fn matched_catalog(&self) -> Option<&CatalogMatch> {
        self.matched()
    }

    #[must_use]
    pub const fn source_fallback(&self) -> bool {
        self.source_fallback
    }

    #[must_use]
    pub const fn used_source_fallback(&self) -> bool {
        self.source_fallback()
    }
}

fn catalogs_entry_is_translated(
    document: &PoDocument,
    key: &CatalogEntryKey,
    variant: &CatalogVariant,
) -> bool {
    let context = candidate_context(key.context(), variant);
    document.entries().iter().any(|entry| {
        key_matches(entry, key, &context)
            && !entry.is_obsolete()
            && !entry.flags().iter().any(|flag| flag == "fuzzy")
            && entry_translation_is_complete(entry, document)
    })
}

fn candidate_context(context: &str, variant: &CatalogVariant) -> String {
    let base = context.split_once('&').map_or(context, |(base, _)| base);
    match variant {
        CatalogVariant::Base => context.to_owned(),
        CatalogVariant::Named(name) => format!("{base}&{name}"),
    }
}

fn key_matches(entry: &PoEntry, key: &CatalogEntryKey, context: &str) -> bool {
    !entry.is_header()
        && entry.context() == Some(context)
        && entry.source_text() == key.source_text()
        && entry.plural_source_text() == key.plural_source_text()
}

fn entry_translation_is_complete(entry: &PoEntry, document: &PoDocument) -> bool {
    if entry.is_plural() {
        let Some(expected_arms) = document
            .headers()
            .iter()
            .find(|header| header.key().eq_ignore_ascii_case("Plural-Forms"))
            .and_then(|header| recite_core::validate_plural_rule(header.value()).ok())
        else {
            return false;
        };
        entry.plural_translations().len() == expected_arms
            && entry
                .plural_translations()
                .iter()
                .all(|translation| !translation.text().trim().is_empty())
    } else {
        entry
            .translation()
            .is_some_and(|translation| !translation.trim().is_empty())
    }
}
