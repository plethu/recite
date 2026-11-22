//! Quick fix for dialogue typed on a header: the kernel moves it into the statement's body.
use lsp_types::{CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams};
use recite_compiler::authoring::{AuthoringQuery, SourceRange};
use recite_ui::{MsgId, UiCatalog};

use super::CodeActionDocument;
use crate::edit_projection::{EditDocument, project_plan};
use crate::position::indexed_lsp_position_to_source;

pub(super) fn actions(
    params: &CodeActionParams,
    document: &CodeActionDocument<'_>,
    snapshot: &AuthoringQuery<'_>,
    documents: &[EditDocument<'_>],
    catalog: &UiCatalog,
) -> Vec<CodeActionOrCommand> {
    let Some(source) = snapshot.document(document.source.key) else {
        return Vec::new();
    };
    let (Some(start), Some(end)) = (
        indexed_lsp_position_to_source(source.source_index(), params.range.start),
        indexed_lsp_position_to_source(source.source_index(), params.range.end),
    ) else {
        return Vec::new();
    };
    let Ok(plan) =
        snapshot.plan_move_header_prose_in_range(document.source.key, SourceRange::new(start, end))
    else {
        return Vec::new();
    };
    let Some(edit) = project_plan(&plan, snapshot, documents) else {
        return Vec::new();
    };
    vec![CodeActionOrCommand::CodeAction(CodeAction {
        title: catalog.text(MsgId::LspCodeActionMoveHeaderProse),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(params.context.diagnostics.clone()),
        edit: Some(edit),
        is_preferred: Some(true),
        ..CodeAction::default()
    })]
}
