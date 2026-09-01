use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use recite_core::LocaleId;
use recite_runtime::{
    LocaleProvider, PluralResolution, PluralResolutionAttempt, PluralResolutionOutcome, TextDomain,
};

use super::po::parse_po_catalog;
use crate::error::CliError;

#[derive(Debug, Default)]
pub(crate) struct DialogueCatalogProvider {
    translations: BTreeMap<CatalogKey, CatalogValue>,
    plural_forms: BTreeMap<String, String>,
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
        let parsed = parse_po_catalog(&catalog.path, &source)?;
        if let Some(plural_forms) = parsed.plural_forms {
            let locale = catalog.locale.as_str().to_owned();
            if let Some(existing) = self.plural_forms.get(&locale)
                && existing != &plural_forms
            {
                return Err(CliError::DialogueCatalogPluralFormsConflict {
                    path: catalog.path,
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
                        path: catalog.path,
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

    fn lookup_context(&self, locale: &str, context: &str, source_text: &str) -> Option<String> {
        self.translations
            .get(&CatalogKey {
                locale: locale.to_owned(),
                context: context.to_owned(),
                source_text: source_text.to_owned(),
                plural_source_text: None,
            })
            .and_then(|value| value.translations.first())
            .filter(|translation| !translation.is_empty())
            .cloned()
    }

    fn plural_entry(
        &self,
        locale: &str,
        context: &str,
        source_singular: &str,
        source_plural: &str,
    ) -> Option<&CatalogValue> {
        self.translations.get(&CatalogKey {
            locale: locale.to_owned(),
            context: context.to_owned(),
            source_text: source_singular.to_owned(),
            plural_source_text: Some(source_plural.to_owned()),
        })
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
    ) -> Result<Option<String>, recite_runtime::LocaleError> {
        let context = gettext_context(id, domain);
        let locales = locale_fallbacks(locale.as_str());
        for candidate_context in gettext_contexts(&context, variant) {
            for candidate_locale in &locales {
                if let Some(translation) =
                    self.lookup_context(candidate_locale, &candidate_context, source_text)
                {
                    return Ok(Some(translation));
                }
            }
        }
        Ok(None)
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
    ) -> Result<PluralResolution, recite_runtime::LocaleError> {
        let context = gettext_context(id, domain);
        let mut attempts = Vec::new();
        let locales = locale_fallbacks(locale.as_str());
        for candidate_context in gettext_contexts(&context, variant) {
            for candidate in &locales {
                let Some(header) = self.plural_forms.get(candidate) else {
                    attempts.push(PluralResolutionAttempt {
                        locale: candidate.clone(),
                        context: candidate_context.clone(),
                        key: id.to_owned(),
                        selected_arm: None,
                        outcome: PluralResolutionOutcome::MissingPluralForms,
                    });
                    continue;
                };
                let arm = recite_core::evaluate_plural_form(header, count)
                    .map_err(|error| recite_runtime::LocaleError::new(error.to_string()))?;
                let Some(entry) = self.plural_entry(
                    candidate,
                    &candidate_context,
                    source_singular,
                    source_plural,
                ) else {
                    attempts.push(PluralResolutionAttempt {
                        locale: candidate.clone(),
                        context: candidate_context.clone(),
                        key: id.to_owned(),
                        selected_arm: Some(arm),
                        outcome: PluralResolutionOutcome::MissingEntry,
                    });
                    continue;
                };
                let Some(translation) = entry.translations.get(arm) else {
                    attempts.push(PluralResolutionAttempt {
                        locale: candidate.clone(),
                        context: candidate_context.clone(),
                        key: id.to_owned(),
                        selected_arm: Some(arm),
                        outcome: PluralResolutionOutcome::MissingTranslation,
                    });
                    continue;
                };
                if translation.is_empty() {
                    attempts.push(PluralResolutionAttempt {
                        locale: candidate.clone(),
                        context: candidate_context.clone(),
                        key: id.to_owned(),
                        selected_arm: Some(arm),
                        outcome: PluralResolutionOutcome::MissingTranslation,
                    });
                    continue;
                }
                let matched_context = candidate_context.clone();
                attempts.push(PluralResolutionAttempt {
                    locale: candidate.clone(),
                    context: candidate_context.clone(),
                    key: id.to_owned(),
                    selected_arm: Some(arm),
                    outcome: PluralResolutionOutcome::Matched,
                });
                return Ok(PluralResolution {
                    template: Some(translation.clone()),
                    selected_arm: Some(arm),
                    matched_locale: Some(candidate.clone()),
                    matched_context: Some(matched_context),
                    matched_key: Some(id.to_owned()),
                    attempts,
                });
            }
        }
        Ok(PluralResolution {
            template: None,
            selected_arm: None,
            matched_locale: None,
            matched_context: None,
            matched_key: None,
            attempts,
        })
    }

    fn validated_plural_arm_count(
        &self,
        resolution: &PluralResolution,
    ) -> Result<Option<usize>, recite_runtime::LocaleError> {
        let Some(locale) = resolution.matched_locale.as_deref() else {
            return Ok(None);
        };
        self.plural_forms
            .get(locale)
            .map(|header| {
                recite_core::validate_plural_rule(header)
                    .map(Some)
                    .map_err(|error| recite_runtime::LocaleError::new(error.to_string()))
            })
            .unwrap_or(Ok(None))
    }
}

fn gettext_contexts(context: &str, variant: Option<&str>) -> Vec<String> {
    variant
        .map(|variant| format!("{context}&{variant}"))
        .into_iter()
        .chain(std::iter::once(context.to_owned()))
        .collect()
}

fn gettext_context(id: &str, domain: TextDomain) -> String {
    match domain {
        TextDomain::Line | TextDomain::Choice => id.to_owned(),
        TextDomain::AvailabilityReason => format!("availability_reason:{id}"),
        TextDomain::PresentationLabel => format!("presentation_label:{id}"),
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
    plural_source_text: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CatalogValue {
    translations: Vec<String>,
}
