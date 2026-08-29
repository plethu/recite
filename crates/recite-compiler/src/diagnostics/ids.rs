use recite_core::{
    Choice, ChoiceId, Diagnostic, DiagnosticCode, Line, LineId, SourceId, SourceSpan,
};

use super::{compiler_diagnostic, diagnostic_contract, related_presentation, string_argument};

const MISSING_LINE_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_ID001");
const MISSING_CHOICE_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_ID002");
const DUPLICATE_LINE_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_ID003");
const DUPLICATE_CHOICE_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_ID004");
const DRAFT_LINE_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_ID005");
const DRAFT_CHOICE_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_ID006");
const MALFORMED_LINE_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_ID007");
const MALFORMED_CHOICE_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_ID008");

pub(crate) fn missing_line_id(line: &Line) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&MISSING_LINE_ID, "diagnostic-id-001"),
        "line header must include a stable line id",
        line.span.clone(),
        [],
    )
    .with_help_presentation(super::auxiliary_presentation("diagnostic-id-001-help", []))
}

pub(crate) fn missing_choice_id(choice: &Choice) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&MISSING_CHOICE_ID, "diagnostic-id-002"),
        "choice header must include a stable choice id",
        choice.span.clone(),
        [],
    )
    .with_help_presentation(super::auxiliary_presentation("diagnostic-id-002-help", []))
}

pub(crate) fn draft_line_id(line: &Line) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&DRAFT_LINE_ID, "diagnostic-id-005"),
        "line header has an unfrozen draft source id",
        line.span.clone(),
        [],
    )
    .with_help_presentation(super::auxiliary_presentation("diagnostic-id-005-help", []))
}

pub(crate) fn draft_choice_id(choice: &Choice) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&DRAFT_CHOICE_ID, "diagnostic-id-006"),
        "choice header has an unfrozen draft source id",
        choice.span.clone(),
        [],
    )
    .with_help_presentation(super::auxiliary_presentation("diagnostic-id-006-help", []))
}

pub(crate) fn malformed_line_id(line: &Line, source_id: &SourceId) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&MALFORMED_LINE_ID, "diagnostic-id-007"),
        format!(
            "line header has malformed source id `{}`",
            source_id.display_text().unwrap_or_default()
        ),
        line.span.clone(),
        vec![(
            "id".to_owned(),
            string_argument(source_id.display_text().unwrap_or_default()),
        )],
    )
    .with_help_presentation(super::auxiliary_presentation("diagnostic-id-007-help", []))
}

pub(crate) fn malformed_choice_id(choice: &Choice, source_id: &SourceId) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&MALFORMED_CHOICE_ID, "diagnostic-id-008"),
        format!(
            "choice header has malformed source id `{}`",
            source_id.display_text().unwrap_or_default()
        ),
        choice.span.clone(),
        vec![(
            "id".to_owned(),
            string_argument(source_id.display_text().unwrap_or_default()),
        )],
    )
    .with_help_presentation(super::auxiliary_presentation("diagnostic-id-008-help", []))
}

pub(crate) fn duplicate_line_id(line: &Line, id: &LineId, first_span: SourceSpan) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&DUPLICATE_LINE_ID, "diagnostic-id-003"),
        format!("duplicate localisable id `{id}` on line"),
        line.span.clone(),
        vec![("id".to_owned(), string_argument(id.to_string()))],
    )
    .with_related_presentations([related_presentation(
        first_span,
        "diagnostic-id-003-related",
        [],
    )])
    .with_help_presentation(super::auxiliary_presentation("diagnostic-id-003-help", []))
}

pub(crate) fn duplicate_choice_id(
    choice: &Choice,
    id: &ChoiceId,
    first_span: SourceSpan,
) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&DUPLICATE_CHOICE_ID, "diagnostic-id-004"),
        format!("duplicate localisable id `{id}` on choice"),
        choice.span.clone(),
        vec![("id".to_owned(), string_argument(id.to_string()))],
    )
    .with_related_presentations([related_presentation(
        first_span,
        "diagnostic-id-004-related",
        [],
    )])
    .with_help_presentation(super::auxiliary_presentation("diagnostic-id-004-help", []))
}
