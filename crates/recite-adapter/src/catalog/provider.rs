use recite_core::LocaleId;
use recite_runtime::localisation::{
    LocaleError, LocaleProvider, PluralResolution, PluralResolutionAttempt,
    PluralResolutionOutcome, TextDomain,
};

use super::{CatalogKey, ReciteDialogueCatalog, contexts, gettext_context, valid_locale};

impl LocaleProvider for ReciteDialogueCatalog {
    fn lookup(
        &self,
        id: &str,
        source_text: &str,
        domain: TextDomain,
        locale: &LocaleId,
        variant: Option<&str>,
    ) -> Result<Option<String>, LocaleError> {
        let context = gettext_context(id, domain);
        let fallbacks = locale_fallbacks(locale)?;
        for candidate_context in contexts(&context, variant) {
            for candidate_locale in &fallbacks {
                if let Some(text) =
                    self.lookup_context_for(candidate_locale, &candidate_context, source_text)
                {
                    return Ok(Some(text));
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
    ) -> Result<PluralResolution, LocaleError> {
        let context = gettext_context(id, domain);
        let fallbacks = locale_fallbacks(locale)?;
        let mut attempts = Vec::new();
        for candidate_context in contexts(&context, variant) {
            for candidate_locale in &fallbacks {
                let Some(header) = self.plural_forms.get(candidate_locale) else {
                    attempts.push(attempt(
                        candidate_locale,
                        &candidate_context,
                        id,
                        None,
                        PluralResolutionOutcome::MissingPluralForms,
                    ));
                    continue;
                };
                let arm = recite_core::po::evaluate_plural_form(header, count)
                    .map_err(|error| LocaleError::new(error.to_string()))?;
                let Some(entry) = self.plural_entry(
                    candidate_locale,
                    &candidate_context,
                    source_singular,
                    source_plural,
                ) else {
                    attempts.push(attempt(
                        candidate_locale,
                        &candidate_context,
                        id,
                        Some(arm),
                        PluralResolutionOutcome::MissingEntry,
                    ));
                    continue;
                };
                let Some(text) = entry.translations.get(arm).filter(|text| !text.is_empty()) else {
                    attempts.push(attempt(
                        candidate_locale,
                        &candidate_context,
                        id,
                        Some(arm),
                        PluralResolutionOutcome::MissingTranslation,
                    ));
                    continue;
                };
                attempts.push(attempt(
                    candidate_locale,
                    &candidate_context,
                    id,
                    Some(arm),
                    PluralResolutionOutcome::Matched,
                ));
                return Ok(PluralResolution {
                    template: Some(text.clone()),
                    selected_arm: Some(arm),
                    matched_locale: Some(candidate_locale.clone()),
                    matched_context: Some(candidate_context),
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
    ) -> Result<Option<usize>, LocaleError> {
        let Some(locale) = resolution.matched_locale.as_deref() else {
            return Ok(None);
        };
        self.plural_forms
            .get(locale)
            .map(|header| {
                recite_core::po::validate_plural_rule(header)
                    .map(Some)
                    .map_err(|error| LocaleError::new(error.to_string()))
            })
            .unwrap_or(Ok(None))
    }
}

impl ReciteDialogueCatalog {
    fn lookup_context_for(&self, locale: &str, context: &str, source_text: &str) -> Option<String> {
        self.translations
            .get(&CatalogKey {
                locale: locale.to_owned(),
                context: context.to_owned(),
                source_text: source_text.to_owned(),
                plural_source_text: None,
            })
            .and_then(|value| value.translations.first())
            .filter(|text| !text.is_empty())
            .cloned()
    }
}

fn locale_fallbacks(locale: &LocaleId) -> Result<Vec<String>, LocaleError> {
    let canonical =
        valid_locale(locale.as_str()).map_err(|error| LocaleError::new(error.to_string()))?;
    let mut fallbacks = vec![canonical.as_str().to_owned()];
    let mut current = canonical.as_str();
    while let Some((parent, _)) = current.rsplit_once('-') {
        if let Ok(parent) = valid_locale(parent)
            && !fallbacks.iter().any(|fallback| fallback == parent.as_str())
        {
            fallbacks.push(parent.as_str().to_owned());
        }
        current = parent;
    }
    Ok(fallbacks)
}

fn attempt(
    locale: &str,
    context: &str,
    key: &str,
    selected_arm: Option<usize>,
    outcome: PluralResolutionOutcome,
) -> PluralResolutionAttempt {
    PluralResolutionAttempt {
        locale: locale.to_owned(),
        context: context.to_owned(),
        key: key.to_owned(),
        selected_arm,
        outcome,
    }
}
