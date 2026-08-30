use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use recite_compiler::{AuthoringKernel, AuthoringRequest, SavedDocument, SnapshotGeneration};
use recite_config::discover_project;
use recite_core::{
    Diagnostic, DiagnosticArgumentValue, ProjectFreshnessInput, ProjectManifest,
    ProjectManifestSource, ProjectSchema, SchemaFingerprint,
    project::{
        MISSING_COMPILED_ASSET, validate_project_freshness_source, validate_project_manifest_source,
    },
};

use super::paths::resolve_project_path;
use super::project_asset::decode_project_asset;
use super::project_diagnostics::project_diagnostic;
use super::project_sources::read_project_sources;
use super::schema::{LoadedSchema, load_schema};
use crate::error::CliError;

pub(crate) fn validate_project(project_root: PathBuf) -> Result<Vec<Diagnostic>, CliError> {
    validate_project_with_mode(project_root, ProjectValidationMode::Authoring)
}

pub(crate) fn check_fresh(project_root: PathBuf) -> Result<Vec<Diagnostic>, CliError> {
    validate_project_with_mode(project_root, ProjectValidationMode::FreshnessOnly)
}

#[derive(Clone, Copy)]
enum ProjectValidationMode {
    Authoring,
    FreshnessOnly,
}

fn validate_project_with_mode(
    project_root: PathBuf,
    mode: ProjectValidationMode,
) -> Result<Vec<Diagnostic>, CliError> {
    let report = match discover_project(&project_root) {
        Ok(report) => report,
        Err(recite_config::ProjectDiscoveryError::Malformed { diagnostics, .. }) => {
            return Ok(diagnostics);
        }
        Err(source) => return Ok(vec![source.as_core_diagnostic()]),
    };
    let discovered = report.manifest();
    let project_root = discovered.project_root().to_owned();
    let manifest_source = discovered.source();
    let mut diagnostics = report
        .diagnostics()
        .iter()
        .map(recite_config::DiscoveryDiagnostic::as_core_diagnostic)
        .collect::<Vec<_>>();

    let contains_invalid_utf8_source = report.diagnostics().iter().any(|diagnostic| {
        matches!(
            diagnostic,
            recite_config::DiscoveryDiagnostic::NonUtf8Source { .. }
        )
    });
    if !report.is_complete() && !contains_invalid_utf8_source {
        return Ok(diagnostics);
    }

    let loaded_schema = load_project_schema(&project_root, manifest_source.manifest())?;
    diagnostics.extend(loaded_schema.diagnostics.iter().cloned());
    diagnostics.extend(validate_project_manifest_source(
        manifest_source,
        loaded_schema.schema.as_ref(),
    ));

    if matches!(mode, ProjectValidationMode::Authoring) {
        diagnostics.extend(validate_project_sources(
            report.documents(),
            loaded_schema.schema.as_ref(),
            loaded_schema.diagnostics.is_empty(),
        )?);
    }

    diagnostics.extend(validate_project_asset_freshness(
        &project_root,
        manifest_source,
        match (
            loaded_schema.schema.as_ref(),
            loaded_schema.diagnostics.is_empty(),
        ) {
            (Some(schema), true) => Some(ProjectSchema::canonical_fingerprint(schema)),
            (None, true) => Some(SchemaFingerprint::NoSchema),
            (_, false) => None,
        },
    )?);

    Ok(diagnostics)
}

fn validate_project_sources(
    documents: &[recite_config::DiscoveredDocument],
    schema: Option<&ProjectSchema>,
    schema_is_valid: bool,
) -> Result<Vec<Diagnostic>, CliError> {
    let saved_documents = documents
        .iter()
        .map(|document| SavedDocument::new(document.key().clone(), document.text().to_owned()));
    let mut kernel = match (schema, schema_is_valid) {
        (Some(schema), true) => AuthoringKernel::with_schema(schema.clone()),
        _ => AuthoringKernel::new(),
    };
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            saved_documents,
            std::iter::empty(),
        ))
        .map_err(|error| {
            CliError::Compile(recite_compiler::CompileError::InvalidValidatedInput(
                format!("authoring kernel rejected initial project request: {error}"),
            ))
        })?;

    Ok(kernel.snapshot().diagnostics().iter().cloned().collect())
}

/// Validate decoded project assets against a parsed project manifest.
///
/// Watch/build callers use this after compiling with the same source-backed
/// manifest, avoiding a second TOML parse and preserving its source spans.
pub(crate) fn validate_project_asset_freshness(
    project_root: &Path,
    manifest_source: &ProjectManifestSource,
    current_schema_fingerprint: Option<SchemaFingerprint>,
) -> Result<Vec<Diagnostic>, CliError> {
    let manifest = manifest_source.manifest();
    let mut diagnostics = Vec::new();

    for (scene_index, scene) in manifest.scenes.iter().enumerate() {
        let asset_path = resolve_project_path(project_root, &scene.asset);
        match fs::metadata(&asset_path) {
            Ok(metadata) if metadata.is_file() => {}
            Ok(_) => return Err(CliError::AssetNotFile { path: asset_path }),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                diagnostics.push(project_diagnostic(
                    &MISSING_COMPILED_ASSET,
                    "diagnostic-project-003",
                    format!(
                        "scene '{}' references missing compiled asset '{}'",
                        scene.id, scene.asset
                    ),
                    manifest_source.scene_key_span(scene_index, "asset"),
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
                ));
                continue;
            }
            Err(source) => {
                return Err(CliError::AssetMetadata {
                    path: asset_path,
                    source,
                });
            }
        }

        let bytes = match fs::read(&asset_path) {
            Ok(bytes) => bytes,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                diagnostics.push(project_diagnostic(
                    &MISSING_COMPILED_ASSET,
                    "diagnostic-project-003",
                    format!(
                        "scene '{}' references missing compiled asset '{}'",
                        scene.id, scene.asset
                    ),
                    manifest_source.scene_key_span(scene_index, "asset"),
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
                ));
                continue;
            }
            Err(source) => {
                return Err(CliError::Read {
                    path: asset_path,
                    source,
                });
            }
        };
        let asset = match decode_project_asset(
            &bytes,
            &scene.id,
            &scene.asset,
            manifest_source.scene_key_span(scene_index, "asset"),
        ) {
            Ok(asset) => asset,
            Err(diagnostic) => {
                diagnostics.push(*diagnostic);
                continue;
            }
        };

        let current_sources = read_project_sources(project_root, &asset_path, &asset.sources)?;
        let current_source_map = current_sources
            .iter()
            .map(|(path, source)| (path.as_str(), source.as_deref()))
            .collect::<BTreeMap<_, _>>();
        diagnostics.extend(validate_project_freshness_source(
            manifest_source,
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
