//! Immutable PO inputs for one trial. Files and editable drafts stay with the host.
use recite_compiler::{
    CatalogCoverageSummary, CatalogInput, CatalogResolution, CatalogResolutionPolicy, PotDocument,
};
use recite_core::{LocaleId, PoEntry};
use recite_runtime::{
    LocaleError, LocaleLookupAttempt, LocaleLookupOutcome, LocaleLookupProvenance, LocaleProvider,
    PluralResolution, PluralResolutionAttempt, PluralResolutionOutcome, TextDomain,
};

pub(super) struct TrialCatalogues {
    inputs: Vec<CatalogInput>,
    resolution: CatalogResolution,
}
impl TrialCatalogues {
    pub fn new(
        mut inputs: Vec<CatalogInput>,
        policy: &CatalogResolutionPolicy,
        expected: &PotDocument,
    ) -> Result<Self, LocaleError> {
        let summary = CatalogCoverageSummary::build(expected, inputs.clone(), policy.clone())
            .map_err(|e| LocaleError::new(e.to_string()))?;
        inputs.sort_by(|a, b| a.identity().cmp(b.identity()));
        Ok(Self {
            inputs,
            resolution: summary.resolution().clone(),
        })
    }
    fn entry(
        &self,
        locale: &LocaleId,
        context: &str,
        source: &str,
        plural: Option<&str>,
    ) -> Option<(&PoEntry, &CatalogInput)> {
        self.inputs
            .iter()
            .filter(|input| input.identity().locale() == locale)
            .flat_map(|input| {
                input
                    .document()
                    .entries()
                    .iter()
                    .map(move |entry| (entry, input))
            })
            .find(|(entry, _)| {
                !entry.is_obsolete()
                    && entry.context() == Some(context)
                    && entry.source_text() == source
                    && entry.plural_source_text() == plural
                    && !entry.flags().iter().any(|flag| flag == "fuzzy")
            })
    }
    fn rule(&self, locale: &LocaleId) -> Option<&str> {
        self.inputs
            .iter()
            .filter(|input| input.identity().locale() == locale)
            .find_map(plural_rule)
    }
}
impl LocaleProvider for TrialCatalogues {
    fn lookup(
        &self,
        id: &str,
        source: &str,
        domain: TextDomain,
        locale: &LocaleId,
        variant: Option<&str>,
    ) -> Result<Option<String>, LocaleError> {
        Ok(self
            .lookup_with_provenance(id, source, domain, locale, variant)?
            .template)
    }
    fn lookup_with_provenance(
        &self,
        id: &str,
        source: &str,
        domain: TextDomain,
        _: &LocaleId,
        _: Option<&str>,
    ) -> Result<LocaleLookupProvenance, LocaleError> {
        let base = context(id, domain);
        let mut attempts = Vec::new();
        for candidate in self.resolution.candidates() {
            let context = candidate
                .variant()
                .name()
                .map_or_else(|| base.clone(), |v| format!("{base}&{v}"));
            let translation = self
                .entry(candidate.locale(), &context, source, None)
                .and_then(|(entry, _)| entry.translation())
                .filter(|t| !t.is_empty());
            attempts.push(LocaleLookupAttempt::new(
                candidate.locale().as_str(),
                &context,
                id,
                if translation.is_some() {
                    LocaleLookupOutcome::Matched
                } else {
                    LocaleLookupOutcome::MissingEntry
                },
            ));
            if let Some(translation) = translation {
                return Ok(LocaleLookupProvenance::new(Some(translation.into()))
                    .with_match(candidate.locale().as_str(), context, id)
                    .with_attempts(attempts));
            }
        }
        Ok(LocaleLookupProvenance::new(None).with_attempts(attempts))
    }
    fn resolve_plural(
        &self,
        id: &str,
        source: &str,
        plural: &str,
        count: i64,
        domain: TextDomain,
        _: &LocaleId,
        _: Option<&str>,
    ) -> Result<PluralResolution, LocaleError> {
        let base = context(id, domain);
        let mut result = PluralResolution {
            template: None,
            selected_arm: None,
            matched_locale: None,
            matched_context: None,
            matched_key: None,
            attempts: Vec::new(),
        };
        for candidate in self.resolution.candidates() {
            let context = candidate
                .variant()
                .name()
                .map_or_else(|| base.clone(), |v| format!("{base}&{v}"));
            let matched = self.entry(candidate.locale(), &context, source, Some(plural));
            let rule = matched.and_then(|(_, input)| plural_rule(input));
            let arm = rule.and_then(|rule| recite_core::evaluate_plural_form(rule, count).ok());
            let entry = matched.map(|(entry, _)| entry);
            let translation = arm
                .and_then(|arm| entry?.plural_translations().get(arm))
                .map(|arm| arm.text())
                .filter(|t| !t.is_empty());
            let outcome = if arm.is_none() {
                PluralResolutionOutcome::MissingPluralForms
            } else if entry.is_none() {
                PluralResolutionOutcome::MissingEntry
            } else if translation.is_none() {
                PluralResolutionOutcome::MissingTranslation
            } else {
                PluralResolutionOutcome::Matched
            };
            result.attempts.push(PluralResolutionAttempt {
                locale: candidate.locale().as_str().into(),
                context: context.clone(),
                key: id.into(),
                selected_arm: arm,
                outcome,
            });
            if let Some(translation) = translation {
                result.template = Some(translation.into());
                result.selected_arm = arm;
                result.matched_locale = Some(candidate.locale().as_str().into());
                result.matched_context = Some(context);
                result.matched_key = Some(id.into());
                break;
            }
        }
        Ok(result)
    }
    fn validated_plural_arm_count(
        &self,
        resolution: &PluralResolution,
    ) -> Result<Option<usize>, LocaleError> {
        let Some(locale) = resolution.matched_locale.as_deref() else {
            return Ok(None);
        };
        let locale = LocaleId::new(locale).map_err(|e| LocaleError::new(e.to_string()))?;
        Ok(self
            .rule(&locale)
            .and_then(|rule| recite_core::validate_plural_rule(rule).ok()))
    }
}
fn context(id: &str, domain: TextDomain) -> String {
    match domain {
        TextDomain::Line | TextDomain::Choice => id.into(),
        TextDomain::AvailabilityReason => format!("availability_reason:{id}"),
        TextDomain::PresentationLabel => format!("presentation_label:{id}"),
    }
}

fn plural_rule(input: &CatalogInput) -> Option<&str> {
    input
        .document()
        .headers()
        .iter()
        .find(|header| header.key().eq_ignore_ascii_case("Plural-Forms"))
        .map(|header| header.value())
}
