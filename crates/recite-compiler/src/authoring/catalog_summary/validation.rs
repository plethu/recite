//! Shared validation prefix for full coverage and resolution-only consumers.
use super::{
    CatalogSummaryError,
    coverage::CatalogEntryKey,
    locale::declared_language,
    resolution::{CatalogResolution, CatalogResolutionPolicy},
    types::{CatalogIdentity, CatalogInput},
};
use recite_core::{
    LocaleId,
    po::{PoEntry, PotDocument},
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct ValidatedInputs {
    pub(super) expected_keys: Vec<CatalogEntryKey>,
    pub(super) catalogs: Vec<CatalogInput>,
    pub(super) resolution: CatalogResolution,
}
impl ValidatedInputs {
    pub(super) fn new(
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
        Ok(Self {
            expected_keys,
            catalogs,
            resolution,
        })
    }
}

fn validate_catalogs(catalogs: &[CatalogInput]) -> Result<(), CatalogSummaryError> {
    // Parsed PO records already have unique active durable keys. Identity,
    // plural-header and translation conflicts require multiple input files.
    if let [catalog] = catalogs {
        declared_language(catalog.document(), &catalog.identity)?;
        return Ok(());
    }
    let mut identities = BTreeSet::new();
    let mut plural_forms: BTreeMap<LocaleId, (CatalogIdentity, String)> = BTreeMap::new();
    let mut records = BTreeMap::new();
    for catalog in catalogs {
        declared_language(catalog.document(), &catalog.identity)?;
        if !identities.insert(catalog.identity.clone()) {
            return Err(CatalogSummaryError::DuplicateCatalog {
                identity: catalog.identity.clone(),
            });
        }
        if let Some(header) = catalog
            .document()
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
        for entry in catalog.document().entries().iter().filter(|entry| {
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
