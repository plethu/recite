use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use recite_compiler::{compile_inputs, compile_inputs_with_schema};
use recite_core::{Diagnostic, ProjectManifest, ProjectSchema, project::validate_project_manifest};

use super::events::WatchState;
use super::inputs::collect_project_sources;
use crate::diagnostics::report_diagnostics;
use crate::error::CliError;
use crate::fs::{
    compile_options, display_path, load_schema, read_compile_inputs_for_output,
    reject_output_input_alias, resolve_project_path, validate_project, write_staged,
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
    let manifest = report
        .manifest
        .expect("manifest is present without diagnostics");

    let loaded_schema = load_project_schema(&state.project_root, &manifest)?;
    state.schema_path = loaded_schema.path;
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
    for output in unique_asset_paths(&state.project_root, &manifest) {
        reject_output_input_alias(&output, &input_files)?;
        let inputs = read_compile_inputs_for_output(&output, input_files.clone())?;
        let options = compile_options(&output, loaded_schema.schema.as_ref())?;
        let report = if let Some(schema) = &loaded_schema.schema {
            compile_inputs_with_schema(inputs, options, schema)?
        } else {
            compile_inputs(inputs, options)?
        };

        report_diagnostics(stderr, report.diagnostics.iter())?;
        let Some(asset) = report.asset else {
            return Ok(BuildStatus::Diagnostics);
        };
        compiled_assets.push((output, asset.messagepack));
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

fn load_project_schema(
    project_root: &Path,
    manifest: &ProjectManifest,
) -> Result<LoadedProjectSchema, CliError> {
    let Some(schema) = manifest.project.schema.as_deref() else {
        return Ok(LoadedProjectSchema {
            path: None,
            schema: None,
            diagnostics: Vec::new(),
        });
    };

    let path = resolve_project_path(project_root, schema);
    let loaded = load_schema(&path)?;
    Ok(LoadedProjectSchema {
        path: Some(path),
        schema: loaded.schema,
        diagnostics: loaded.diagnostics,
    })
}

fn unique_asset_paths(project_root: &Path, manifest: &ProjectManifest) -> Vec<PathBuf> {
    manifest
        .scenes
        .iter()
        .map(|scene| resolve_project_path(project_root, &scene.asset))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[derive(Debug, Eq, PartialEq)]
pub(super) enum BuildStatus {
    Fresh { asset_count: usize },
    Diagnostics,
}

struct LoadedProjectSchema {
    path: Option<PathBuf>,
    schema: Option<ProjectSchema>,
    diagnostics: Vec<Diagnostic>,
}
