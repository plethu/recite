//! One asset is read and assessed per group; diagnostics retain manifest scene order.
use super::{
    paths::resolve_project_path, project_asset::asset_decode_diagnostic,
    project_diagnostics::project_diagnostic, project_sources::read_project_sources,
};
use crate::error::CliError;
use recite_core::{
    Diagnostic, DiagnosticArgumentValue,
    compiled::{SchemaFingerprint, decode_compiled_dialogue_messagepack},
    project::{MISSING_COMPILED_ASSET, ProjectAssetFreshness, ProjectManifestSource},
};
use std::{collections::BTreeMap, fs, path::Path};

/// Inspect published output even when captured build inputs remain unchanged.
pub(crate) fn validate_project_asset_freshness(
    project_root: &Path,
    source: &ProjectManifestSource,
    current_schema: Option<SchemaFingerprint>,
) -> Result<Vec<Diagnostic>, CliError> {
    let scenes = &source.manifest().scenes;
    let mut by_asset = BTreeMap::new();
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (index, scene) in scenes.iter().enumerate() {
        let group = *by_asset.entry(scene.asset.as_str()).or_insert_with(|| {
            groups.push(Vec::new());
            groups.len() - 1
        });
        groups[group].push(index);
    }
    let mut diagnostics = vec![Vec::new(); scenes.len()];
    // Encounter order preserves the first typed filesystem error. Grouping by
    // exact declared text retains asset-relative source resolution and aliases.
    for group in groups {
        let first = group[0];
        let asset_path = resolve_project_path(project_root, &scenes[first].asset);
        let Some(bytes) = read_asset(&asset_path)? else {
            for index in group {
                diagnostics[index].push(missing_asset(source, index));
            }
            continue;
        };
        let asset = match decode_compiled_dialogue_messagepack(&bytes) {
            Ok(asset) => asset,
            Err(error) => {
                for index in group {
                    let scene = &scenes[index];
                    diagnostics[index].push(asset_decode_diagnostic(
                        &error,
                        &scene.id,
                        &scene.asset,
                        source.scene_key_span(index, "asset"),
                    ));
                }
                continue;
            }
        };
        drop(bytes);
        let current_sources = read_project_sources(project_root, &asset_path, &asset.sources)?;
        let current_map = current_sources
            .iter()
            .map(|(path, text)| (path.as_str(), text.as_deref()))
            .collect();
        let assessment = ProjectAssetFreshness::new(&asset, &current_map, current_schema.clone());
        drop(current_map);
        drop(current_sources);
        for index in group {
            diagnostics[index] = assessment.validate_scene_source(source, index, &scenes[index]);
        }
    }
    Ok(diagnostics.into_iter().flatten().collect())
}

fn read_asset(path: &Path) -> Result<Option<Vec<u8>>, CliError> {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => {
            return Err(CliError::AssetNotFile {
                path: path.to_owned(),
            });
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(CliError::AssetMetadata {
                path: path.to_owned(),
                source,
            });
        }
    }
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(CliError::Read {
            path: path.to_owned(),
            source,
        }),
    }
}

fn missing_asset(source: &ProjectManifestSource, index: usize) -> Diagnostic {
    let scene = &source.manifest().scenes[index];
    project_diagnostic(
        &MISSING_COMPILED_ASSET,
        "diagnostic-project-003",
        format!(
            "scene '{}' references missing compiled asset '{}'",
            scene.id, scene.asset
        ),
        source.scene_key_span(index, "asset"),
        [
            (
                "scene_id",
                DiagnosticArgumentValue::String(scene.id.clone()),
            ),
            (
                "asset",
                DiagnosticArgumentValue::String(scene.asset.clone()),
            ),
        ],
    )
}

#[cfg(test)]
mod tests;
