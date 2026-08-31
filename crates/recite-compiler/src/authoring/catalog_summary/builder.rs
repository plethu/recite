use std::collections::BTreeSet;

use recite_core::PoDocumentFingerprint;

use super::CatalogSummaryError;
use super::coverage::{CatalogEntryKey, CatalogEntryStatus};
use super::resolution::{CatalogEntryResolution, CatalogResolution, CatalogResolutionPolicy};
use super::types::{CatalogCoverageSummary, CatalogInput, CatalogSummary};
use crate::PotDocument;

impl CatalogCoverageSummary {
    /// Build a deterministic summary from an expected POT and lossless PO
    /// documents. Catalogues are sorted by explicit identity; duplicate
    /// identities or locales are rejected because they would make a match
    /// ambiguous.
    pub fn build(
        expected: &PotDocument,
        catalogs: impl IntoIterator<Item = CatalogInput>,
        policy: CatalogResolutionPolicy,
    ) -> Result<Self, CatalogSummaryError> {
        let expected_keys = expected
            .entries
            .iter()
            .map(CatalogEntryKey::from_pot_entry)
            .collect::<Result<Vec<_>, _>>()?;
        let mut expected_seen = BTreeSet::new();
        for key in &expected_keys {
            if !expected_seen.insert((key.context().to_owned(), key.source_text().to_owned())) {
                return Err(CatalogSummaryError::DuplicateExpectedEntry {
                    context: key.context().to_owned(),
                    source_text: key.source_text().to_owned(),
                });
            }
        }

        let mut catalogs = catalogs.into_iter().collect::<Vec<_>>();
        catalogs.sort_by(|left, right| left.identity.cmp(&right.identity));
        for pair in catalogs.windows(2) {
            if pair[0].identity == pair[1].identity {
                return Err(CatalogSummaryError::DuplicateCatalog {
                    identity: pair[0].identity.clone(),
                });
            }
            if pair[0].identity.locale() == pair[1].identity.locale() {
                return Err(CatalogSummaryError::DuplicateCatalogLocale {
                    locale: pair[0].identity.locale().clone(),
                });
            }
        }

        let resolution = CatalogResolution::new(&policy)?;
        let summaries = catalogs
            .iter()
            .map(|catalog| CatalogSummary::build(catalog, &expected_keys))
            .collect::<Vec<_>>();
        let entries = expected_keys
            .iter()
            .map(|key| CatalogEntryResolution::build(key, &resolution, &catalogs))
            .collect::<Vec<_>>();

        Ok(Self {
            expected_fingerprint: expected_fingerprint(expected),
            expected_count: expected_keys.len(),
            catalogs: summaries,
            resolution,
            entries,
        })
    }

    /// Alias for [`Self::build`] that reads naturally at authoring call sites.
    pub fn from_documents(
        expected: &PotDocument,
        catalogs: impl IntoIterator<Item = CatalogInput>,
        policy: CatalogResolutionPolicy,
    ) -> Result<Self, CatalogSummaryError> {
        Self::build(expected, catalogs, policy)
    }
}

impl CatalogSummary {
    pub(super) fn build(input: &CatalogInput, expected: &[CatalogEntryKey]) -> Self {
        let plural_forms = input
            .document
            .headers()
            .iter()
            .find(|header| header.key().eq_ignore_ascii_case("Plural-Forms"))
            .and_then(|header| recite_core::validate_plural_rule(header.value()).ok());
        let entries = expected
            .iter()
            .map(|key| CatalogEntryStatus::build(key, &input.document, plural_forms))
            .collect::<Vec<_>>();
        let coverage = super::coverage::CatalogCoverage::from_entries(
            expected.len(),
            &entries,
            input.document.entries(),
        );
        Self {
            identity: input.identity.clone(),
            fingerprint: input.document.fingerprint(),
            plural_forms,
            coverage,
            entries,
        }
    }
}

fn expected_fingerprint(expected: &PotDocument) -> PoDocumentFingerprint {
    PoDocumentFingerprint::from_source(&expected.to_pot_string())
}
