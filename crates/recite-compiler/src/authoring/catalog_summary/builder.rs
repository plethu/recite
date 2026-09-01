use std::collections::{BTreeMap, BTreeSet};

use recite_core::{LocaleId, PoDocumentFingerprint, PoEntry};

use super::CatalogSummaryError;
use super::coverage::{CatalogEntryKey, CatalogEntryStatus};
use super::locale::declared_language;
use super::record_status::CatalogRecordStatus;
use super::resolution::{CatalogEntryResolution, CatalogResolution, CatalogResolutionPolicy};
use super::summary::CatalogCoverageSummary;
use super::types::{CatalogIdentity, CatalogInput, CatalogSummary};
use crate::PotDocument;

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
        validate_catalogs(&catalogs)?;

        let resolution = CatalogResolution::new(&policy)?;
        let summaries = catalogs
            .iter()
            .map(|catalog| CatalogSummary::build(catalog, &expected_keys))
            .collect::<Result<Vec<_>, _>>()?;
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
    pub(super) fn build(
        input: &CatalogInput,
        expected: &[CatalogEntryKey],
    ) -> Result<Self, CatalogSummaryError> {
        declared_language(&input.document, &input.identity)?;
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
        let records = input
            .document
            .entries()
            .iter()
            .filter(|entry| !entry.is_header())
            .map(|entry| CatalogRecordStatus::build(entry, plural_forms))
            .collect::<Vec<_>>();
        let coverage =
            super::coverage::CatalogCoverage::from_entries(expected.len(), &entries, &records);
        Ok(Self {
            identity: input.identity.clone(),
            fingerprint: input.document.fingerprint(),
            plural_forms,
            coverage,
            entries,
            records,
        })
    }
}

fn expected_fingerprint(expected: &PotDocument) -> PoDocumentFingerprint {
    PoDocumentFingerprint::from_source(&expected.to_pot_string())
}

fn validate_catalogs(catalogs: &[CatalogInput]) -> Result<(), CatalogSummaryError> {
    let mut identities = BTreeSet::new();
    let mut plural_forms: BTreeMap<LocaleId, (CatalogIdentity, String)> = BTreeMap::new();
    let mut records = BTreeMap::new();
    for catalog in catalogs {
        declared_language(&catalog.document, &catalog.identity)?;
        if !identities.insert(catalog.identity.clone()) {
            return Err(CatalogSummaryError::DuplicateCatalog {
                identity: catalog.identity.clone(),
            });
        }
        if let Some(header) = catalog
            .document
            .headers()
            .iter()
            .find(|header| header.key().eq_ignore_ascii_case("Plural-Forms"))
        {
            let locale = catalog.identity.locale().clone();
            let provided = header.value().to_owned();
            if let Some((first_identity, first)) = plural_forms.get(&locale)
                && first != &provided
            {
                return Err(CatalogSummaryError::CatalogPluralFormsConflict {
                    first: Box::new(first_identity.clone()),
                    second: Box::new(catalog.identity.clone()),
                    locale,
                    first_forms: first.clone().into_boxed_str(),
                    second_forms: provided.into_boxed_str(),
                });
            }
            plural_forms
                .entry(locale)
                .or_insert_with(|| (catalog.identity.clone(), provided));
        }
        for entry in catalog.document.entries().iter().filter(|entry| {
            !entry.is_header()
                && !entry.is_obsolete()
                && !entry.flags().iter().any(|flag| flag == "fuzzy")
        }) {
            validate_record(&mut records, catalog, entry)?;
        }
    }
    Ok(())
}

fn validate_record(
    records: &mut BTreeMap<(LocaleId, CatalogRecordKey), (CatalogIdentity, Vec<String>)>,
    catalog: &CatalogInput,
    entry: &PoEntry,
) -> Result<(), CatalogSummaryError> {
    let Some(context) = entry.context() else {
        return Ok(());
    };
    let key = (
        catalog.identity.locale().clone(),
        CatalogRecordKey {
            context: context.to_owned(),
            source_text: entry.source_text().to_owned(),
            plural_source_text: entry.plural_source_text().map(str::to_owned),
        },
    );
    let translations = entry_translations(entry);
    if let Some((first_identity, first_translations)) = records.get(&key) {
        if first_translations != &translations {
            return Err(CatalogSummaryError::CatalogEntryConflict {
                first: Box::new(first_identity.clone()),
                second: Box::new(catalog.identity.clone()),
                context: key.1.context.clone(),
                source_text: key.1.source_text.clone(),
            });
        }
    } else {
        records.insert(key, (catalog.identity.clone(), translations));
    }
    Ok(())
}

fn entry_translations(entry: &PoEntry) -> Vec<String> {
    if entry.is_plural() {
        entry
            .plural_translations()
            .iter()
            .map(|translation| translation.text().to_owned())
            .collect()
    } else {
        vec![entry.translation().unwrap_or_default().to_owned()]
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CatalogRecordKey {
    context: String,
    source_text: String,
    plural_source_text: Option<String>,
}
