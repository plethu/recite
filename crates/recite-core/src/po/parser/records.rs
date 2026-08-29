use super::diagnostics::{PoDiagnostic, error, error_span};
use super::types::{ActiveField, EntryBuilder, PoFieldTarget, SourceLine};
use super::{PoComment, PoCommentKind, PoPreviousField, PoPreviousValue, PoUnknownField};

mod directive;
pub(super) use directive::directive;

pub(super) fn comment(
    name: &str,
    line: &SourceLine,
    trimmed: &str,
    builder: &mut EntryBuilder,
    active: &mut Option<ActiveField>,
) -> Result<(), super::PoParseError> {
    let number = line.number;
    let (body, obsolete) = if let Some(rest) = trimmed.strip_prefix("#~") {
        builder.obsolete = true;
        (rest.trim_start(), true)
    } else {
        (
            trimmed.strip_prefix('#').unwrap_or_default().trim_start(),
            false,
        )
    };
    if let Some(previous) = body.strip_prefix('|').map(str::trim_start) {
        let Some((keyword, value, target)) = directive(name, number, previous)? else {
            return Err(error(name, number, PoDiagnostic::ExpectedDirective));
        };
        let target = match target {
            PoFieldTarget::Context => PoFieldTarget::Previous(PoPreviousField::Context),
            PoFieldTarget::SourceText => PoFieldTarget::Previous(PoPreviousField::SourceText),
            PoFieldTarget::PluralSourceText => {
                PoFieldTarget::Previous(PoPreviousField::PluralSourceText)
            }
            PoFieldTarget::Translation => PoFieldTarget::Previous(PoPreviousField::Translation),
            PoFieldTarget::PluralTranslation(index) => {
                PoFieldTarget::Previous(PoPreviousField::PluralTranslation(index))
            }
            _ => return Err(error(name, number, PoDiagnostic::ExpectedDirective)),
        };
        builder.comments.push(PoComment {
            kind: PoCommentKind::Previous,
            text: body.to_owned(),
            obsolete,
        });
        *active = Some(ActiveField {
            target,
            keyword,
            value,
            start: line.start,
            value_start: line.start + line.text.find('"').unwrap_or(0),
            end: line.content_end,
            multiline: false,
            obsolete,
        });
        return Ok(());
    }
    if obsolete && let Some((keyword, value, target)) = directive(name, number, body)? {
        *active = Some(ActiveField {
            target,
            keyword,
            value,
            start: line.start,
            value_start: line.start + line.text.find('"').unwrap_or(0),
            end: line.content_end,
            multiline: false,
            obsolete,
        });
        return Ok(());
    }
    let (kind, text) = match body.chars().next() {
        Some('.') => (PoCommentKind::Extracted, body[1..].trim_start()),
        Some(':') => (PoCommentKind::Reference, body[1..].trim_start()),
        Some(',') => {
            builder.flags.extend(
                body[1..]
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(ToOwned::to_owned),
            );
            (PoCommentKind::Flag, body[1..].trim_start())
        }
        Some(_) => (PoCommentKind::Translator, body),
        None => (PoCommentKind::Other, ""),
    };
    let is_source_id =
        kind == PoCommentKind::Extracted && text.strip_prefix("source id:").is_some();
    builder.comments.push(PoComment {
        kind,
        text: text.to_owned(),
        obsolete,
    });
    if is_source_id {
        builder.source_id_comments.push((
            text["source id:".len()..].trim().to_owned(),
            line.start..line.content_end,
        ));
    }
    Ok(())
}

pub(super) fn comment_continuation(input: &str, obsolete: bool, previous: bool) -> Option<&str> {
    if !obsolete || previous {
        let prefix = if obsolete { "#~|" } else { "#|" };
        if let Some(body) = input.strip_prefix(prefix).map(str::trim_start) {
            return body.starts_with('"').then_some(body);
        }
    }
    // Obsolete regular fields use `#~ msgid` and `#~ "continuation"`.
    if obsolete {
        let body = input.strip_prefix("#~")?.trim_start();
        return body.starts_with('"').then_some(body);
    }
    None
}

pub(super) fn finish(
    active: &mut Option<ActiveField>,
    b: &mut EntryBuilder,
    name: &str,
    source: &str,
) -> Result<(), super::PoParseError> {
    let Some(active) = active.take() else {
        return Ok(());
    };
    match active.target {
        PoFieldTarget::Context => set(
            &mut b.context,
            active.value,
            active.target,
            name,
            source,
            active.start..active.end,
        )?,
        PoFieldTarget::SourceText => set(
            &mut b.source_text,
            active.value,
            active.target,
            name,
            source,
            active.start..active.end,
        )?,
        PoFieldTarget::PluralSourceText => set(
            &mut b.plural_source_text,
            active.value,
            active.target,
            name,
            source,
            active.start..active.end,
        )?,
        PoFieldTarget::Translation => {
            if b.translation.is_some() {
                return Err(error_span(
                    name,
                    source,
                    active.start..active.end,
                    PoDiagnostic::DuplicateField(active.target),
                ));
            }
            b.translation = Some(super::PoTranslation {
                index: None,
                text: active.value,
            });
        }
        PoFieldTarget::PluralTranslation(index) => {
            if b.plural_translations
                .iter()
                .any(|arm| arm.index == Some(index))
            {
                return Err(error_span(
                    name,
                    source,
                    active.start..active.end,
                    PoDiagnostic::DuplicateField(active.target),
                ));
            }
            b.plural_translations.push(super::PoTranslation {
                index: Some(index),
                text: active.value,
            });
        }
        PoFieldTarget::Previous(field) => b.previous.push(PoPreviousValue {
            field,
            value: active.value,
        }),
        PoFieldTarget::Unknown => b.unknown_fields.push(PoUnknownField {
            keyword: active.keyword.clone(),
            value: active.value,
            obsolete: active.obsolete,
        }),
    }
    b.fields.push(super::types::PoFieldRange {
        target: active.target,
        range: active.start..active.end,
        value_range: active.value_start..active.end,
        keyword: active.keyword,
        multiline: active.multiline,
        obsolete: active.obsolete,
    });
    b.obsolete |= active.obsolete;
    Ok(())
}

fn set<T>(
    field: &mut Option<T>,
    value: T,
    target: PoFieldTarget,
    name: &str,
    source: &str,
    range: std::ops::Range<usize>,
) -> Result<(), super::PoParseError> {
    if field.is_some() {
        return Err(error_span(
            name,
            source,
            range,
            PoDiagnostic::DuplicateField(target),
        ));
    }
    *field = Some(value);
    Ok(())
}
