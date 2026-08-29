use super::diagnostics::{PoDiagnostic, PoPluralDiagnostic, error, error_span};
use super::types::EntryBuilder;
use crate::SourceId;

mod headers;
mod markup;
mod placeholders;

use markup::validate_translation_markup;
use placeholders::{placeholder_names, validate_translation};

pub(super) use headers::{parse_headers, parse_plural_forms};

pub(super) fn validate(
    name: &str,
    source: &str,
    line: usize,
    b: &EntryBuilder,
) -> Result<(), super::PoParseError> {
    if b.source_text.is_none() {
        return Err(error(name, line, PoDiagnostic::MissingField("msgid")));
    }
    let stale = b.obsolete || b.flags.iter().any(|flag| flag == "fuzzy");
    let header = b.source_text.as_deref() == Some("")
        && b.context.is_none()
        && b.plural_source_text.is_none();
    if !stale && !header && b.plural_source_text.is_some() {
        if b.translation.is_some() || b.plural_translations.is_empty() {
            return Err(error(
                name,
                line,
                PoDiagnostic::InvalidPluralArms(PoPluralDiagnostic::ContiguousArms),
            ));
        }
        for (expected, arm) in b.plural_translations.iter().enumerate() {
            if arm.index != Some(expected) {
                return Err(error(
                    name,
                    line,
                    PoDiagnostic::InvalidPluralArms(PoPluralDiagnostic::ExpectedArm(expected)),
                ));
            }
        }
    } else if !stale && !header && !b.plural_translations.is_empty() {
        return Err(error(
            name,
            line,
            PoDiagnostic::InvalidPluralArms(PoPluralDiagnostic::RequiresPluralSource),
        ));
    } else if !stale && b.translation.is_none() {
        return Err(error(name, line, PoDiagnostic::MissingField("msgstr")));
    }
    if !stale && !header {
        let Some(context) = &b.context else {
            return Err(error(
                name,
                line,
                PoDiagnostic::InvalidStableId(String::new()),
            ));
        };
        validate_context(name, source, line, b, context)?;
    }
    if stale || header {
        return Ok(());
    }
    for (value, range) in &b.source_id_comments {
        if !matches!(SourceId::parse(Some(value)), SourceId::Frozen { .. }) {
            return Err(error_span(
                name,
                source,
                range.clone(),
                PoDiagnostic::InvalidStableId(value.clone()),
            ));
        }
    }
    let source_names = placeholder_names(
        name,
        source,
        line,
        b,
        super::types::PoFieldTarget::SourceText,
        b.source_text.as_deref().unwrap_or_default(),
    )?;
    if let Some(translation) = &b.translation {
        validate_translation(
            name,
            source,
            line,
            b,
            super::types::PoFieldTarget::Translation,
            &source_names,
            &translation.text,
        )?;
        validate_translation_markup(
            name,
            source,
            line,
            b,
            super::types::PoFieldTarget::Translation,
            b.source_text.as_deref().unwrap_or_default(),
            &translation.text,
        )?;
    }
    let plural_names = b
        .plural_source_text
        .as_deref()
        .map(|text| {
            placeholder_names(
                name,
                source,
                line,
                b,
                super::types::PoFieldTarget::PluralSourceText,
                text,
            )
        })
        .transpose()?;
    for translation in &b.plural_translations {
        // gettext uses the singular source for arm zero and the plural source
        // for every subsequent arm, including locales with more than two arms.
        let expected = if translation.index == Some(0) {
            &source_names
        } else {
            plural_names.as_ref().unwrap_or(&source_names)
        };
        validate_translation(
            name,
            source,
            line,
            b,
            super::types::PoFieldTarget::PluralTranslation(translation.index.unwrap_or(0)),
            expected,
            &translation.text,
        )?;
        validate_translation_markup(
            name,
            source,
            line,
            b,
            super::types::PoFieldTarget::PluralTranslation(translation.index.unwrap_or(0)),
            if translation.index == Some(0) {
                b.source_text.as_deref().unwrap_or_default()
            } else {
                b.plural_source_text.as_deref().unwrap_or_default()
            },
            &translation.text,
        )?;
    }
    Ok(())
}

fn validate_context(
    name: &str,
    source: &str,
    line: usize,
    b: &EntryBuilder,
    context: &str,
) -> Result<(), super::PoParseError> {
    let (base, variant) = context.split_once('&').unwrap_or((context, ""));
    if context.matches('&').count() > 1
        || base.is_empty()
        || (context.contains('&') && variant.trim().is_empty())
        || (base.contains('@') && !matches!(SourceId::parse(Some(base)), SourceId::Frozen { .. }))
        || (base.len() == 20
            && !base
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)))
    {
        return Err(field_error(
            name,
            source,
            line,
            b,
            super::types::PoFieldTarget::Context,
            PoDiagnostic::InvalidStableId(context.to_owned()),
        ));
    }
    for prefix in [
        "dialogue_speaker:",
        "availability_reason:",
        "presentation_label:",
    ] {
        if let Some(value) = base.strip_prefix(prefix) {
            if value.trim().is_empty() {
                return Err(field_error(
                    name,
                    source,
                    line,
                    b,
                    super::types::PoFieldTarget::Context,
                    PoDiagnostic::InvalidStableId(context.to_owned()),
                ));
            }
            return Ok(());
        }
    }
    if base.contains('@') {
        return Ok(());
    }
    if base.len() == 20
        && base
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
    {
        Ok(())
    } else {
        Err(field_error(
            name,
            source,
            line,
            b,
            super::types::PoFieldTarget::Context,
            PoDiagnostic::InvalidStableId(context.to_owned()),
        ))
    }
}

fn field_error(
    name: &str,
    source: &str,
    line: usize,
    b: &EntryBuilder,
    target: super::types::PoFieldTarget,
    cause: PoDiagnostic,
) -> super::PoParseError {
    if let Some(field) = b.fields.iter().find(|field| field.target == target) {
        error_span(name, source, field.value_range.clone(), cause)
    } else {
        error(name, line, cause)
    }
}
