use recite_core::SourceLineIndex;
use recite_core::schema::{MetadataContextSelector, MetadataTarget};

pub(super) enum SelectorResolution<'a> {
    Missing,
    Value(&'a str),
    Malformed,
}

pub(super) fn resolve_selector<'a>(
    source: &'a SourceLineIndex,
    selector: &MetadataContextSelector,
    line_number: u32,
    target: MetadataTarget,
) -> SelectorResolution<'a> {
    let line_index = line_number.saturating_sub(1) as usize;
    let Some(line) = source.line(line_index) else {
        return SelectorResolution::Missing;
    };
    let line = line.strip_suffix('\r').unwrap_or(line);
    if matches!(selector, MetadataContextSelector::FieldSpeaker)
        && matches!(target, MetadataTarget::Line)
        && !recite_parser::metadata_assignments(line)
            .iter()
            .any(|assignment| assignment.key == "speaker")
    {
        for prior in (0..line_index).rev().filter_map(|line| source.line(line)) {
            let prior = prior.strip_suffix('\r').unwrap_or(prior);
            if prior.trim_start().starts_with("::") {
                return selector_symbol(prior, "speaker");
            }
        }
    }
    match selector {
        MetadataContextSelector::FieldSpeaker if !matches!(target, MetadataTarget::Line) => {
            SelectorResolution::Missing
        }
        MetadataContextSelector::FieldSpeaker => selector_symbol(line, "speaker"),
        MetadataContextSelector::MetadataKey(key) => selector_symbol(line, key),
    }
}

fn selector_symbol<'a>(line: &'a str, key: &str) -> SelectorResolution<'a> {
    let values = recite_parser::metadata_assignments(line)
        .into_iter()
        .filter(|assignment| assignment.key == key)
        .collect::<Vec<_>>();
    match values.as_slice() {
        [] => SelectorResolution::Missing,
        [assignment] => scalar_symbol(assignment.value)
            .map_or(SelectorResolution::Malformed, SelectorResolution::Value),
        [..] => SelectorResolution::Malformed,
    }
}

fn scalar_symbol(value: &str) -> Option<&str> {
    let mut chars = value.chars();
    let first = chars.next()?;
    (first.is_ascii_alphabetic() || first == '_')
        .then_some(value)
        .filter(|_| {
            chars.all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-')
            })
        })
}

pub(super) fn empty_values() -> &'static std::collections::BTreeSet<String> {
    use std::sync::OnceLock;
    static EMPTY: OnceLock<std::collections::BTreeSet<String>> = OnceLock::new();
    EMPTY.get_or_init(std::collections::BTreeSet::new)
}

#[path = "metadata_context/tests.rs"]
#[cfg(test)]
mod tests;
