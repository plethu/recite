//! Search ISO language names, native names and registered BCP-47 variants.
use super::super::{
    create,
    messages::{MsgId, text as wording},
};
use crate::design::{Button, tokens as t};
use freya::prelude::*;

pub(super) fn caption(tag: &str) -> Option<String> {
    let locale = create::language(tag).ok()?;
    if locale.is_cofi() {
        return Some(format!("{} · {locale}", wording(MsgId::WriterCofiLanguage)));
    }
    let language = isolang::Language::from_639_1(locale.base().language.as_str())
        .or_else(|| isolang::Language::from_639_3(locale.base().language.as_str()))?;
    Some(format!("{} · {locale}", language.to_name()))
}

pub(super) fn matches(query: &str) -> Vec<(String, String)> {
    let query = query.trim().to_lowercase();
    let mut choices = Vec::new();
    // A registered region/script typed in search is a selectable result, never
    // implicitly accepted as the chosen language.
    if let Ok(locale) = create::language(&query) {
        let tag = locale.to_string();
        if let Some(name) = caption(&tag) {
            choices.push((tag, name));
        }
    }
    if query.contains("cofi")
        && !choices
            .iter()
            .any(|(tag, _)| tag == super::super::target::COFI)
    {
        let tag = super::super::target::COFI;
        if let Some(name) = caption(tag) {
            choices.push((tag.into(), name));
        }
    }
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
    let mut languages: Vec<_> = isolang::languages()
        .filter_map(|language| {
            let tag = language.to_639_1().unwrap_or_else(|| language.to_639_3());
            let native = language.to_autonym().unwrap_or_default();
            let name = language.to_name();
            (tag.contains(&query)
                || name.to_lowercase().contains(&query)
                || native.to_lowercase().contains(&query))
            .then(|| (tag.to_owned(), format!("{name} · {native} · {tag}")))
        })
        .collect();
    languages.extend(
        VARIANTS
            .iter()
            .filter(|(tag, name)| {
                tag.to_lowercase().contains(&query) || name.to_lowercase().contains(&query)
            })
            .map(|(tag, name)| (tag.to_string(), format!("{name} · {tag}"))),
    );
    languages.sort_by(|a, b| a.1.cmp(&b.1));
    for choice in languages {
        if !choices.iter().any(|c| c.0 == choice.0) {
            choices.push(choice);
        }
        if choices.len() >= 8 {
            break;
        }
    }
    choices
}

pub(super) fn picker(
    mut selected: State<String>,
    query: State<String>,
    mut open: State<bool>,
    input: AccessibilityId,
    trigger: AccessibilityId,
    ids: [AccessibilityId; 8],
) -> Element {
    let choices = matches(&query.read());
    let count = choices.len();
    let mut list = rect().width(Size::fill()).spacing(t::SPACE_XS).child(
        Input::new(query)
            .a11y_id(input)
            .width(Size::fill())
            .placeholder(wording(MsgId::WriterLanguageExample))
            .on_submit(move |_| {
                if let Some((tag, _)) = matches(&query.peek()).into_iter().next() {
                    selected.set(tag);
                    open.set(false);
                    trigger.request_focus();
                }
            })
            .on_pre_key_down(move |event: Event<KeyboardEventData>| {
                if event.key == Key::Named(NamedKey::ArrowDown) && count > 0 {
                    ids[0].request_focus();
                    event.stop_propagation();
                    return false;
                }
                if event.key == Key::Named(NamedKey::Escape) {
                    return false;
                }
                crate::closing::text_input_key(event)
            }),
    );
    let mut options = rect()
        .width(Size::fill())
        .a11y_role(AccessibilityRole::ListBox)
        .a11y_alt(wording(MsgId::WriterTargetLanguage));
    for (index, (tag, caption)) in choices.into_iter().enumerate() {
        options = options.child(
            rect()
                .width(Size::fill())
                .on_key_down(move |event: Event<KeyboardEventData>| {
                    match event.key {
                        Key::Named(NamedKey::ArrowDown) => ids[(index + 1) % count].request_focus(),
                        Key::Named(NamedKey::ArrowUp) => {
                            if index == 0 {
                                input.request_focus();
                            } else {
                                ids[index - 1].request_focus();
                            }
                        }
                        _ => return,
                    }
                    event.stop_propagation();
                    event.prevent_default();
                })
                .child(
                    Button::new()
                        .option(*selected.peek() == tag)
                        .flat()
                        .a11y_id(ids[index])
                        .width(Size::fill())
                        .on_press(move |_| {
                            selected.set(tag.clone());
                            open.set(false);
                            trigger.request_focus();
                        })
                        .child(caption),
                ),
        );
    }
    list = list.child(options);
    if count == 0 {
        list = list.child(label().text(wording(MsgId::WriterNoLanguages)));
    }
    list.into_element()
}

#[cfg(test)]
mod tests;
