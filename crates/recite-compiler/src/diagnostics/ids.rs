use recite_core::{
    Choice, ChoiceId, Diagnostic, DiagnosticCode, Line, LineId, RelatedSpan, SourceId, SourceSpan,
};

const MISSING_LINE_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_ID001");
const MISSING_CHOICE_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_ID002");
const DUPLICATE_LINE_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_ID003");
const DUPLICATE_CHOICE_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_ID004");
const DRAFT_LINE_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_ID005");
const DRAFT_CHOICE_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_ID006");
const MALFORMED_LINE_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_ID007");
const MALFORMED_CHOICE_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_ID008");

pub(crate) fn missing_line_id(line: &Line) -> Diagnostic {
    Diagnostic::error(
        MISSING_LINE_ID,
        "line header must include a stable line id",
        line.span.clone(),
    )
    .with_help("add a stable author-visible ID to the line header")
}

pub(crate) fn missing_choice_id(choice: &Choice) -> Diagnostic {
    Diagnostic::error(
        MISSING_CHOICE_ID,
        "choice header must include a stable choice id",
        choice.span.clone(),
    )
    .with_help("add a stable author-visible ID to the choice header")
}

pub(crate) fn draft_line_id(line: &Line) -> Diagnostic {
    Diagnostic::error(
        DRAFT_LINE_ID,
        "line header has an unfrozen draft source id",
        line.span.clone(),
    )
    .with_help("freeze the line ID as `label@20hexanchor`")
}

pub(crate) fn draft_choice_id(choice: &Choice) -> Diagnostic {
    Diagnostic::error(
        DRAFT_CHOICE_ID,
        "choice header has an unfrozen draft source id",
        choice.span.clone(),
    )
    .with_help("freeze the choice ID as `label@20hexanchor`")
}

pub(crate) fn malformed_line_id(line: &Line, source_id: &SourceId) -> Diagnostic {
    Diagnostic::error(
        MALFORMED_LINE_ID,
        format!(
            "line header has malformed source id `{}`",
            source_id.display_text().unwrap_or_default()
        ),
        line.span.clone(),
    )
    .with_help("use `label@20hexanchor`; plain unsuffixed IDs are invalid")
}

pub(crate) fn malformed_choice_id(choice: &Choice, source_id: &SourceId) -> Diagnostic {
    Diagnostic::error(
        MALFORMED_CHOICE_ID,
        format!(
            "choice header has malformed source id `{}`",
            source_id.display_text().unwrap_or_default()
        ),
        choice.span.clone(),
    )
    .with_help("use `label@20hexanchor`; plain unsuffixed IDs are invalid")
}

pub(crate) fn duplicate_line_id(line: &Line, id: &LineId, first_span: SourceSpan) -> Diagnostic {
    Diagnostic::error(
        DUPLICATE_LINE_ID,
        format!("duplicate localisable id `{id}` on line"),
        line.span.clone(),
    )
    .with_related([RelatedSpan::new(first_span, "first localisable ID is here")])
    .with_help("rename one of the duplicate localisable IDs")
}

pub(crate) fn duplicate_choice_id(
    choice: &Choice,
    id: &ChoiceId,
    first_span: SourceSpan,
) -> Diagnostic {
    Diagnostic::error(
        DUPLICATE_CHOICE_ID,
        format!("duplicate localisable id `{id}` on choice"),
        choice.span.clone(),
    )
    .with_related([RelatedSpan::new(first_span, "first localisable ID is here")])
    .with_help("rename one of the duplicate localisable IDs")
}
