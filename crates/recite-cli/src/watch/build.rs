use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use recite_compiler::{CompileOptions, compile_inputs, compile_inputs_with_schema};
use recite_core::{
    CompiledAssetId, CompilerVersion, Diagnostic, ProjectManifest, ProjectSchema,
    SchemaFingerprint, SourceMapId, project::validate_project_manifest,
};

use super::events::WatchState;
use super::inputs::collect_project_sources;
use crate::diagnostics::report_diagnostics;
use crate::error::CliError;
use crate::fs::{
    display_path, load_schema, read_compile_inputs_for_output, reject_output_input_alias,
    resolve_project_path, validate_project, write_staged,
};

pub(super) fn build_once(
    state: &mut WatchState,
    stderr: &mut dyn Write,
) -> Result<BuildStatus, CliError> {
    let manifest_path = state.manifest_path();
    let manifest_source = fs::read_to_string(&manifest_path).map_err(|source| CliError::Read {
        path: manifest_path.clone(),
        source,
    })?;
    let manifest_file = display_path(&manifest_path);
    let report = ProjectManifest::load_str(manifest_file.clone(), &manifest_source);
    if !report.diagnostics.is_empty() {
        report_diagnostics(stderr, report.diagnostics.iter())?;
        state.schema_path = None;
        return Ok(BuildStatus::Diagnostics);
    }
    let Some(manifest) = report.manifest else {
        return Ok(BuildStatus::Diagnostics);
    };

    state.schema_path = project_schema_path(&state.project_root, &manifest);
    let loaded_schema = load_project_schema(state.schema_path.as_deref())?;
    if !loaded_schema.diagnostics.is_empty() {
        report_diagnostics(stderr, loaded_schema.diagnostics.iter())?;
        return Ok(BuildStatus::Diagnostics);
    }

    let manifest_diagnostics = validate_project_manifest(
        &manifest_file,
        &manifest_source,
        &manifest,
        loaded_schema.schema.as_ref(),
    );
    if !manifest_diagnostics.is_empty() {
        report_diagnostics(stderr, manifest_diagnostics.iter())?;
        return Ok(BuildStatus::Diagnostics);
    }

    let input_files = collect_project_sources(&state.project_root)?;
    if input_files.is_empty() {
        return Err(CliError::NoInputs);
    }

    let mut compiled_assets = Vec::new();
    for target in unique_asset_targets(&state.project_root, &manifest) {
        reject_output_input_alias(&target.write_path, &input_files)?;
        let inputs = read_compile_inputs_for_output(&target.write_path, input_files.clone())?;
        let options =
            compile_options_for_asset_id(&target.asset_id, loaded_schema.schema.as_ref())?;
        let report = if let Some(schema) = &loaded_schema.schema {
            compile_inputs_with_schema(inputs, options, schema)?
        } else {
            compile_inputs(inputs, options)?
        };

        report_diagnostics(stderr, report.diagnostics.iter())?;
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

    let diagnostics = validate_project(state.project_root.clone())?;
    report_diagnostics(stderr, diagnostics.iter())?;
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
