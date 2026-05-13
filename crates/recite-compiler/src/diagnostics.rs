use recite_core::{
    Block, BlockId, BlockReference, Choice, Diagnostic, DiagnosticCode, DiagnosticSeverity, Line,
    RelatedSpan, SourceSpan,
};

pub(crate) const MISSING_LINE_ID: &str = "RECITE_VALIDATE001";
pub(crate) const MISSING_CHOICE_ID: &str = "RECITE_VALIDATE002";
pub(crate) const DUPLICATE_LINE_ID: &str = "RECITE_VALIDATE003";
pub(crate) const DUPLICATE_CHOICE_ID: &str = "RECITE_VALIDATE004";
pub(crate) const MISSING_DEFAULT_BLOCK: &str = "RECITE_VALIDATE005";
pub(crate) const AMBIGUOUS_DEFAULT_BLOCK: &str = "RECITE_VALIDATE006";
pub(crate) const UNKNOWN_BLOCK_REFERENCE: &str = "RECITE_VALIDATE007";
pub(crate) const INVALID_SOURCE_SPAN: &str = "RECITE_VALIDATE008";
pub(crate) const DUPLICATE_BLOCK_ID: &str = "RECITE_VALIDATE009";

pub(crate) fn missing_line_id(line: &Line) -> Diagnostic {
    diagnostic(
        MISSING_LINE_ID,
        "line header must include a stable line id",
        line.span.clone(),
    )
    .with_help("add a stable author-visible ID to the line header")
}

pub(crate) fn missing_choice_id(choice: &Choice) -> Diagnostic {
    diagnostic(
        MISSING_CHOICE_ID,
        "choice header must include a stable choice id",
        choice.span.clone(),
    )
    .with_help("add a stable author-visible ID to the choice header")
}

pub(crate) fn duplicate_line_id(line: &Line, first_span: SourceSpan) -> Diagnostic {
    let id = line.id.as_ref().expect("duplicate line IDs have an ID");
    diagnostic(
        DUPLICATE_LINE_ID,
        format!("duplicate localisable id `{id}` on line"),
        line.span.clone(),
    )
    .with_related([RelatedSpan::new(first_span, "first localisable ID is here")])
    .with_help("rename one of the duplicate localisable IDs")
}

pub(crate) fn duplicate_choice_id(choice: &Choice, first_span: SourceSpan) -> Diagnostic {
    let id = choice.id.as_ref().expect("duplicate choice IDs have an ID");
    diagnostic(
        DUPLICATE_CHOICE_ID,
        format!("duplicate localisable id `{id}` on choice"),
        choice.span.clone(),
    )
    .with_related([RelatedSpan::new(first_span, "first localisable ID is here")])
    .with_help("rename one of the duplicate localisable IDs")
}

pub(crate) fn missing_default_block(span: SourceSpan) -> Diagnostic {
    diagnostic(
        MISSING_DEFAULT_BLOCK,
        "project must declare exactly one default block",
        span,
    )
    .with_help("mark one block header with `default`")
}

pub(crate) fn ambiguous_default_block(block: &Block, first: &Block) -> Diagnostic {
    diagnostic(
        AMBIGUOUS_DEFAULT_BLOCK,
        format!("block `{}` is another default block", block.id),
        block.span.clone(),
    )
    .with_related([RelatedSpan::new(
        first.span.clone(),
        "first default block is here",
    )])
    .with_help("keep exactly one block marked `default`")
}

pub(crate) fn unknown_block_reference(reference: &BlockReference, span: SourceSpan) -> Diagnostic {
    diagnostic(
        UNKNOWN_BLOCK_REFERENCE,
        format!("unknown block reference `{}`", display_reference(reference)),
        span,
    )
}

pub(crate) fn invalid_source_span(span: SourceSpan, owner: &str, detail: &str) -> Diagnostic {
    diagnostic(
        INVALID_SOURCE_SPAN,
        format!("invalid source span for {owner}: {detail}"),
        span,
    )
}

pub(crate) fn duplicate_block_id(
    block_id: &BlockId,
    span: SourceSpan,
    first_span: SourceSpan,
) -> Diagnostic {
    diagnostic(
        DUPLICATE_BLOCK_ID,
        format!("duplicate block id `{block_id}`"),
        span,
    )
    .with_related([RelatedSpan::new(first_span, "first block ID is here")])
    .with_help("rename one of the duplicate block IDs")
}

fn diagnostic(code: &str, message: impl Into<String>, span: SourceSpan) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::new(code).expect("compiler diagnostic codes are static and namespaced"),
        DiagnosticSeverity::Error,
        message,
        span,
    )
}

fn display_reference(reference: &BlockReference) -> String {
    match &reference.file {
        Some(file) => format!("{file}::{}", reference.block_id),
        None => reference.block_id.to_string(),
    }
}
