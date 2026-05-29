use recite_core::{
    Block, BlockId, BlockReference, Diagnostic, DiagnosticCode, RelatedSpan, SourceFile, SourceSpan,
};

const MISSING_DEFAULT_BLOCK: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE005");
const AMBIGUOUS_DEFAULT_BLOCK: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE006");
const UNKNOWN_BLOCK_REFERENCE: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE007");
const DUPLICATE_BLOCK_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE009");
const DUPLICATE_SOURCE_PATH: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE010");
const AMBIGUOUS_COMPILED_BLOCK_ID: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_VALIDATE011");

pub(crate) fn missing_default_block(span: SourceSpan) -> Diagnostic {
    Diagnostic::error(
        MISSING_DEFAULT_BLOCK,
        "project must declare exactly one default block",
        span,
    )
    .with_help("mark one block header with `default`")
}

pub(crate) fn ambiguous_default_block(block: &Block, first: &Block) -> Diagnostic {
    Diagnostic::error(
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
    Diagnostic::error(
        UNKNOWN_BLOCK_REFERENCE,
        format!("unknown block reference `{}`", display_reference(reference)),
        span,
    )
}

pub(crate) fn duplicate_block_id(
    block_id: &BlockId,
    span: SourceSpan,
    first_span: SourceSpan,
) -> Diagnostic {
    Diagnostic::error(
        DUPLICATE_BLOCK_ID,
        format!("duplicate block id `{block_id}`"),
        span,
    )
    .with_related([RelatedSpan::new(first_span, "first block ID is here")])
    .with_help("rename one of the duplicate block IDs")
}

pub(crate) fn duplicate_source_path(
    source_file: &SourceFile,
    first_span: SourceSpan,
) -> Diagnostic {
    Diagnostic::error(
        DUPLICATE_SOURCE_PATH,
        format!("duplicate source path `{}`", source_file.path),
        first_span_for(source_file),
    )
    .with_related([RelatedSpan::new(
        first_span,
        "first source file with this path is here",
    )])
    .with_help("compile each source path once")
}

pub(crate) fn ambiguous_compiled_block_id(
    block_id: &BlockId,
    span: SourceSpan,
    first_span: SourceSpan,
) -> Diagnostic {
    Diagnostic::error(
        AMBIGUOUS_COMPILED_BLOCK_ID,
        format!("compiled block id `{block_id}` must be globally unique"),
        span,
    )
    .with_related([RelatedSpan::new(
        first_span,
        "first compiled block ID is here",
    )])
    .with_help("rename one block or split the runtime lookup contract in a future format version")
}

fn display_reference(reference: &BlockReference) -> String {
    match &reference.file {
        Some(file) => format!("{file}::{}", reference.block_id),
        None => reference.block_id.to_string(),
    }
}

fn first_span_for(source_file: &SourceFile) -> SourceSpan {
    source_file.blocks.first().map_or_else(
        || {
            SourceSpan::point(
                source_file.path.clone(),
                recite_core::SourcePosition::new(1, 1).expect("1:1 is a valid source position"),
            )
        },
        |block| block.span.clone(),
    )
}
