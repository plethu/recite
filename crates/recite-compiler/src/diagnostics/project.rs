use recite_core::{Block, BlockId, BlockReference, SourceFile, SourceSpan};

use super::{compiler_diagnostic, diagnostic_contract, related_presentation, string_argument};

const MISSING_DEFAULT_BLOCK: recite_core::DiagnosticCode =
    recite_core::DiagnosticCode::new_static("RECITE_VALIDATE005");
const AMBIGUOUS_DEFAULT_BLOCK: recite_core::DiagnosticCode =
    recite_core::DiagnosticCode::new_static("RECITE_VALIDATE006");
const UNKNOWN_BLOCK_REFERENCE: recite_core::DiagnosticCode =
    recite_core::DiagnosticCode::new_static("RECITE_VALIDATE007");
const DUPLICATE_BLOCK_ID: recite_core::DiagnosticCode =
    recite_core::DiagnosticCode::new_static("RECITE_VALIDATE009");
const DUPLICATE_SOURCE_PATH: recite_core::DiagnosticCode =
    recite_core::DiagnosticCode::new_static("RECITE_VALIDATE010");
const AMBIGUOUS_COMPILED_BLOCK_ID: recite_core::DiagnosticCode =
    recite_core::DiagnosticCode::new_static("RECITE_VALIDATE011");

pub(crate) fn missing_default_block(span: SourceSpan) -> recite_core::Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&MISSING_DEFAULT_BLOCK, "diagnostic-validate-005"),
        "project must declare exactly one default block",
        span,
        [],
    )
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-005-help",
        [],
    ))
}

pub(crate) fn ambiguous_default_block(block: &Block, first: &Block) -> recite_core::Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&AMBIGUOUS_DEFAULT_BLOCK, "diagnostic-validate-006"),
        format!("block `{}` is another default block", block.id),
        block.span.clone(),
        vec![("block_id".to_owned(), string_argument(block.id.to_string()))],
    )
    .with_related_presentations([related_presentation(
        first.span.clone(),
        "diagnostic-validate-006-related",
        [],
    )])
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-006-help",
        [],
    ))
}

pub(crate) fn unknown_block_reference(
    reference: &BlockReference,
    span: SourceSpan,
) -> recite_core::Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&UNKNOWN_BLOCK_REFERENCE, "diagnostic-validate-007"),
        format!("unknown block reference `{}`", display_reference(reference)),
        span,
        vec![(
            "reference".to_owned(),
            string_argument(display_reference(reference)),
        )],
    )
}

pub(crate) fn duplicate_block_id(
    block_id: &BlockId,
    span: SourceSpan,
    first_span: SourceSpan,
) -> recite_core::Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&DUPLICATE_BLOCK_ID, "diagnostic-validate-009"),
        format!("duplicate block id `{block_id}`"),
        span,
        vec![("block_id".to_owned(), string_argument(block_id.to_string()))],
    )
    .with_related_presentations([related_presentation(
        first_span,
        "diagnostic-validate-009-related",
        [],
    )])
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-009-help",
        [],
    ))
}

pub(crate) fn duplicate_source_path(
    source_file: &SourceFile,
    first_span: SourceSpan,
) -> recite_core::Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&DUPLICATE_SOURCE_PATH, "diagnostic-validate-010"),
        format!("duplicate source path `{}`", source_file.path),
        first_span_for(source_file),
        vec![("path".to_owned(), string_argument(source_file.path.clone()))],
    )
    .with_related_presentations([related_presentation(
        first_span,
        "diagnostic-validate-010-related",
        [],
    )])
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-010-help",
        [],
    ))
}

pub(crate) fn ambiguous_compiled_block_id(
    block_id: &BlockId,
    span: SourceSpan,
    first_span: SourceSpan,
) -> recite_core::Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&AMBIGUOUS_COMPILED_BLOCK_ID, "diagnostic-validate-011"),
        format!("compiled block id `{block_id}` must be globally unique"),
        span,
        vec![("block_id".to_owned(), string_argument(block_id.to_string()))],
    )
    .with_related_presentations([related_presentation(
        first_span,
        "diagnostic-validate-011-related",
        [],
    )])
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-011-help",
        [],
    ))
}

fn display_reference(reference: &BlockReference) -> String {
    match &reference.file {
        Some(file) => format!("{file}::{}", reference.block_id),
        None => reference.block_id.to_string(),
    }
}

// Invariant: 1:1 is a valid fallback source position for source files without blocks.
#[allow(clippy::expect_used)]
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
