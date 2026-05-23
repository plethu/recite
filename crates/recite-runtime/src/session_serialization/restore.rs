use recite_core::{BlockIndex, CompiledDialogue, StatementIndex};

use crate::session_snapshot::{
    DialogueSessionSnapshot, SESSION_SNAPSHOT_FORMAT_VERSION_V0, statement_range,
};
use crate::traversal::AssetView;
use crate::{DialogueError, DialogueSession};

use super::identity::ensure_snapshot_matches_asset;
use super::prompt::restore_pending_prompt;
use super::references::{restore_choice_ids, restore_effects, restore_locale, snapshot_reference};
use super::stack::{restore_frames, validate_range_stack, validate_statement_pointer};

pub fn restore_session(
    asset: &CompiledDialogue,
    snapshot: DialogueSessionSnapshot,
) -> Result<DialogueSession, DialogueError> {
    if snapshot.snapshot_format_version != SESSION_SNAPSHOT_FORMAT_VERSION_V0 {
        return Err(DialogueError::UnsupportedSessionSnapshotFormat {
            snapshot_format_version: snapshot.snapshot_format_version,
        });
    }

    let asset_view = AssetView::new(asset)?;
    ensure_snapshot_matches_asset(asset, &snapshot)?;

    let current_block = BlockIndex::new(snapshot.current_block);
    let block = snapshot_reference("current block", asset_view.block_at(current_block))?;

    let current_range = statement_range(snapshot.current_range);
    snapshot_reference("current range", asset_view.statement_range(current_range))?;

    let next_statement = StatementIndex::new(snapshot.next_statement);
    validate_statement_pointer("next statement", current_range, next_statement)?;

    let continuation_stack = restore_frames(asset_view, &snapshot.continuation_stack)?;
    validate_range_stack(
        asset_view,
        block.statements,
        current_range,
        &continuation_stack,
    )?;
    let previous_prompt_choices = restore_choice_ids(
        asset_view,
        "previous prompt choices",
        &snapshot.previous_prompt_choices,
    )?;
    let selected_choice_history = restore_choice_ids(
        asset_view,
        "selected choice history",
        &snapshot.selected_choice_history,
    )?;
    let deferred_effects = restore_effects(asset_view, &snapshot.deferred_effects)?;
    let locale = restore_locale(snapshot.locale.as_deref())?;
    let pending_prompt = restore_pending_prompt(
        asset_view,
        snapshot.pending_prompt.as_ref(),
        &previous_prompt_choices,
        snapshot.ended,
        current_range,
        next_statement,
    )?;

    Ok(DialogueSession {
        asset_id: asset.header.asset_id.clone(),
        format_version: asset.header.format_version,
        compiler_compatibility_version: asset.header.compiler_compatibility_version,
        compiler_version: asset.header.compiler_version.clone(),
        source_map_id: asset.header.source_map_id.clone(),
        schema_fingerprint: asset.header.schema_fingerprint.clone(),
        sources: asset.sources.clone(),
        current_block,
        current_range,
        next_statement,
        continuation_stack,
        pending_prompt,
        pending_effect: None,
        previous_prompt_choices,
        selected_choice_history,
        deferred_effects,
        locale,
        trace_counter: snapshot.trace_counter,
        ended: snapshot.ended,
    })
}
