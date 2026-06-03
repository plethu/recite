use std::path::Path;

use super::DialogueCatalogMalformedReason;
use crate::error::CliError;
use recite_core::{extract_placeholder_names, validate_translation_placeholders};

#[derive(Debug)]
pub(super) struct PoEntry {
    pub(super) context: String,
    pub(super) source_text: String,
    pub(super) translation: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PoField {
    Context,
    Id,
    Translation,
}

#[derive(Default)]
struct PoEntryBuilder {
    context: Option<String>,
    source_text: Option<String>,
    translation: Option<String>,
    active: Option<PoField>,
    start_line: usize,
}

pub(super) fn parse_po_catalog(path: &Path, source: &str) -> Result<Vec<PoEntry>, CliError> {
    let mut entries = Vec::new();
    let mut builder = PoEntryBuilder::default();

    for (line_index, line) in source.lines().enumerate() {
        let line_number = line_index + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            finish_entry(path, &mut builder, &mut entries)?;
            continue;
        }
        if trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('"') {
            let value = parse_po_quoted(path, line_number, trimmed)?;
            match builder.active {
                Some(PoField::Context) => append_field(&mut builder.context, value),
                Some(PoField::Id) => append_field(&mut builder.source_text, value),
                Some(PoField::Translation) => append_field(&mut builder.translation, value),
                None => {
                    return Err(malformed(
                        path,
                        line_number,
                        DialogueCatalogMalformedReason::QuotedContinuationWithoutField,
                    ));
                }
            }
            continue;
        }

        let Some((field, value)) = parse_directive(path, line_number, trimmed)? else {
            return Err(malformed(
                path,
                line_number,
                DialogueCatalogMalformedReason::ExpectedDirective,
            ));
        };
        if builder.start_line == 0 {
            builder.start_line = line_number;
        }
        match field {
            PoField::Context => builder.context = Some(value),
            PoField::Id => builder.source_text = Some(value),
            PoField::Translation => builder.translation = Some(value),
        }
        builder.active = Some(field);
    }

    finish_entry(path, &mut builder, &mut entries)?;
    Ok(entries)
}

fn parse_directive(
    path: &Path,
    line_number: usize,
    trimmed: &str,
) -> Result<Option<(PoField, String)>, CliError> {
    if trimmed.starts_with("msgid_plural") || trimmed.starts_with("msgstr[") {
        return Err(malformed(
            path,
            line_number,
            DialogueCatalogMalformedReason::PluralEntriesUnsupported,
        ));
    }

    for (keyword, field) in [
        ("msgctxt", PoField::Context),
        ("msgid", PoField::Id),
        ("msgstr", PoField::Translation),
    ] {
        let Some(rest) = trimmed.strip_prefix(keyword) else {
            continue;
        };
        let rest = rest.trim_start();
        if rest.starts_with('[') {
            return Err(malformed(
                path,
                line_number,
                DialogueCatalogMalformedReason::PluralEntriesUnsupported,
            ));
        }
        return Ok(Some((field, parse_po_quoted(path, line_number, rest)?)));
    }

    Ok(None)
}

fn append_field(field: &mut Option<String>, value: String) {
    field.get_or_insert_with(String::new).push_str(&value);
}

fn finish_entry(
    path: &Path,
    builder: &mut PoEntryBuilder,
    entries: &mut Vec<PoEntry>,
) -> Result<(), CliError> {
    if builder.context.is_none() && builder.source_text.is_none() && builder.translation.is_none() {
        builder.active = None;
        builder.start_line = 0;
        return Ok(());
    }

    let line = builder.start_line.max(1);
    let context = builder.context.take();
    let source_text = builder
        .source_text
        .take()
        .ok_or_else(|| malformed(path, line, DialogueCatalogMalformedReason::MissingId))?;
    let translation = builder.translation.take().ok_or_else(|| {
        malformed(
            path,
            line,
            DialogueCatalogMalformedReason::MissingTranslation,
        )
    })?;
    if context.is_none() && source_text.is_empty() {
        builder.active = None;
        builder.start_line = 0;
        return Ok(());
    }
    validate_placeholders(path, line, &source_text, &translation)?;
    let context = context
        .ok_or_else(|| malformed(path, line, DialogueCatalogMalformedReason::MissingContext))?;

    entries.push(PoEntry {
        context,
        source_text,
        translation,
    });
    builder.active = None;
    builder.start_line = 0;
    Ok(())
}

fn validate_placeholders(
    path: &Path,
    line: usize,
    source_text: &str,
    translation: &str,
) -> Result<(), CliError> {
    if translation.is_empty() {
        return Ok(());
    }

    extract_placeholder_names(source_text).map_err(|error| {
        malformed(
            path,
            line,
            DialogueCatalogMalformedReason::PlaceholderMismatch {
                detail: format!("msgid has invalid placeholder syntax: {}", error.message()),
            },
        )
    })?;
    extract_placeholder_names(translation).map_err(|error| {
        malformed(
            path,
            line,
            DialogueCatalogMalformedReason::PlaceholderMismatch {
                detail: format!("msgstr has invalid placeholder syntax: {}", error.message()),
            },
        )
    })?;
    validate_translation_placeholders(source_text, translation).map_err(|error| {
        malformed(
            path,
            line,
            DialogueCatalogMalformedReason::PlaceholderMismatch {
                detail: error.message(),
            },
        )
    })
}

fn parse_po_quoted(path: &Path, line_number: usize, input: &str) -> Result<String, CliError> {
    let mut chars = input.chars();
    if chars.next() != Some('"') {
        return Err(malformed(
            path,
            line_number,
            DialogueCatalogMalformedReason::ExpectedQuotedString,
        ));
    }

    let mut output = String::new();
    let mut escaped = false;
    while let Some(character) = chars.next() {
        if escaped {
            output.push(match character {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '"' => '"',
                '\\' => '\\',
                other => {
                    return Err(malformed(
                        path,
                        line_number,
                        DialogueCatalogMalformedReason::UnsupportedEscape {
                            escape: format!("\\{other}"),
                        },
                    ));
                }
            });
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '"' => {
                if chars.as_str().trim().is_empty() {
                    return Ok(output);
                }
                return Err(malformed(
                    path,
                    line_number,
                    DialogueCatalogMalformedReason::UnexpectedTextAfterQuotedString,
                ));
            }
            other => output.push(other),
        }
    }

    Err(malformed(
        path,
        line_number,
        DialogueCatalogMalformedReason::UnterminatedQuotedString,
    ))
}

fn malformed(path: &Path, line: usize, reason: DialogueCatalogMalformedReason) -> CliError {
    CliError::DialogueCatalogMalformed {
        path: path.to_owned(),
        line,
        reason,
    }
}
