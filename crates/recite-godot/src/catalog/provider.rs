use recite_core::LocaleId;
use recite_runtime::{
    LocaleError, LocaleProvider, PluralResolution, PluralResolutionAttempt,
    PluralResolutionOutcome, TextDomain,
};

use super::{CatalogKey, ReciteDialogueCatalog, contexts, gettext_context};

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
        for candidate_context in contexts(&context, variant) {
            for candidate_locale in locale_fallbacks(locale.as_str()) {
                if let Some(text) = self.lookup_context_for(
                    &candidate_locale,
                    domain,
                    &candidate_context,
                    source_text,
                ) {
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
        let mut attempts = Vec::new();
        for candidate_context in contexts(&context, variant) {
            for candidate_locale in locale_fallbacks(locale.as_str()) {
                let Some(header) = self.plural_forms.get(&candidate_locale) else {
                    attempts.push(attempt(
                        &candidate_locale,
                        &candidate_context,
                        id,
                        None,
                        PluralResolutionOutcome::MissingPluralForms,
                    ));
                    continue;
                };
                let arm = recite_core::evaluate_plural_form(header, count)
                    .map_err(|error| LocaleError::new(error.to_string()))?;
                let Some(entry) = self.plural_entry(
                    &candidate_locale,
                    domain,
                    &candidate_context,
                    source_singular,
                    source_plural,
                ) else {
                    attempts.push(attempt(
                        &candidate_locale,
                        &candidate_context,
                        id,
                        Some(arm),
                        PluralResolutionOutcome::MissingEntry,
                    ));
                    continue;
                };
                let Some(text) = entry.translations.get(arm).filter(|text| !text.is_empty()) else {
                    attempts.push(attempt(
                        &candidate_locale,
                        &candidate_context,
                        id,
                        Some(arm),
                        PluralResolutionOutcome::MissingTranslation,
                    ));
                    continue;
                };
                attempts.push(attempt(
                    &candidate_locale,
                    &candidate_context,
                    id,
                    Some(arm),
                    PluralResolutionOutcome::Matched,
                ));
                return Ok(PluralResolution {
                    template: Some(text.clone()),
                    selected_arm: Some(arm),
                    matched_locale: Some(candidate_locale),
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
}

impl ReciteDialogueCatalog {
    fn lookup_context_for(
        &self,
        locale: &str,
        domain: TextDomain,
        context: &str,
        source_text: &str,
    ) -> Option<String> {
        self.translations
            .get(&CatalogKey {
                locale: locale.to_owned(),
                domain,
                context: context.to_owned(),
                source_text: source_text.to_owned(),
                plural_source_text: None,
            })
            .and_then(|value| value.translations.first())
            .filter(|text| !text.is_empty())
            .cloned()
    }
}

fn locale_fallbacks(locale: &str) -> Vec<String> {
    let mut fallbacks = vec![locale.to_owned()];
    let mut current = locale;
    while let Some((parent, _)) = current.rsplit_once('-') {
        if parent.is_empty() {
            break;
        }
        fallbacks.push(parent.to_owned());
        current = parent;
    }
    fallbacks
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
