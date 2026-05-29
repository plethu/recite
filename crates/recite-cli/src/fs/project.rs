use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use recite_core::{
    COMPILED_ASSET_FORMAT_VERSION_V0, COMPILER_COMPATIBILITY_VERSION_V0, CompiledAssetDecodeError,
    Diagnostic, ProjectFreshnessInput, ProjectManifest, ProjectSchema, SchemaFingerprint,
    decode_compiled_dialogue_messagepack,
    project::{
        MALFORMED_COMPILED_ASSET, MISSING_COMPILED_ASSET, STALE_COMPILER_COMPATIBILITY,
        project_scene_key_span, validate_project_freshness, validate_project_manifest,
    },
};

use super::paths::{display_path, resolve_project_path};
use super::schema::{LoadedSchema, load_schema};
use crate::error::CliError;

const PROJECT_MANIFEST_FILE: &str = "recite.project.toml";

pub(crate) fn validate_project(project_root: PathBuf) -> Result<Vec<Diagnostic>, CliError> {
    let manifest_path = project_root.join(PROJECT_MANIFEST_FILE);
    let manifest_source = fs::read_to_string(&manifest_path).map_err(|source| CliError::Read {
        path: manifest_path.clone(),
        source,
    })?;
    let manifest_file = display_path(&manifest_path);
    let report = ProjectManifest::load_str(manifest_file.clone(), &manifest_source);
    let mut diagnostics = report.diagnostics;
    let Some(manifest) = report.manifest else {
        return Ok(diagnostics);
    };

    let loaded_schema = load_project_schema(&project_root, &manifest)?;
    diagnostics.extend(loaded_schema.diagnostics.iter().cloned());
    diagnostics.extend(validate_project_manifest(
        &manifest_file,
        &manifest_source,
        &manifest,
        loaded_schema.schema.as_ref(),
    ));

    let current_schema_fingerprint = match loaded_schema.schema.as_ref() {
        Some(schema) => Some(ProjectSchema::canonical_fingerprint(schema)),
        None if loaded_schema.diagnostics.is_empty() => Some(SchemaFingerprint::NoSchema),
        None => None,
    };

    for (scene_index, scene) in manifest.scenes.iter().enumerate() {
        let asset_path = resolve_project_path(&project_root, &scene.asset);
        if !asset_path.is_file() {
            diagnostics.push(Diagnostic::error(
                MISSING_COMPILED_ASSET,
                format!(
                    "scene '{}' references missing compiled asset '{}'",
                    scene.id, scene.asset
                ),
                project_scene_key_span(&manifest_file, &manifest_source, scene_index, "asset"),
            ));
            continue;
        }

        let bytes = fs::read(&asset_path).map_err(|source| CliError::Read {
            path: asset_path.clone(),
            source,
        })?;
        let asset = match decode_compiled_dialogue_messagepack(&bytes) {
            Ok(asset) => asset,
            Err(CompiledAssetDecodeError::UnsupportedFormat {
                format_version,
                compiler_compatibility_version,
            }) if format_version == COMPILED_ASSET_FORMAT_VERSION_V0
                && compiler_compatibility_version != COMPILER_COMPATIBILITY_VERSION_V0 =>
            {
                diagnostics.push(Diagnostic::error(
                    STALE_COMPILER_COMPATIBILITY,
                    format!(
                        "compiled asset '{}' uses compiler compatibility version {}, expected {}",
                        scene.asset,
                        compiler_compatibility_version,
                        COMPILER_COMPATIBILITY_VERSION_V0
                    ),
                    project_scene_key_span(&manifest_file, &manifest_source, scene_index, "asset"),
                ));
                continue;
            }
            Err(error) => {
                diagnostics.push(Diagnostic::error(
                    MALFORMED_COMPILED_ASSET,
                    format!(
                        "scene '{}' references malformed compiled asset '{}': {error}",
                        scene.id, scene.asset
                    ),
                    project_scene_key_span(&manifest_file, &manifest_source, scene_index, "asset"),
                ));
                continue;
            }
        };

        let current_sources = read_project_sources(&project_root, &asset_path, &asset.sources);
        let current_source_map = current_sources
            .iter()
            .map(|(path, source)| (path.as_str(), source.as_deref()))
            .collect::<BTreeMap<_, _>>();
        diagnostics.extend(validate_project_freshness(
            &manifest_file,
            &manifest_source,
            ProjectFreshnessInput {
                scene_index,
                scene,
                asset: &asset,
                current_sources: current_source_map,
                current_schema_fingerprint: current_schema_fingerprint.clone(),
            },
        ));
    }

    Ok(diagnostics)
}

fn load_project_schema(
    project_root: &Path,
    manifest: &ProjectManifest,
) -> Result<LoadedSchema, CliError> {
    let Some(schema_path) = manifest.project.schema.as_deref() else {
        return Ok(LoadedSchema {
            schema: None,
            diagnostics: Vec::new(),
        });
    };

    load_schema(&resolve_project_path(project_root, schema_path))
}

fn read_project_sources(
    project_root: &Path,
    asset_path: &Path,
    sources: &[recite_core::CompiledSourceFile],
) -> Vec<(String, Option<String>)> {
    sources
        .iter()
        .map(|source| {
            let current_source = project_source_candidates(project_root, asset_path, &source.path)
                .into_iter()
                .find_map(|path| fs::read_to_string(path).ok());
            (source.path.clone(), current_source)
        })
        .collect()
}

fn project_source_candidates(
    project_root: &Path,
    asset_path: &Path,
    source_path: &str,
) -> Vec<PathBuf> {
    let source_path = Path::new(source_path);
    if source_path.is_absolute() {
        return vec![source_path.to_owned()];
    }

    let mut candidates = Vec::new();
    let mut ancestor = asset_path.parent();
    while let Some(directory) = ancestor {
        let candidate = directory.join(source_path);
        if !candidates.iter().any(|existing| existing == &candidate) {
            candidates.push(candidate);
        }

        if directory == project_root {
            break;
        }
        ancestor = directory.parent();
    }

    let project_candidate = project_root.join(source_path);
    if !candidates
        .iter()
        .any(|existing| existing == &project_candidate)
    {
        candidates.push(project_candidate);
    }

    candidates
}
