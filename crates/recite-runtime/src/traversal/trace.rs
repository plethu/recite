use std::cell::RefCell;
use std::collections::BTreeMap;

use crate::locale::PluralResolutionAttempt;

/// Trace-only data captured while resolving runtime output.
///
/// This is deliberately separate from [`crate::DialogueEvent`]: localized
/// templates are diagnostic provenance, not part of the normal game-facing
/// event stream. The runtime records the selected availability-reason
/// template at the same point as the provider lookup so trace consumers do
/// not need to perform a second, potentially lossy lookup.
#[derive(Default)]
pub struct DialogueTrace {
    localized_availability_templates: RefCell<BTreeMap<String, String>>,
    plural_lines: RefCell<BTreeMap<String, PluralLineTrace>>,
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
            .get(id)
            .cloned()
    }

    #[must_use]
    pub fn plural_line(&self, id: &str) -> Option<PluralLineTrace> {
        self.plural_lines.borrow().get(id).cloned()
    }

    pub(crate) fn record_localized_availability_template(&self, id: &str, template: &str) {
        self.localized_availability_templates
            .borrow_mut()
            .insert(id.to_owned(), template.to_owned());
    }

    pub(crate) fn record_plural_line(&self, id: &str, trace: PluralLineTrace) {
        self.plural_lines.borrow_mut().insert(id.to_owned(), trace);
    }
}
