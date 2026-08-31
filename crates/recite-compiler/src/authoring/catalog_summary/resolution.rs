#[path = "policy.rs"]
mod policy;
#[path = "resolution_candidates.rs"]
mod resolution_candidates;

use recite_core::{PoDocument, PoEntry};

use super::CatalogSummaryError;
use super::coverage::CatalogEntryKey;
use super::types::{CatalogIdentity, CatalogInput};
pub use policy::{CatalogResolutionPolicy, CatalogVariant};
pub use resolution_candidates::{CatalogFallbackCandidate, CatalogResolution};

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
        let matched = resolution.candidates().iter().find_map(|candidate| {
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
            candidates: resolution.candidates().to_vec(),
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
