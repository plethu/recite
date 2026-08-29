use std::collections::{BTreeMap, BTreeSet};

use super::interpolation::{PlaceholderSyntaxKind, extract_placeholder_occurrences};

/// Placeholder mismatch between source text and a non-empty translation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlaceholderValidationError {
    missing: Vec<String>,
    extra: Vec<String>,
    repetition: Vec<String>,
    syntax: Option<String>,
    syntax_kind: Option<PlaceholderSyntaxKind>,
}

impl PlaceholderValidationError {
    #[must_use]
    pub fn missing(&self) -> &[String] {
        &self.missing
    }

    #[must_use]
    pub fn extra(&self) -> &[String] {
        &self.extra
    }

    #[must_use]
    pub fn syntax(&self) -> Option<&str> {
        self.syntax.as_deref()
    }

    #[must_use]
    pub fn syntax_kind(&self) -> Option<&PlaceholderSyntaxKind> {
        self.syntax_kind.as_ref()
    }

    #[must_use]
    pub fn repetition(&self) -> &[String] {
        &self.repetition
    }

    #[must_use]
    pub fn message(&self) -> String {
        let mut parts = Vec::new();
        if !self.missing.is_empty() {
            parts.push(format!(
                "missing {}",
                self.missing
                    .iter()
                    .map(|name| format!("{{{name}}}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !self.extra.is_empty() {
            parts.push(format!(
                "extra {}",
                self.extra
                    .iter()
                    .map(|name| format!("{{{name}}}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !self.repetition.is_empty() {
            parts.push(format!("repetition {}", self.repetition.join(", ")));
        }
        let syntax = self
            .syntax
            .as_deref()
            .map_or_else(String::new, |message| format!("; syntax error: {message}"));
        format!(
            "translation placeholders must match msgid: {}{}",
            parts.join("; "),
            syntax
        )
    }
}

/// Validate that a non-empty translation preserves source placeholder names.
pub fn validate_translation_placeholders(
    source: &str,
    translation: &str,
) -> Result<(), PlaceholderValidationError> {
    let source_occurrences = extract_placeholder_occurrences(source);
    let translation_occurrences = extract_placeholder_occurrences(translation);
    let source_names = source_occurrences
        .clone()
        .unwrap_or_default()
        .into_iter()
        .collect::<BTreeSet<_>>();
    let translation_names = translation_occurrences
        .clone()
        .unwrap_or_default()
        .into_iter()
        .collect::<BTreeSet<_>>();
    let missing = source_names
        .difference(&translation_names)
        .cloned()
        .collect::<Vec<_>>();
    let extra = translation_names
        .difference(&source_names)
        .cloned()
        .collect::<Vec<_>>();

    let counts = |values: &[String]| {
        values
            .iter()
            .fold(BTreeMap::<String, usize>::new(), |mut counts, name| {
                *counts.entry(name.clone()).or_default() += 1;
                counts
            })
    };
    let repetition_mismatch = source_occurrences
        .as_ref()
        .ok()
        .zip(translation_occurrences.as_ref().ok())
        .is_some_and(|(source, translation)| counts(source) != counts(translation));
    let repetition = source_occurrences
        .as_ref()
        .ok()
        .zip(translation_occurrences.as_ref().ok())
        .map(|(source, translation)| {
            let source_counts = counts(source);
            let translation_counts = counts(translation);
            source_counts
                .keys()
                .chain(translation_counts.keys())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .filter_map(|name| {
                    let expected = source_counts.get(name).copied().unwrap_or_default();
                    let actual = translation_counts.get(name).copied().unwrap_or_default();
                    (expected != actual)
                        .then(|| format!("{{{name}}} expected x{expected}, got x{actual}"))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if missing.is_empty()
        && extra.is_empty()
        && !repetition_mismatch
        && source_occurrences.is_ok()
        && translation_occurrences.is_ok()
    {
        Ok(())
    } else {
        let syntax = source_occurrences
            .as_ref()
            .err()
            .or_else(|| translation_occurrences.as_ref().err())
            .map(|error| error.message().to_owned());
        let syntax_kind = source_occurrences
            .as_ref()
            .err()
            .or_else(|| translation_occurrences.as_ref().err())
            .map(|error| error.kind().clone());
        Err(PlaceholderValidationError {
            missing,
            extra,
            repetition,
            syntax,
            syntax_kind,
        })
    }
}
