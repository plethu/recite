use lsp_types::{CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams};
use recite_compiler::{
    AuthoringEditError, AuthoringEditOperation, AuthoringEditPlan, AuthoringSnapshot,
};
use recite_ui::{MsgId, UiCatalog};

use super::CodeActionDocument;
use crate::edit_projection::{EditDocument, project_plan};
use crate::position::source_positions_in_lsp_range;

pub(super) fn actions(
    params: &CodeActionParams,
    document: &CodeActionDocument<'_>,
    snapshot: &AuthoringSnapshot,
    documents: &[EditDocument],
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
    let mut selected = None;
    for position in source_positions_in_lsp_range(&document.source.text, range)? {
        match snapshot.plan_create_block_stub(&document.source.key, position) {
            Ok(plan) => match &selected {
                None => selected = Some(plan),
                Some(existing) if existing == &plan => {}
                Some(_) => return None,
            },
            Err(AuthoringEditError::NoSymbol { .. }) => continue,
            Err(_) => return None,
        }
    }
    selected
}
