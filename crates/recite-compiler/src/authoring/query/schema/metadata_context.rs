use recite_core::{MetadataContextSelector, MetadataTarget};

pub(super) enum SelectorResolution<'a> {
    Missing,
    Value(&'a str),
    Malformed,
}

pub(super) fn resolve_selector<'a>(
    text: &'a str,
    selector: &MetadataContextSelector,
    line_number: u32,
    target: MetadataTarget,
) -> SelectorResolution<'a> {
    let Some(line) = text.lines().nth(line_number.saturating_sub(1) as usize) else {
        return SelectorResolution::Missing;
    };
    if matches!(selector, MetadataContextSelector::FieldSpeaker)
        && matches!(target, MetadataTarget::Line)
        && !recite_parser::metadata_assignments(line)
            .iter()
            .any(|assignment| assignment.key == "speaker")
    {
        let prior_lines = text
            .lines()
            .take(line_number.saturating_sub(1) as usize)
            .collect::<Vec<_>>();
        for prior in prior_lines.into_iter().rev() {
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
