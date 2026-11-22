use std::fs;
use std::path::{Path, PathBuf};

use recite_compiler::{compile::CompileOptions, validation::validate_source_files};
use recite_core::{
    compiled::{CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId},
    schema::ProjectSchema,
};
use recite_parser::parse;

use super::paths::{canonical_output_path, display_path};
use super::sources::{LooseKeys, SourceSet};
use crate::diagnostics::InputDiagnostics;
use crate::error::CliError;

/// Parse and validate the named sources. Inside a project the project's other sources are read as
/// context, so references into them resolve, and only the named sources' diagnostics are kept.
pub(crate) fn validate_inputs(
    paths: &[PathBuf],
    schema: Option<&ProjectSchema>,
) -> Result<InputDiagnostics, CliError> {
    validate_sources(&SourceSet::read(paths, LooseKeys::AsTyped)?, schema)
}

pub(crate) fn validate_sources(
    sources: &SourceSet,
    schema: Option<&ProjectSchema>,
) -> Result<InputDiagnostics, CliError> {
    let file_count = sources.named.len();
    let mut source_files = Vec::new();
    let mut parse_diagnostics = Vec::new();

    for source in sources.named.iter().chain(&sources.context) {
        let parse = parse(&source.key, &source.source);
        let lowered = parse.lower_source_file();
        parse_diagnostics.extend(lowered.diagnostics);
        source_files.push(lowered.source_file);
    }
    parse_diagnostics.retain(|diagnostic| sources.is_named(&diagnostic.span.file));

    let mut validation_diagnostics = if parse_diagnostics.is_empty() {
        if let Some(schema) = schema {
            recite_compiler::validation::validate_inputs(
                source_files
                    .iter()
                    .map(recite_compiler::validation::ValidationInput::all_complete),
                Some(schema),
                recite_compiler::validation::ProjectCompleteness::Complete,
            )
        } else {
            validate_source_files(&source_files)
        }
        .diagnostics
    } else {
        Vec::new()
    };
    // Context sources are read, not validated: their own findings belong to their own command.
    validation_diagnostics.retain(|diagnostic| sources.is_named(&diagnostic.span.file));
    sources.relocate(&mut parse_diagnostics);
    sources.relocate(&mut validation_diagnostics);

    Ok(InputDiagnostics {
        parse_diagnostics,
        validation_diagnostics,
        file_count,
    })
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

pub(super) fn compile_path_root(output: &Path, files: &[PathBuf]) -> Option<PathBuf> {
    let output = canonical_output_path(output)?;
    let mut root = output.parent()?.to_owned();

    for file in files {
        let canonical = fs::canonicalize(file).ok()?;
        root = common_path_prefix(&root, &canonical)?;
    }

    (root.components().count() > 1).then_some(root)
}

pub(super) fn common_path_prefix(left: &Path, right: &Path) -> Option<PathBuf> {
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
