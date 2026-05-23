use std::fs;
use std::path::{Path, PathBuf};

use recite_compiler::{
    CompileInput, CompileOptions, validate_source_files, validate_source_files_with_schema,
};
use recite_core::{
    CompiledAssetId, CompilerVersion, ProjectSchema, SchemaFingerprint, SourceMapId,
};
use recite_parser::parse;

use super::paths::{canonical_output_path, display_path};
use crate::diagnostics::InputDiagnostics;
use crate::error::CliError;

pub(crate) fn validate_inputs(
    paths: &[PathBuf],
    schema: Option<&ProjectSchema>,
) -> Result<InputDiagnostics, CliError> {
    let inputs = read_compile_inputs(paths)?;
    let mut source_files = Vec::new();
    let mut parse_diagnostics = Vec::new();

    for input in inputs {
        let parse = parse(&input.path, &input.source);
        let lowered = parse.lower_source_file();
        parse_diagnostics.extend(lowered.diagnostics);
        source_files.push(lowered.source_file);
    }

    let validation_diagnostics = if parse_diagnostics.is_empty() {
        if let Some(schema) = schema {
            validate_source_files_with_schema(&source_files, schema)
        } else {
            validate_source_files(&source_files)
        }
        .diagnostics
    } else {
        Vec::new()
    };

    Ok(InputDiagnostics {
        parse_diagnostics,
        validation_diagnostics,
    })
}

fn read_compile_inputs(paths: &[PathBuf]) -> Result<Vec<CompileInput>, CliError> {
    read_compile_inputs_from_files(collect_input_files(paths)?)
}

pub(crate) fn collect_input_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>, CliError> {
    let mut files = Vec::new();
    for path in paths {
        collect_recite_files(path, &mut files)?;
    }
    files.sort();

    if files.is_empty() {
        return Err(CliError::NoInputs);
    }

    Ok(files)
}

pub(crate) fn read_compile_inputs_from_files(
    files: Vec<PathBuf>,
) -> Result<Vec<CompileInput>, CliError> {
    files
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path).map_err(|source| CliError::Read {
                path: path.clone(),
                source,
            })?;
            Ok(CompileInput::new(display_path(&path), source))
        })
        .collect()
}

pub(crate) fn read_compile_inputs_for_output(
    output: &Path,
    files: Vec<PathBuf>,
) -> Result<Vec<CompileInput>, CliError> {
    let project_root = compile_path_root(output, &files);
    files
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path).map_err(|source| CliError::Read {
                path: path.clone(),
                source,
            })?;
            let input_path = project_root
                .as_ref()
                .and_then(|root| project_relative_path(root, &path))
                .unwrap_or_else(|| display_path(&path));
            Ok(CompileInput::new(input_path, source))
        })
        .collect()
}

fn compile_path_root(output: &Path, files: &[PathBuf]) -> Option<PathBuf> {
    let output = canonical_output_path(output)?;
    let mut root = output.parent()?.to_owned();

    for file in files {
        let canonical = fs::canonicalize(file).ok()?;
        root = common_path_prefix(&root, &canonical)?;
    }

    (root.components().count() > 1).then_some(root)
}

fn project_relative_path(root: &Path, path: &Path) -> Option<String> {
    let canonical = fs::canonicalize(path).ok()?;
    canonical
        .strip_prefix(root)
        .ok()
        .map(display_path)
        .filter(|path| !path.is_empty())
}

fn common_path_prefix(left: &Path, right: &Path) -> Option<PathBuf> {
    let mut prefix = PathBuf::new();
    for (left_component, right_component) in left.components().zip(right.components()) {
        if left_component != right_component {
            break;
        }
        prefix.push(left_component.as_os_str());
    }

    (!prefix.as_os_str().is_empty()).then_some(prefix)
}

fn collect_recite_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), CliError> {
    if path.is_file() {
        files.push(path.to_owned());
        return Ok(());
    }

    if !path.is_dir() {
        return Err(CliError::MissingPath(path.to_owned()));
    }

    for entry in fs::read_dir(path).map_err(|source| CliError::ReadDir {
        path: path.to_owned(),
        source,
    })? {
        let entry = entry.map_err(|source| CliError::ReadDir {
            path: path.to_owned(),
            source,
        })?;
        let child = entry.path();
        if child.is_dir() {
            collect_recite_files(&child, files)?;
        } else if child
            .extension()
            .is_some_and(|extension| extension == "recite")
        {
            files.push(child);
        }
    }

    Ok(())
}

pub(crate) fn compile_options(
    output: &Path,
    schema: Option<&ProjectSchema>,
) -> Result<CompileOptions, CliError> {
    let output = display_path(output);
    let source_map = format!("{output}.map");
    Ok(CompileOptions::new(
        CompilerVersion::new(env!("CARGO_PKG_VERSION"))?,
        CompiledAssetId::new(output)?,
        SourceMapId::new(source_map)?,
        schema.map_or(
            SchemaFingerprint::NoSchema,
            ProjectSchema::canonical_fingerprint,
        ),
    ))
}
