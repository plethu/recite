use std::path::Path;

use recite_core::{PoDiagnosticKind, PoDocument, PoParseError};

use super::DialogueCatalogMalformedReason;
use crate::error::CliError;

/// Runtime lookup's narrow view of a shared PO entry.
///
/// Parsing, preservation, and editing remain owned by `recite-core`; the CLI
/// projects only the fields its current preview path can consume.
#[derive(Debug)]
pub(super) struct PoEntry {
    pub(super) context: String,
    pub(super) source_text: String,
    pub(super) plural_source_text: Option<String>,
    pub(super) translations: Vec<String>,
}

pub(super) struct ParsedPoCatalog {
    pub(super) entries: Vec<PoEntry>,
    pub(super) plural_forms: Option<String>,
}

pub(super) fn parse_po_catalog(path: &Path, source: &str) -> Result<ParsedPoCatalog, CliError> {
    let document = PoDocument::parse_with_path(path.display().to_string(), source)
        .map_err(|error| malformed(path, error.line(), map_reason(&error)))?;
    let mut entries = Vec::new();
    let plural_forms = document
        .headers()
        .iter()
        .find(|header| header.key().eq_ignore_ascii_case("Plural-Forms"))
        .map(|header| header.value().to_owned());
    for entry in document.entries() {
        if entry.is_header() {
            continue;
        }
        if entry.is_obsolete() || entry.flags().iter().any(|flag| flag == "fuzzy") {
            continue;
        }
        let context = entry.context().ok_or_else(|| {
            malformed(
                path,
                entry.line(),
                DialogueCatalogMalformedReason::MissingContext,
            )
        })?;
        entries.push(PoEntry {
            context: context.to_owned(),
            source_text: entry.source_text().to_owned(),
            plural_source_text: entry.plural_source_text().map(str::to_owned),
            translations: if entry.is_plural() {
                entry
                    .plural_translations()
                    .iter()
                    .map(|translation| translation.text().to_owned())
                    .collect()
            } else {
                vec![entry.translation().unwrap_or_default().to_owned()]
            },
        });
    }
    Ok(ParsedPoCatalog {
        entries,
        plural_forms,
    })
}

fn map_reason(error: &PoParseError) -> DialogueCatalogMalformedReason {
    match error.kind() {
        PoDiagnosticKind::ExpectedDirective => DialogueCatalogMalformedReason::ExpectedDirective,
        PoDiagnosticKind::ExpectedQuotedString => {
            DialogueCatalogMalformedReason::ExpectedQuotedString
        }
        PoDiagnosticKind::MissingField("msgid") => DialogueCatalogMalformedReason::MissingId,
        PoDiagnosticKind::MissingField("msgstr") => {
            DialogueCatalogMalformedReason::MissingTranslation
        }
        PoDiagnosticKind::MissingField(_) => DialogueCatalogMalformedReason::ExpectedDirective,
        PoDiagnosticKind::QuotedContinuationWithoutField => {
            DialogueCatalogMalformedReason::QuotedContinuationWithoutField
        }
        PoDiagnosticKind::UnexpectedTextAfterQuotedString => {
            DialogueCatalogMalformedReason::UnexpectedTextAfterQuotedString
        }
        PoDiagnosticKind::UnterminatedQuotedString => {
            DialogueCatalogMalformedReason::UnterminatedQuotedString
        }
        PoDiagnosticKind::UnsupportedEscape(escape) => {
            DialogueCatalogMalformedReason::UnsupportedEscape {
                escape: escape.clone(),
            }
        }
        PoDiagnosticKind::InvalidPluralArms(detail) => {
            DialogueCatalogMalformedReason::InvalidHeader {
                detail: format!("plural entry has invalid arms: {detail}"),
            }
        }
        PoDiagnosticKind::InvalidPluralRule(reason) => {
            DialogueCatalogMalformedReason::InvalidPluralRule {
                detail: reason.to_string(),
            }
        }
        PoDiagnosticKind::PlaceholderMismatch(detail) => {
            DialogueCatalogMalformedReason::PlaceholderMismatch {
                detail: detail.clone(),
            }
        }
        PoDiagnosticKind::InvalidStableId(value) => {
            DialogueCatalogMalformedReason::InvalidStableId {
                value: value.clone(),
            }
        }
        PoDiagnosticKind::DuplicateField(field) => DialogueCatalogMalformedReason::DuplicateField {
            field: field.clone(),
        },
        PoDiagnosticKind::DuplicateKey(key) => {
            DialogueCatalogMalformedReason::DuplicateEntry { key: key.clone() }
        }
        PoDiagnosticKind::InvalidFieldOrder(detail) => {
            DialogueCatalogMalformedReason::InvalidFieldOrder {
                detail: detail.clone(),
            }
        }
        PoDiagnosticKind::InvalidHeader(detail) if detail.contains("active plural entries") => {
            DialogueCatalogMalformedReason::InvalidHeader {
                detail: detail.clone(),
            }
        }
        PoDiagnosticKind::InvalidHeader(detail) => DialogueCatalogMalformedReason::InvalidHeader {
            detail: detail.clone(),
        },
        PoDiagnosticKind::MarkupUnknownTag(_)
        | PoDiagnosticKind::MarkupUnbalancedTag(_)
        | PoDiagnosticKind::MarkupMissingTag(_)
        | PoDiagnosticKind::MarkupAttributeChange { .. } => {
            let Some(presentation) = error.diagnostic().presentation.clone() else {
                unreachable!("PO markup diagnostics have structured presentations");
            };
            DialogueCatalogMalformedReason::Markup {
                presentation,
                compatibility_message: error.diagnostic().message.clone(),
            }
        }
        _ => DialogueCatalogMalformedReason::ExpectedDirective,
    }
}

fn malformed(path: &Path, line: usize, reason: DialogueCatalogMalformedReason) -> CliError {
    CliError::DialogueCatalogMalformed {
        path: path.to_owned(),
        line,
        reason,
    }
}
