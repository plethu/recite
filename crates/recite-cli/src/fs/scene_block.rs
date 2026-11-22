//! The block a compiled asset starts from, when the author does not name one: the block of the
//! project scene that uses that asset.
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use recite_config::discover_project;

use super::paths::{display_path, resolve_project_path};
use crate::error::CliError;

pub(crate) fn start_block(asset: &Path, explicit: Option<&str>) -> Result<String, CliError> {
    if let Some(block) = explicit {
        return Ok(block.to_owned());
    }
    let missing = || CliError::BlockRequired {
        asset: display_path(asset),
    };
    let start = asset
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let project = discover_project(start).map_err(|_| missing())?;
    let manifest = project.manifest();
    let asset = fs::canonicalize(asset).map_err(|_| missing())?;
    let blocks = manifest
        .source()
        .manifest()
        .scenes
        .iter()
        .filter(|scene| {
            fs::canonicalize(resolve_project_path(manifest.project_root(), &scene.asset))
                .is_ok_and(|path| path == asset)
        })
        .map(|scene| scene.block.clone())
        .collect::<BTreeSet<_>>();
    let mut blocks = blocks.into_iter();
    match (blocks.next(), blocks.next()) {
        (Some(block), None) => Ok(block),
        (None, _) => Err(missing()),
        (Some(first), Some(second)) => Err(CliError::BlockAmbiguous {
            asset: display_path(&asset),
            blocks: [first, second].into_iter().chain(blocks).collect(),
        }),
    }
}
