use lsp_types::Range;
use recite_compiler::{AuthoringEditError, AuthoringEditPlan, AuthoringSnapshot};

use super::CodeActionDocument;
use crate::edit_projection::{EditDocument, project_plan};
use crate::position::source_positions_in_lsp_range;

pub(super) fn edit(
    document: &CodeActionDocument<'_>,
    snapshot: &AuthoringSnapshot,
    documents: &[EditDocument],
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
    let mut selected = None;
    for position in source_positions_in_lsp_range(&document.source.text, range)? {
        match snapshot.plan_insert_missing_id(&document.source.key, position) {
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

pub(super) fn fix_all(
    snapshot: &AuthoringSnapshot,
    documents: &[EditDocument],
) -> Option<lsp_types::WorkspaceEdit> {
    let plan = snapshot.plan_insert_missing_ids().ok()?;
    project_plan(&plan, snapshot, documents)
}
