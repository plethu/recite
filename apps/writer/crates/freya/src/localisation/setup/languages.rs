//! Search ISO language names, native names and registered BCP-47 variants.
use super::super::{
    create,
    messages::{MsgId, text as wording},
};
use crate::design::PickerOption;
use std::sync::OnceLock;

pub(crate) fn caption(tag: &str) -> Option<String> {
    let choice = locale_option(tag)?;
    Some(format!("{} · {}", choice.title, choice.value))
}

fn locale_option(tag: &str) -> Option<PickerOption> {
    let locale = create::language(tag).ok()?;
    let value = locale.to_string();
    if locale.is_cofi() {
        return Some(PickerOption {
            annotation: value.clone(),
            value,
            title: wording(MsgId::WriterCofiLanguage),
            detail: String::new(),
        });
    }
    if let Some(entry) = search_index()
        .iter()
        .find(|entry| entry.option.value == value)
    {
        return Some(entry.option.clone());
    }
    let language = isolang::Language::from_639_1(locale.base().language.as_str())
        .or_else(|| isolang::Language::from_639_3(locale.base().language.as_str()))?;
    Some(PickerOption {
        annotation: value.clone(),
        value,
        title: language.to_name().into(),
        detail: language.to_autonym().unwrap_or_default().into(),
    })
}

pub(crate) fn matches(query: &str) -> Vec<PickerOption> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    let mut choices: Vec<_> = locale_option(&query).into_iter().collect();
    if query.contains("cofi")
        && !choices
            .iter()
            .any(|choice| choice.value == super::super::target::COFI)
        && let Some(choice) = locale_option(super::super::target::COFI)
    {
        choices.push(choice);
    }
    let mut seen: std::collections::BTreeSet<String> =
        choices.iter().map(|choice| choice.value.clone()).collect();
    // Exact language/native names precede substring matches; each group retains
    // its cached alphabetical order. Rendering remains bounded by the viewport.
    for exact in [true, false] {
        for entry in search_index() {
            if entry.search.iter().any(|field| field == &query) == exact
                && entry.search.iter().any(|field| field.contains(&query))
                && seen.insert(entry.option.value.clone())
            {
                choices.push(entry.option.clone());
            }
        }
    }
    choices
}

struct SearchEntry {
    option: PickerOption,
    search: [String; 3],
}

// ISO names are static. Normalise and sort once. The picker virtualises results. Keep translated Cofi wording outside this cache.
fn search_index() -> &'static [SearchEntry] {
    static INDEX: OnceLock<Vec<SearchEntry>> = OnceLock::new();
    const VARIANTS: &[(&str, &str)] = &[
        ("en-US", "English (United States)"),
        ("en-GB", "English (United Kingdom)"),
        ("fr-CA", "French (Canada) · Français canadien"),
        ("fr-FR", "French (France) · Français"),
        ("pt-BR", "Portuguese (Brazil) · Português brasileiro"),
        ("pt-PT", "Portuguese (Portugal) · Português"),
        ("es-ES", "Spanish (Spain) · Español"),
        ("es-MX", "Spanish (Mexico) · Español mexicano"),
        (
            "es-419",
            "Spanish (Latin America) · Español latinoamericano",
        ),
        ("zh-Hans", "Chinese (Simplified) · 简体中文"),
        ("zh-Hant", "Chinese (Traditional) · 繁體中文"),
        ("sr-Latn", "Serbian (Latin) · Srpski"),
        ("sr-Cyrl", "Serbian (Cyrillic) · Српски"),
    ];
    INDEX.get_or_init(|| {
        let mut entries: Vec<_> = isolang::languages()
            .map(|language| {
                let tag = language.to_639_1().unwrap_or_else(|| language.to_639_3());
                let native = language.to_autonym().unwrap_or_default();
                let name = language.to_name();
                SearchEntry {
                    option: PickerOption {
                        annotation: tag.into(),
                        value: tag.into(),
                        title: name.into(),
                        detail: native.into(),
                    },
                    search: [tag.into(), name.to_lowercase(), native.to_lowercase()],
                }
            })
            .collect();
        entries.extend(VARIANTS.iter().map(|(tag, name)| {
            let (title, detail) = name.split_once(" · ").unwrap_or((name, ""));
            SearchEntry {
                option: PickerOption {
                    annotation: (*tag).into(),
                    value: (*tag).into(),
                    title: title.into(),
                    detail: detail.into(),
                },
                search: [
                    tag.to_lowercase(),
                    title.to_lowercase(),
                    detail.to_lowercase(),
                ],
            }
        }));
        entries.sort_by(|a, b| a.option.title.cmp(&b.option.title));
        entries
    })
}

#[cfg(test)]
mod tests;
