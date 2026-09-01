use std::cell::RefCell;

use crate::locale::{
    LocaleLookupAttempt, LocaleLookupOutcome, LocaleLookupProvenance, PluralResolutionAttempt,
    TextDomain,
};

/// Trace-only data captured while resolving runtime output.
///
/// This is deliberately separate from [`crate::DialogueEvent`]: localized
/// templates are diagnostic provenance, not part of the normal game-facing
/// event stream. The runtime records the selected availability-reason
/// template at the same point as the provider lookup so trace consumers do
/// not need to perform a second, potentially lossy lookup.
#[derive(Default)]
pub struct DialogueTrace {
    localized_availability_templates: RefCell<Vec<(String, String)>>,
    plural_lines: RefCell<Vec<(String, PluralLineTrace)>>,
    plural_arm_counts: RefCell<Vec<(String, usize)>>,
    localized_lookups: RefCell<Vec<LocalizedLookupTrace>>,
}

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalizedLookupTrace {
    pub id: String,
    pub source_text: String,
    pub resolved_text: Option<String>,
    pub domain: TextDomain,
    pub attempts: Vec<LocaleLookupAttempt>,
    pub matched_locale: Option<String>,
    pub matched_context: Option<String>,
    pub matched_key: Option<String>,
    pub outcome: LocaleLookupOutcome,
}

/// Trace-only plural resolution provenance for one line delivery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluralLineTrace {
    pub singular_source_text: String,
    pub plural_source_text: String,
    pub count: i64,
    pub selected_arm: usize,
    pub attempts: Vec<PluralResolutionAttempt>,
    pub matched_locale: Option<String>,
    pub matched_context: Option<String>,
    pub matched_key: Option<String>,
    pub matched_arm: Option<usize>,
    pub source_fallback_arm: Option<usize>,
}

impl DialogueTrace {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the exact availability-reason template selected by runtime
    /// locale resolution, if that reason was traversed.
    #[must_use]
    pub fn localized_availability_template(&self, id: &str) -> Option<String> {
        self.localized_availability_templates
            .borrow()
            .iter()
            .rev()
            .find(|(candidate, _)| candidate == id)
            .map(|(_, template)| template.clone())
    }

    #[must_use]
    pub fn plural_line(&self, id: &str) -> Option<PluralLineTrace> {
        self.plural_lines
            .borrow()
            .iter()
            .rev()
            .find(|(candidate, _)| candidate == id)
            .map(|(_, trace)| trace.clone())
    }

    pub(crate) fn plural_lines(&self) -> Vec<(String, PluralLineTrace)> {
        self.plural_lines.borrow().iter().cloned().collect()
    }

    pub(crate) fn localized_lookups(&self) -> Vec<LocalizedLookupTrace> {
        self.localized_lookups.borrow().clone()
    }

    pub(crate) fn record_localized_availability_template(&self, id: &str, template: &str) {
        self.localized_availability_templates
            .borrow_mut()
            .push((id.to_owned(), template.to_owned()));
    }

    pub(crate) fn record_plural_line(&self, id: &str, trace: PluralLineTrace) {
        self.plural_lines.borrow_mut().push((id.to_owned(), trace));
    }

    pub(crate) fn plural_arm_count(&self) -> Option<usize> {
        self.plural_arm_counts
            .borrow()
            .last()
            .map(|(_, count)| *count)
    }

    pub(crate) fn record_plural_arm_count(&self, id: &str, count: usize) {
        self.plural_arm_counts
            .borrow_mut()
            .push((id.to_owned(), count));
    }

    pub(crate) fn record_localized_lookup(
        &self,
        id: &str,
        source_text: &str,
        domain: TextDomain,
        resolved: &LocaleLookupProvenance,
    ) {
        let trace = LocalizedLookupTrace {
            id: id.to_owned(),
            source_text: source_text.to_owned(),
            resolved_text: resolved.template.clone(),
            domain,
            attempts: resolved.attempts.clone(),
            matched_locale: resolved.matched_locale.clone(),
            matched_context: resolved.matched_context.clone(),
            matched_key: resolved.matched_key.clone(),
            outcome: if resolved.template.is_some() {
                LocaleLookupOutcome::Matched
            } else {
                LocaleLookupOutcome::MissingEntry
            },
        };
        self.localized_lookups.borrow_mut().push(trace);
    }
}
