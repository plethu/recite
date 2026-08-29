use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use recite_compiler::{CompileOptions, compile_inputs, compile_inputs_with_schema};
use recite_core::{
    CompiledAssetId, CompilerVersion, Diagnostic, ProjectManifest, ProjectSchema,
    SchemaFingerprint, SourceMapId, project::validate_project_manifest_source,
};

use super::events::WatchState;
use crate::diagnostics::report_diagnostics;
use crate::error::CliError;
use crate::fs::{
    display_path, load_schema, read_compile_inputs_relative_to, reject_output_input_alias,
    resolve_project_path, validate_project_asset_freshness, write_staged,
};
use crate::i18n::Messages;
use recite_config::discover_project;

pub(super) fn build_once(
    state: &mut WatchState,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<BuildStatus, CliError> {
    let report = discover_project(&state.project_root)
        .map_err(|source| CliError::ProjectDiscovery { source })?;
    let discovered = report.manifest();
    state.project_root = discovered.project_root().to_owned();
    state.manifest = Some(discovered.clone());
    let manifest_source = discovered.source();
    let discovery_diagnostics = report
        .diagnostics()
        .iter()
        .map(recite_config::DiscoveryDiagnostic::as_core_diagnostic)
        .collect::<Vec<_>>();
    report_diagnostics(stderr, messages, discovery_diagnostics.iter())?;
    if !report.is_complete() {
        state.schema_path = None;
        return Ok(BuildStatus::Diagnostics);
    }
    let manifest = manifest_source.manifest();

    state.schema_path = project_schema_path(&state.project_root, manifest);
    let loaded_schema = load_project_schema(state.schema_path.as_deref())?;
    if !loaded_schema.diagnostics.is_empty() {
        report_diagnostics(stderr, messages, loaded_schema.diagnostics.iter())?;
        return Ok(BuildStatus::Diagnostics);
    }

    let manifest_diagnostics =
        validate_project_manifest_source(manifest_source, loaded_schema.schema.as_ref());
    if !manifest_diagnostics.is_empty() {
        report_diagnostics(stderr, messages, manifest_diagnostics.iter())?;
        return Ok(BuildStatus::Diagnostics);
    }

    let input_files = report
        .documents()
        .iter()
        .map(|document| document.path().to_owned())
        .collect::<Vec<_>>();
    if input_files.is_empty() {
        return Err(CliError::NoInputs);
    }

    let mut compiled_assets = Vec::new();
    for target in unique_asset_targets(&state.project_root, manifest) {
        reject_output_input_alias(&target.write_path, &input_files)?;
        let inputs = read_compile_inputs_relative_to(&state.project_root, input_files.clone())?;
        let options =
            compile_options_for_asset_id(&target.asset_id, loaded_schema.schema.as_ref())?;
        let report = if let Some(schema) = &loaded_schema.schema {
            compile_inputs_with_schema(inputs, options, schema)?
        } else {
            compile_inputs(inputs, options)?
        };

        report_diagnostics(stderr, messages, report.diagnostics.iter())?;
        let Some(asset) = report.asset else {
            return Ok(BuildStatus::Diagnostics);
        };
        compiled_assets.push((target.write_path, asset.messagepack));
    }

    for (output, bytes) in &compiled_assets {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|source| CliError::Write {
                path: parent.to_owned(),
                source,
            })?;
        }
        write_staged(output, bytes)?;
    }

    let diagnostics = validate_project_asset_freshness(
        &state.project_root,
        manifest_source,
        Some(loaded_schema.schema.as_ref().map_or(
            SchemaFingerprint::NoSchema,
            ProjectSchema::canonical_fingerprint,
        )),
    )?;
    report_diagnostics(stderr, messages, diagnostics.iter())?;
    if diagnostics.is_empty() {
        Ok(BuildStatus::Fresh {
            asset_count: compiled_assets.len(),
        })
    } else {
        Ok(BuildStatus::Diagnostics)
    }
}

fn load_project_schema(schema_path: Option<&Path>) -> Result<LoadedProjectSchema, CliError> {
    let Some(schema_path) = schema_path else {
        return Ok(LoadedProjectSchema {
            schema: None,
            diagnostics: Vec::new(),
        });
    };

    let loaded = load_schema(schema_path)?;
    Ok(LoadedProjectSchema {
        schema: loaded.schema,
        diagnostics: loaded.diagnostics,
    })
}

fn project_schema_path(project_root: &Path, manifest: &ProjectManifest) -> Option<PathBuf> {
    manifest
        .project
        .schema
        .as_deref()
        .map(|schema| resolve_project_path(project_root, schema))
}

fn compile_options_for_asset_id(
    asset_id: &str,
    schema: Option<&ProjectSchema>,
) -> Result<CompileOptions, CliError> {
    Ok(CompileOptions::new(
        CompilerVersion::new(env!("CARGO_PKG_VERSION"))?,
        CompiledAssetId::new(asset_id.to_owned())?,
        SourceMapId::new(format!("{asset_id}.map"))?,
        schema.map_or(
            SchemaFingerprint::NoSchema,
            ProjectSchema::canonical_fingerprint,
        ),
    ))
}

fn unique_asset_targets(project_root: &Path, manifest: &ProjectManifest) -> Vec<AssetTarget> {
    let mut targets = BTreeMap::new();
    for scene in &manifest.scenes {
        targets
            .entry(resolve_project_path(project_root, &scene.asset))
            .or_insert_with(|| display_path(Path::new(&scene.asset)));
    }

    targets
        .into_iter()
        .map(|(write_path, asset_id)| AssetTarget {
            write_path,
            asset_id,
        })
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AssetTarget {
    write_path: PathBuf,
    asset_id: String,
}

#[derive(Debug, Eq, PartialEq)]
pub(super) enum BuildStatus {
    Fresh { asset_count: usize },
    Diagnostics,
}

struct LoadedProjectSchema {
    schema: Option<ProjectSchema>,
    diagnostics: Vec<Diagnostic>,
}
