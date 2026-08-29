use recite_core::MetadataContextSelector;

#[derive(Clone, Copy)]
pub(super) enum SelectorSite {
    Block,
    Line,
    Choice,
}

pub(super) enum SelectorResolution {
    Missing,
    Value(String),
    Malformed,
}

pub(super) fn selector_site(line: &str) -> Option<SelectorSite> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("::") {
        Some(SelectorSite::Block)
    } else if trimmed.starts_with('>') {
        Some(SelectorSite::Line)
    } else if trimmed.starts_with('?') {
        Some(SelectorSite::Choice)
    } else {
        None
    }
}

pub(super) fn resolve_selector(
    selector: &MetadataContextSelector,
    text: &str,
    line: &str,
    line_index: usize,
    site: SelectorSite,
) -> SelectorResolution {
    match selector {
        MetadataContextSelector::FieldSpeaker => match site {
            SelectorSite::Line => match selector_symbol(line, "speaker") {
                SelectorResolution::Value(value) => SelectorResolution::Value(value),
                SelectorResolution::Missing => block_default_speaker(text, line_index),
                SelectorResolution::Malformed => SelectorResolution::Malformed,
            },
            SelectorSite::Block | SelectorSite::Choice => SelectorResolution::Missing,
        },
        MetadataContextSelector::MetadataKey(key) => selector_symbol(line, key),
    }
}

fn selector_symbol(line: &str, key: &str) -> SelectorResolution {
    let values = line
        .split_whitespace()
        .filter_map(|token| token.split_once('='))
        .filter(|(candidate, _)| *candidate == key)
        .map(|(_, value)| scalar_symbol(value))
        .collect::<Vec<_>>();
    match values.as_slice() {
        [] => SelectorResolution::Missing,
        [Some(value)] => SelectorResolution::Value(value.clone()),
        [_] | [_, ..] => SelectorResolution::Malformed,
    }
}

fn block_default_speaker(text: &str, line_index: usize) -> SelectorResolution {
    text.lines()
        .take(line_index.saturating_add(1))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .find(|line| line.trim_start().starts_with("::"))
        .map_or(SelectorResolution::Missing, |line| {
            selector_symbol(line, "speaker")
        })
}

fn scalar_symbol(value: &str) -> Option<String> {
    let mut characters = value.chars();
    let first = characters.next()?;
    ((first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-')
        }))
    .then(|| value.to_owned())
}
