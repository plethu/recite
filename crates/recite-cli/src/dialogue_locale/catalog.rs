use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use recite_core::LocaleId;
use recite_runtime::localisation::{LocaleProvider, PluralResolution, TextDomain};

use super::po::parse_po_catalog;
use crate::error::CliError;

#[derive(Debug, Default)]
struct CatalogValidation {
    translations: BTreeMap<CatalogKey, CatalogValue>,
    plural_forms: BTreeMap<String, String>,
}

impl CatalogValidation {
    fn load_catalog(
        &mut self,
        catalog: &DialogueCatalogSource,
        source: &str,
    ) -> Result<(), CliError> {
        let parsed = parse_po_catalog(&catalog.path, source)?;
        if let Some(plural_forms) = parsed.plural_forms {
            let locale = catalog.locale.as_str().to_owned();
            if let Some(existing) = self.plural_forms.get(&locale)
                && existing != &plural_forms
            {
                return Err(CliError::DialogueCatalogPluralFormsConflict {
                    path: catalog.path.clone(),
                    locale,
                    existing: existing.clone(),
                    provided: plural_forms,
                });
            }
            self.plural_forms.insert(locale, plural_forms);
        }

        for entry in parsed.entries {
            let key = CatalogKey {
                locale: catalog.locale.as_str().to_owned(),
                context: entry.context,
                source_text: entry.source_text,
                plural_source_text: entry.plural_source_text,
            };
            let value = CatalogValue {
                translations: entry.translations,
            };
            if let Some(existing) = self.translations.get(&key) {
                if existing != &value {
                    return Err(CliError::DialogueCatalogConflict {
                        path: catalog.path.clone(),
                        locale: key.locale,
                        context: key.context,
                        source_text: key.source_text,
                    });
                }
                continue;
            }
            self.translations.insert(key, value);
        }

        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct DialogueCatalogProvider(recite_adapter::ReciteDialogueCatalog);

impl DialogueCatalogProvider {
    pub(crate) fn load(catalogs: Vec<DialogueCatalogSource>) -> Result<Self, CliError> {
        let mut validation = CatalogValidation::default();
        let mut captured = Vec::with_capacity(catalogs.len());
        for catalog in catalogs {
            let source = fs::read_to_string(&catalog.path).map_err(|source| CliError::Read {
                path: catalog.path.clone(),
                source,
            })?;
            validation.load_catalog(&catalog, &source)?;
            captured.push((catalog, source));
        }
        let mut provider = recite_adapter::ReciteDialogueCatalog::new();
        for (catalog, source) in captured {
            provider
                .import_po(
                    catalog.locale.as_str(),
                    &catalog.path.display().to_string(),
                    &source,
                )
                .map_err(|source| CliError::DialogueCatalogInvalid {
                    path: catalog.path,
                    source,
                })?;
        }
        Ok(Self(provider))
    }
}

impl LocaleProvider for DialogueCatalogProvider {
    fn lookup(
        &self,
        id: &str,
        source_text: &str,
        domain: TextDomain,
        locale: &LocaleId,
        variant: Option<&str>,
    ) -> Result<Option<String>, recite_runtime::localisation::LocaleError> {
        self.0.lookup(id, source_text, domain, locale, variant)
    }

    fn lookup_with_provenance(
        &self,
        id: &str,
        source_text: &str,
        domain: TextDomain,
        locale: &LocaleId,
        variant: Option<&str>,
    ) -> Result<
        recite_runtime::localisation::LocaleLookupProvenance,
        recite_runtime::localisation::LocaleError,
    > {
        self.0
            .lookup_with_provenance(id, source_text, domain, locale, variant)
    }

    fn resolve_plural(
        &self,
        id: &str,
        source_singular: &str,
        source_plural: &str,
        count: i64,
        domain: TextDomain,
        locale: &LocaleId,
        variant: Option<&str>,
    ) -> Result<PluralResolution, recite_runtime::localisation::LocaleError> {
        self.0.resolve_plural(
            id,
            source_singular,
            source_plural,
            count,
            domain,
            locale,
            variant,
        )
    }

    fn validated_plural_arm_count(
        &self,
        resolution: &PluralResolution,
    ) -> Result<Option<usize>, recite_runtime::localisation::LocaleError> {
        self.0.validated_plural_arm_count(resolution)
    }
}

pub(super) fn locale_fallbacks(locale: &str) -> Vec<String> {
    let mut fallbacks = vec![locale.to_owned()];
    let mut current = locale;
    while let Some((parent, _)) = current.rsplit_once('-') {
        if parent.is_empty() {
            break;
        }
        if parent.parse::<language_tags::LanguageTag>().is_ok()
            && !fallbacks.iter().any(|fallback| fallback == parent)
        {
            fallbacks.push(parent.to_owned());
        }
        current = parent;
    }
    fallbacks
}

#[derive(Clone, Debug)]
pub(crate) struct DialogueCatalogSource {
    pub(crate) locale: LocaleId,
    pub(crate) path: PathBuf,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CatalogKey {
    locale: String,
    context: String,
    source_text: String,
    plural_source_text: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CatalogValue {
    translations: Vec<String>,
}
