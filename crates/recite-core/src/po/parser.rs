use std::collections::BTreeSet;

use super::{PoDocument, PoEntry, PoEntryId, PoTranslation};

mod plural;
pub use plural::{PluralRuleError, evaluate_plural_form, validate_plural_rule};
mod diagnostics;
mod fields;
mod records;
mod syntax;
mod types;
mod validate;

use diagnostics::{PoDiagnostic, PoHeaderDiagnostic, PoPluralDiagnostic, error};
use fields::{
    starts_comment_for_next_record, starts_record, starts_translation, validate_field_order,
};
use records::{comment, comment_continuation, finish};
use syntax::source_lines;
use types::{ActiveField, EntryBuilder, SourceLine};
pub use types::{
    PoComment, PoCommentKind, PoDiagnosticKind, PoHeader, PoParseError, PoParseReport,
    PoPreviousField, PoPreviousValue, PoUnknownField,
};
pub(crate) use types::{PoFieldRange, PoFieldTarget};

pub(super) fn parse_document(name: String, source: String) -> Result<PoDocument, PoParseError> {
    let lines = source_lines(&source);
    let line_ending = lines
        .first()
        .and_then(|line| source.get(line.content_end..line.end))
        .filter(|ending| !ending.is_empty())
        .map_or("\n", |ending| if ending == "\r\n" { "\r\n" } else { "\n" });
    let mut entries = Vec::new();
    let mut start = None;
    let mut has_translation = false;
    for (index, line) in lines.iter().enumerate() {
        if line.text.trim().is_empty() {
            if let Some(start) = start.take() {
                entries.push(parse_entry(&name, &source, &lines[start..index])?);
            }
            has_translation = false;
        } else if start.is_none() {
            start = Some(index);
            has_translation = starts_translation(line.text.trim_start());
        } else if has_translation
            && (starts_record(line.text.trim_start())
                || starts_comment_for_next_record(line.text.trim_start()))
        {
            let start_index = start.replace(index).unwrap_or(index);
            entries.push(parse_entry(&name, &source, &lines[start_index..index])?);
            has_translation = false;
        } else {
            has_translation |= starts_translation(line.text.trim_start());
        }
    }
    if let Some(start) = start {
        entries.push(parse_entry(&name, &source, &lines[start..])?);
    }
    for (index, entry) in entries.iter_mut().enumerate() {
        entry.id = PoEntryId::new(index);
    }
    let mut header_entries = entries.iter().filter(|entry| entry.header);
    let header_entry = header_entries.next();
    if let Some(entry) = header_entries.next() {
        return Err(error(
            &name,
            entry.start_line,
            PoDiagnostic::InvalidHeader(PoHeaderDiagnostic::MultipleHeaders),
        ));
    }
    let mut headers = Vec::new();
    if let Some(entry) = header_entry {
        let value = entry
            .translation
            .as_ref()
            .map(PoTranslation::text)
            .unwrap_or_default();
        headers = validate::parse_headers(&name, entry, value)?;
    }
    let plural_forms = headers
        .iter()
        .find(|header| header.key.eq_ignore_ascii_case("Plural-Forms"))
        .and_then(|header| validate::parse_plural_forms(&header.value));
    let mut keys = BTreeSet::new();
    for entry in &entries {
        if entry.header || entry.obsolete || entry.flags.iter().any(|flag| flag == "fuzzy") {
            continue;
        }
        let Some(context) = entry.context.as_ref() else {
            return Err(error(
                &name,
                entry.start_line,
                PoDiagnostic::InvalidStableId(String::new()),
            ));
        };
        let key = (context.clone(), entry.source_text.clone());
        if !keys.insert(key) {
            return Err(error(
                &name,
                entry.start_line,
                PoDiagnostic::DuplicateKey {
                    context: context.clone(),
                    source_text: entry.source_text.clone(),
                },
            ));
        }
        if entry.is_plural() {
            let Some((expected, _)) = plural_forms.as_ref() else {
                // POT files are locale-neutral templates. Their plural arms
                // are intentionally empty and receive a locale's validated
                // Plural-Forms header only in a translated PO catalogue.
                if entry
                    .plural_translations
                    .iter()
                    .all(|translation| translation.text().is_empty())
                {
                    continue;
                }
                return Err(error(
                    &name,
                    entry.start_line,
                    PoDiagnostic::InvalidHeader(PoHeaderDiagnostic::PluralHeaderRequired),
                ));
            };
            if entry.plural_translations.len() != *expected {
                return Err(error(
                    &name,
                    entry.start_line,
                    PoDiagnostic::InvalidPluralArms(PoPluralDiagnostic::Count {
                        expected: *expected,
                        actual: entry.plural_translations.len(),
                    }),
                ));
            }
        }
    }
    Ok(PoDocument {
        source_name: name,
        source,
        entries,
        headers,
        line_ending,
    })
}

pub(crate) use syntax::format_field;

fn parse_entry(name: &str, source: &str, lines: &[SourceLine]) -> Result<PoEntry, PoParseError> {
    let mut builder = EntryBuilder {
        start: lines[0].start,
        end: lines
            .last()
            .map_or(lines[0].content_end, |line| line.content_end),
        ..EntryBuilder::default()
    };
    let mut active: Option<ActiveField> = None;
    for line in lines {
        let trimmed = line.text.trim_start();
        if trimmed.starts_with('#') {
            if let Some(current) = active.as_mut()
                && let Some(input) = comment_continuation(
                    trimmed,
                    current.obsolete,
                    matches!(current.target, PoFieldTarget::Previous(_)),
                )
            {
                current
                    .value
                    .push_str(&syntax::parse_quoted(name, line.number, input)?);
                current.end = line.content_end;
                current.multiline = true;
                continue;
            }
            finish(&mut active, &mut builder, name, source)?;
            comment(name, line, trimmed, &mut builder, &mut active)?;
        } else if trimmed.starts_with('"') {
            let Some(current) = active.as_mut() else {
                return Err(error(
                    name,
                    line.number,
                    PoDiagnostic::QuotedContinuationWithoutField,
                ));
            };
            current
                .value
                .push_str(&syntax::parse_quoted(name, line.number, trimmed)?);
            current.end = line.content_end;
            current.multiline = true;
        } else {
            finish(&mut active, &mut builder, name, source)?;
            let Some((keyword, value, target)) = records::directive(name, line.number, trimmed)?
            else {
                return Err(error(name, line.number, PoDiagnostic::ExpectedDirective));
            };
            validate_field_order(&builder, target, name, source, line)?;
            active = Some(ActiveField {
                target,
                keyword,
                value,
                start: line.start,
                value_start: line.start + line.text.find('"').unwrap_or(0),
                end: line.content_end,
                multiline: false,
                obsolete: false,
            });
        }
    }
    finish(&mut active, &mut builder, name, source)?;
    validate::validate(name, source, lines[0].number, &builder)?;
    let source_text = builder.source_text.clone().unwrap_or_default();
    let header =
        source_text.is_empty() && builder.context.is_none() && builder.plural_source_text.is_none();
    Ok(PoEntry {
        id: PoEntryId::new(0),
        range: builder.start..builder.end,
        start_line: lines[0].number,
        context: builder.context,
        source_text: source_text.clone(),
        plural_source_text: builder.plural_source_text,
        translation: builder.translation,
        plural_translations: builder.plural_translations,
        comments: builder.comments,
        flags: builder.flags,
        previous: builder.previous,
        unknown_fields: builder.unknown_fields,
        obsolete: builder.obsolete,
        header,
        fields: builder.fields,
    })
}
