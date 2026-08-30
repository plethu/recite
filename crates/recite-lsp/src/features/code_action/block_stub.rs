use lsp_types::{CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams};
use recite_compiler::{AuthoringEditOperation, AuthoringEditPlan, AuthoringSnapshot, SourceRange};
use recite_ui::{MsgId, UiCatalog};

use super::CodeActionDocument;
use crate::edit_projection::{EditDocument, project_plan};
use crate::position::lsp_position_to_source;

pub(super) fn actions(
    params: &CodeActionParams,
    document: &CodeActionDocument<'_>,
    snapshot: &AuthoringSnapshot,
    documents: &[EditDocument<'_>],
    catalog: &UiCatalog,
) -> Vec<CodeActionOrCommand> {
    let Some(plan) = plan_for_range(snapshot, document, params.range) else {
        return Vec::new();
    };
    let AuthoringEditOperation::CreateBlockStub { block, .. } = plan.operation() else {
        return Vec::new();
    };
    let Some(edit) = project_plan(&plan, snapshot, documents) else {
        return Vec::new();
    };
    vec![CodeActionOrCommand::CodeAction(CodeAction {
        title: catalog.format_pairs(
            MsgId::LspCodeActionCreateBlockStub,
            [("block", block.as_str())],
        ),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(params.context.diagnostics.clone()),
        edit: Some(edit),
        ..CodeAction::default()
    })]
}

fn plan_for_range(
    snapshot: &AuthoringSnapshot,
    document: &CodeActionDocument<'_>,
    range: lsp_types::Range,
) -> Option<AuthoringEditPlan> {
    let start = lsp_position_to_source(document.source.text, range.start)?;
    let end = lsp_position_to_source(document.source.text, range.end)?;
    let source_range = SourceRange::new(start, end);
    snapshot
        .plan_create_block_stub_in_range(document.source.key, source_range)
        .ok()
}
