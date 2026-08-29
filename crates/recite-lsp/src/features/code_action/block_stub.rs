use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, Range, TextEdit,
};
use recite_ui::{MsgId, UiCatalog};

use super::{CodeActionDocument, end_position, newline_for, ranges_intersect, workspace_edit};
use crate::summary::BlockReferenceSummary;

pub(super) fn actions(
    params: &CodeActionParams,
    document: &CodeActionDocument<'_>,
    documents: &[CodeActionDocument<'_>],
    catalog: &UiCatalog,
) -> Vec<CodeActionOrCommand> {
    if !document.summary.completeness.block_references {
        return Vec::new();
    }

    document
        .summary
        .block_references
        .iter()
        .filter(|reference| {
            let reference_range = crate::position::span_to_range(document.text, &reference.span);
            ranges_intersect(params.range, reference_range)
        })
        .filter_map(|reference| action(params, reference, document, documents, catalog))
        .collect()
}

fn action(
    params: &CodeActionParams,
    reference: &BlockReferenceSummary,
    document: &CodeActionDocument<'_>,
    documents: &[CodeActionDocument<'_>],
    catalog: &UiCatalog,
) -> Option<CodeActionOrCommand> {
    let target = target_document(reference, document, documents)?;
    if !target.summary.completeness.block_definitions {
        return None;
    }
    if target
        .summary
        .blocks
        .iter()
        .any(|block| block.name == reference.block_id)
    {
        return None;
    }

    let position = end_position(target.text);
    Some(CodeActionOrCommand::CodeAction(CodeAction {
        title: catalog.format_pairs(
            MsgId::LspCodeActionCreateBlockStub,
            [("block", reference.block_id.as_str())],
        ),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(params.context.diagnostics.clone()),
        edit: Some(workspace_edit(
            target.uri.clone(),
            target.summary.version,
            vec![TextEdit {
                range: Range {
                    start: position,
                    end: position,
                },
                new_text: stub_text(target.text, &reference.block_id),
            }],
        )),
        ..CodeAction::default()
    }))
}

fn target_document<'a>(
    reference: &BlockReferenceSummary,
    document: &'a CodeActionDocument<'_>,
    documents: &'a [CodeActionDocument<'_>],
) -> Option<&'a CodeActionDocument<'a>> {
    let Some(file) = &reference.file else {
        return Some(document);
    };
    let mut matches = documents
        .iter()
        .filter(|document| document.summary.project_relative_path() == Some(file.as_str()));
    let target = matches.next()?;
    matches.next().is_none().then_some(target)
}

fn stub_text(text: &str, block_id: &str) -> String {
    let newline = newline_for(text);
    let prefix = if text.is_empty() || text.ends_with('\n') {
        ""
    } else {
        newline
    };
    format!("{prefix}:: {block_id}{newline}")
}
