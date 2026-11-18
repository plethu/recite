//! Staged values are independent of the immutable running trial.
pub use super::Writer;
use crate::attention::{Pane, PaneMotion};
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
    /// The language the preview used before source-only was turned on, restored when it is off.
    pub remembered_locale: String,
    /// Source-only is off but no language is chosen yet: the language search is open for one.
    pub choosing_language: bool,
    pub review: Review,
    /// How far right of the preview's transcript the Configure preview action starts, as the
    /// native host measured it, so the slip hangs under the action that opened it.
    pub anchor: Option<u32>,
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
    /// Hang the Configure preview slip `x` right of the transcript's edge, under its action.
    pub fn place_trial_inputs(&mut self, x: u32) {
        self.trial.settings.anchor = Some(x);
    }
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

    /// Filter the preview languages as the query is typed, from the first page.
    pub fn set_preview_language_query(&mut self, query: String) {
        let settings = &mut self.trial.settings;
        settings.languages = languages::matches(&query);
        settings.query = query;
        settings.language_page = 0;
    }
    /// Whether the preview uses the source language only: no language chosen and none being
    /// chosen.
    pub fn previews_source_only(&self) -> bool {
        let settings = &self.trial.settings;
        settings.locale.is_empty() && !settings.choosing_language
    }
    /// Preview in the source language only, or turn that off again: back to the language chosen
    /// before, or, with none, into the language search to choose one.
    pub fn set_source_language_only(&mut self, on: bool) {
        let settings = &mut self.trial.settings;
        settings.languages.clear();
        settings.query.clear();
        if on {
            if !settings.locale.is_empty() {
                settings.remembered_locale = std::mem::take(&mut settings.locale);
            }
            settings.choosing_language = false;
        } else if settings.remembered_locale.is_empty() {
            settings.choosing_language = true;
            settings.review = Review::Inputs;
            self.focus_pane(PaneMotion::To(Pane::Preview));
        } else {
            settings.locale = settings.remembered_locale.clone();
        }
    }
    /// Preview in `locale`, chosen from the language search.
    pub fn choose_preview_language(&mut self, locale: String) {
        let settings = &mut self.trial.settings;
        settings.locale = locale;
        settings.languages.clear();
        settings.choosing_language = false;
    }
}

#[cfg(test)]
mod tests;
