//! Staged values are independent of the immutable running trial.
use super::Writer;
use recite_core::ast::{InterpolationBinding, InterpolationType};
use recite_writer_session::preview::parse_value;
use recite_writer_session::{
    choices::PickerOption, localisation::setup::languages, preview::Inputs,
};
use std::collections::BTreeMap;
pub const VALUE_PAGE: usize = 4;

#[derive(Default)]
pub enum Review {
    #[default]
    Inputs,
    Value {
        value: String,
        kind: InterpolationType,
    },
    Diagnostic,
}

impl Review {
    pub fn targets(&self, binding: &InterpolationBinding) -> bool {
        matches!(self, Self::Value { value, kind } if *value == binding.value && *kind == binding.value_type)
    }
}

#[derive(Default)]
pub struct Settings {
    pub open: bool,
    pub locale: String,
    pub variant: String,
    pub include_drafts: bool,
    pub values: BTreeMap<String, String>,
    pub binding_page: usize,
    pub query: String,
    pub languages: Vec<PickerOption>,
    pub language_page: usize,
    pub review: Review,
}

impl Settings {
    pub fn capture(&self) -> Inputs {
        Inputs {
            locale: self.locale.clone(),
            variant: self.variant.clone(),
            include_drafts: self.include_drafts,
            values: Some(self.values.clone()),
        }
    }
}

impl Writer {
    pub fn review_trial_inputs(&mut self) {
        if self.phase != super::Phase::Writing || self.session.read_only() || !self.trial.failed() {
            return;
        }
        self.configure_trial();
        let bindings = self.session.model().document().preview_bindings();
        let invalid = bindings.iter().enumerate().find(|(_, binding)| {
            if binding.value_type == InterpolationType::String {
                return false;
            }
            let fallback = if binding.value_type == InterpolationType::Boolean {
                "false"
            } else {
                ""
            };
            let value = self
                .trial
                .settings
                .values
                .get(binding.value.trim_start_matches('$'))
                .map_or(fallback, String::as_str);
            parse_value(binding.value_type, value).is_err()
        });
        self.trial.settings.review = if let Some((index, binding)) = invalid {
            self.trial.settings.binding_page = index / VALUE_PAGE;
            Review::Value {
                value: binding.value.clone(),
                kind: binding.value_type,
            }
        } else {
            Review::Diagnostic
        };
    }

    pub fn search_preview_languages(&mut self) {
        let settings = &mut self.trial.settings;
        settings.languages = languages::matches(&settings.query);
        settings.language_page = 0;
    }
}
