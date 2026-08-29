use super::diagnostics::{PoDiagnostic, error_span};
use super::types::{EntryBuilder, PoFieldTarget};
use crate::po::parser::types::SourceLine;

pub(super) fn starts_record(line: &str) -> bool {
    starts_directive(line, "msgid")
        || starts_directive(line, "msgctxt")
        || starts_directive(line, "#~ msgid")
        || starts_directive(line, "#~ msgctxt")
}

fn starts_directive(line: &str, directive: &str) -> bool {
    line.strip_prefix(directive)
        .is_some_and(|rest| rest.starts_with(char::is_whitespace))
}

pub(super) fn starts_comment_for_next_record(line: &str) -> bool {
    if let Some(rest) = line.strip_prefix("#~") {
        // `#~` also prefixes obsolete comments. They belong to the next
        // obsolete record when records are adjacent, unlike `#~|` previous
        // value continuations and `#~ "..."` obsolete field continuations.
        return rest.trim_start().starts_with('#');
    }
    line.starts_with('#') && !line.starts_with("#|")
}

pub(super) fn starts_translation(line: &str) -> bool {
    line.strip_prefix("msgstr")
        .is_some_and(|rest| rest.starts_with(char::is_whitespace) || rest.starts_with('['))
        || line
            .strip_prefix("#~ msgstr")
            .is_some_and(|rest| rest.starts_with(char::is_whitespace) || rest.starts_with('['))
}

pub(super) fn validate_field_order(
    builder: &EntryBuilder,
    target: PoFieldTarget,
    name: &str,
    source: &str,
    line: &SourceLine,
) -> Result<(), super::PoParseError> {
    let invalid = match target {
        PoFieldTarget::Context => {
            builder.source_text.is_some()
                || builder.plural_source_text.is_some()
                || builder.translation.is_some()
                || !builder.plural_translations.is_empty()
        }
        PoFieldTarget::SourceText => {
            builder.source_text.is_some()
                || builder.plural_source_text.is_some()
                || builder.translation.is_some()
                || !builder.plural_translations.is_empty()
        }
        PoFieldTarget::PluralSourceText => {
            builder.source_text.is_none()
                || builder.plural_source_text.is_some()
                || builder.translation.is_some()
                || !builder.plural_translations.is_empty()
        }
        PoFieldTarget::Translation => {
            builder.source_text.is_none()
                || builder.plural_source_text.is_some()
                || builder.translation.is_some()
                || !builder.plural_translations.is_empty()
        }
        PoFieldTarget::PluralTranslation(_) => {
            builder.source_text.is_none()
                || builder.plural_source_text.is_none()
                || builder.translation.is_some()
        }
        PoFieldTarget::Previous(_) | PoFieldTarget::Unknown => false,
    };
    if invalid {
        return Err(error_span(
            name,
            source,
            line.start..line.content_end,
            PoDiagnostic::InvalidFieldOrder(target),
        ));
    }
    Ok(())
}
