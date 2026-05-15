use recite_core::CompiledDialogue;

use crate::session::DialogueSessionOptions;
use crate::{DialogueError, DialogueSession};

use super::AssetView;

pub fn start_scene(
    asset: &CompiledDialogue,
    block: Option<&str>,
) -> Result<DialogueSession, DialogueError> {
    start_scene_with_options(asset, block, DialogueSessionOptions::default())
}

/// Start a dialogue session at the compiled default block or an explicit block
/// with explicit runtime options.
pub fn start_scene_with_options(
    asset: &CompiledDialogue,
    block: Option<&str>,
    options: DialogueSessionOptions,
) -> Result<DialogueSession, DialogueError> {
    let asset_view = AssetView::new(asset)?;

    let block_index = match block {
        Some(block) => asset_view.lookup_block(block)?,
        None => asset_view.default_block(),
    };
    let compiled_block = asset_view.block_at(block_index)?;

    Ok(DialogueSession::new(
        &asset.header,
        asset.sources.clone(),
        block_index,
        compiled_block.statements,
        options,
    ))
}
