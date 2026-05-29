use recite_core::{
    Choice, ChoiceId, Diagnostic, DiagnosticCode, Line, LineId, RelatedSpan, SourceSpan,
};

const MISSING_LINE_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_ID001");
const MISSING_CHOICE_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_ID002");
const DUPLICATE_LINE_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_ID003");
const DUPLICATE_CHOICE_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_ID004");

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
