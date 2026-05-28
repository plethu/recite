use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use recite_core::LocaleId;
use recite_runtime::{LocaleProvider, TextDomain};

use super::po::parse_po_catalog;
use crate::error::CliError;

#[derive(Debug, Default)]
pub(crate) struct DialogueCatalogProvider {
    translations: BTreeMap<CatalogKey, String>,
}

impl DialogueCatalogProvider {
    pub(crate) fn load(catalogs: Vec<DialogueCatalogSource>) -> Result<Self, CliError> {
        let mut provider = Self::default();
        for catalog in catalogs {
            provider.load_catalog(catalog)?;
        }
        Ok(provider)
    }

    fn load_catalog(&mut self, catalog: DialogueCatalogSource) -> Result<(), CliError> {
        let source = fs::read_to_string(&catalog.path).map_err(|source| CliError::Read {
            path: catalog.path.clone(),
            source,
        })?;
        let entries = parse_po_catalog(&catalog.path, &source)?;

        for entry in entries {
            let key = CatalogKey {
                locale: catalog.locale.as_str().to_owned(),
                context: entry.context,
                source_text: entry.source_text,
            };
            if let Some(existing) = self.translations.get(&key) {
                if existing != &entry.translation {
                    return Err(CliError::DialogueCatalogConflict {
                        path: catalog.path,
                        locale: key.locale,
                        context: key.context,
                        source_text: key.source_text,
                    });
                }
                continue;
            }
            self.translations.insert(key, entry.translation);
        }

        Ok(())
    }

    fn lookup_context(&self, locale: &str, context: &str, source_text: &str) -> Option<String> {
        self.translations
            .get(&CatalogKey {
                locale: locale.to_owned(),
                context: context.to_owned(),
                source_text: source_text.to_owned(),
            })
            .filter(|translation| !translation.is_empty())
            .cloned()
    }
}

impl LocaleProvider for DialogueCatalogProvider {
    fn lookup(
        &self,
        id: &str,
        source_text: &str,
        _domain: TextDomain,
        locale: &LocaleId,
        variant: Option<&str>,
    ) -> Option<String> {
        for locale in locale_fallbacks(locale.as_str()) {
            if let Some(translation) = variant.and_then(|variant| {
                self.lookup_context(&locale, &format!("{id}&{variant}"), source_text)
            }) {
                return Some(translation);
            }
            if let Some(translation) = self.lookup_context(&locale, id, source_text) {
                return Some(translation);
            }
        }
        None
    }
}

pub(super) fn locale_fallbacks(locale: &str) -> Vec<String> {
    let mut fallbacks = vec![locale.to_owned()];
    let mut current = locale;
    while let Some((parent, _)) = current.rsplit_once('-') {
        if parent.is_empty() {
            break;
        }
        if !fallbacks.iter().any(|fallback| fallback == parent) {
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
}
