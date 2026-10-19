use recite_core::po::PoDocumentFingerprint;

use super::CatalogSummaryError;
use super::coverage::{CatalogEntryKey, CatalogEntryStatus};
use super::record_status::CatalogRecordStatus;
use super::resolution::{CatalogEntryResolution, CatalogResolution, CatalogResolutionPolicy};
use super::summary::CatalogCoverageSummary;
use super::types::{CatalogInput, CatalogSummary};
use super::validation::ValidatedInputs;
use recite_core::po::PotDocument;

impl CatalogCoverageSummary {
    /// Build a deterministic summary from an expected POT and lossless PO
    /// documents. Catalogues are sorted by explicit identity; duplicate
    /// identities and conflicting records are rejected, while multiple
    /// source documents may contribute records to the same explicit locale.
    pub fn build(
        expected: &PotDocument,
        catalogs: impl IntoIterator<Item = CatalogInput>,
        policy: CatalogResolutionPolicy,
    ) -> Result<Self, CatalogSummaryError> {
        let ValidatedInputs {
            expected_keys,
            catalogs,
            resolution,
        } = ValidatedInputs::new(expected, catalogs, policy)?;
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
}

impl CatalogSummary {
    pub(super) fn build(input: &CatalogInput, expected: &[CatalogEntryKey]) -> Self {
        let plural_forms = input
            .document()
            .headers()
            .iter()
            .find(|header| header.key().eq_ignore_ascii_case("Plural-Forms"))
            .and_then(|header| recite_core::po::validate_plural_rule(header.value()).ok());
        let entries = expected
            .iter()
            .map(|key| CatalogEntryStatus::build(key, input, plural_forms))
            .collect::<Vec<_>>();
        let records = input
            .document()
            .entries()
            .iter()
            .filter(|entry| !entry.is_header())
            .map(|entry| CatalogRecordStatus::build(entry, plural_forms))
            .collect::<Vec<_>>();
        let coverage =
            super::coverage::CatalogCoverage::from_entries(expected.len(), &entries, &records);
        Self {
            identity: input.identity.clone(),
            fingerprint: input.document().fingerprint(),
            plural_forms,
            coverage,
            entries,
            records,
        }
    }
}

fn expected_fingerprint(expected: &PotDocument) -> PoDocumentFingerprint {
    PoDocumentFingerprint::from_source(&expected.to_pot_string())
}

impl CatalogResolution {
    /// Validate expected entries and catalogues, then resolve locale/variant policy.
    ///
    /// Uses the same expected-entry, catalogue and policy validation order as
    /// [`CatalogCoverageSummary::build`], without constructing coverage rows or
    /// serializing a POT fingerprint. Multiple inputs are checked in identity order.
    pub fn build(
        expected: &PotDocument,
        catalogs: impl IntoIterator<Item = CatalogInput>,
        policy: CatalogResolutionPolicy,
    ) -> Result<Self, CatalogSummaryError> {
        Ok(ValidatedInputs::new(expected, catalogs, policy)?.resolution)
    }
}
