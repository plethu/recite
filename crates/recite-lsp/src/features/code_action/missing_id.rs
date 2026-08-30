use lsp_types::Range;
use recite_compiler::{AuthoringEditError, AuthoringEditPlan, AuthoringSnapshot, SourceRange};

use super::CodeActionDocument;
use crate::edit_projection::{EditDocument, project_plan};
use crate::position::lsp_position_to_source;

pub(super) fn edit(
    document: &CodeActionDocument<'_>,
    snapshot: &AuthoringSnapshot,
    documents: &[EditDocument<'_>],
    range: Range,
) -> Option<lsp_types::WorkspaceEdit> {
    let plan = plan_for_range(snapshot, document, range)?;
    (plan.edits().len() == 1).then(|| project_plan(&plan, snapshot, documents))?
}

fn plan_for_range(
    snapshot: &AuthoringSnapshot,
    document: &CodeActionDocument<'_>,
    range: Range,
) -> Option<AuthoringEditPlan> {
    let start = lsp_position_to_source(document.source.text, range.start)?;
    let end = lsp_position_to_source(document.source.text, range.end)?;
    let source_range = SourceRange::new(start, end);
    match snapshot.plan_insert_missing_ids_in_range(document.source.key, source_range) {
        Ok(plan) => Some(plan),
        Err(AuthoringEditError::NoEdits | AuthoringEditError::NoSymbol { .. }) => None,
        Err(_) => None,
    }
}

pub(super) fn fix_all(
    document: &CodeActionDocument<'_>,
    snapshot: &AuthoringSnapshot,
    documents: &[EditDocument<'_>],
) -> Option<lsp_types::WorkspaceEdit> {
    let plan = snapshot
        .plan_insert_missing_ids_for_document(document.source.key)
        .ok()?;
    project_plan(&plan, snapshot, documents)
}
