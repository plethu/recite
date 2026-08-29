use std::collections::BTreeMap;

use super::super::diagnostics::{PoDiagnostic, error_span};
use super::super::types::{EntryBuilder, PoFieldTarget};

pub(super) fn placeholder_names(
    name: &str,
    source: &str,
    line: usize,
    b: &EntryBuilder,
    target: PoFieldTarget,
    text: &str,
) -> Result<Vec<String>, super::super::PoParseError> {
    crate::extract_placeholder_occurrences(text).map_err(|error| {
        field_error(
            name,
            source,
            line,
            b,
            target,
            PoDiagnostic::PlaceholderMismatch(error.message().to_owned()),
        )
    })
}

pub(super) fn validate_translation(
    name: &str,
    source: &str,
    line: usize,
    b: &EntryBuilder,
    target: PoFieldTarget,
    source_names: &[String],
    text: &str,
) -> Result<(), super::super::PoParseError> {
    if text.is_empty() {
        return Ok(());
    }
    let names = placeholder_names(name, source, line, b, target, text)?;
    let counts = |names: &[String]| {
        names
            .iter()
            .fold(BTreeMap::<String, usize>::new(), |mut counts, name| {
                *counts.entry(name.clone()).or_default() += 1;
                counts
            })
    };
    let source_counts = counts(source_names);
    let translation_counts = counts(&names);
    let missing = source_counts
        .iter()
        .filter(|(name, count)| translation_counts.get(*name).unwrap_or(&0) < count)
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    let extra = translation_counts
        .iter()
        .filter(|(name, count)| source_counts.get(*name).unwrap_or(&0) < count)
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    let mut details = Vec::new();
    if !missing.is_empty() {
        details.push(format!(
            "missing {}",
            missing
                .iter()
                .map(|name| format!("{{{name}}}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !extra.is_empty() {
        details.push(format!(
            "extra {}",
            extra
                .iter()
                .map(|name| format!("{{{name}}}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let repetition = source_counts
        .keys()
        .chain(translation_counts.keys())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .filter_map(|name| {
            let expected = source_counts.get(name).copied().unwrap_or_default();
            let actual = translation_counts.get(name).copied().unwrap_or_default();
            (expected != actual).then(|| format!("{{{name}}} expected x{expected}, got x{actual}"))
        })
        .collect::<Vec<_>>();
    if !repetition.is_empty() {
        details.push(format!("repetition {}", repetition.join(", ")));
    }
    if details.is_empty() {
        return Ok(());
    }
    Err(field_error(
        name,
        source,
        line,
        b,
        target,
        PoDiagnostic::PlaceholderMismatch(format!(
            "translation placeholders must match msgid: {}",
            details.join("; ")
        )),
    ))
}

fn field_error(
    name: &str,
    source: &str,
    line: usize,
    b: &EntryBuilder,
    target: PoFieldTarget,
    cause: PoDiagnostic,
) -> super::super::PoParseError {
    if let Some(field) = b.fields.iter().find(|field| field.target == target) {
        error_span(name, source, field.value_range.clone(), cause)
    } else {
        super::super::diagnostics::error(name, line, cause)
    }
}
