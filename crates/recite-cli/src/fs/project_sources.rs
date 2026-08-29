use std::fs;
use std::path::{Path, PathBuf};

use recite_core::CompiledSourceFile;

use crate::error::CliError;

pub(super) fn read_project_sources(
    project_root: &Path,
    asset_path: &Path,
    sources: &[CompiledSourceFile],
) -> Result<Vec<(String, Option<String>)>, CliError> {
    sources
        .iter()
        .map(|source| {
            let current_source = read_source_candidates(project_source_candidates(
                project_root,
                asset_path,
                &source.path,
            ))?;
            Ok((source.path.clone(), current_source))
        })
        .collect()
}

fn read_source_candidates(candidates: Vec<PathBuf>) -> Result<Option<String>, CliError> {
    for path in candidates {
        match fs::read_to_string(&path) {
            Ok(source) => return Ok(Some(source)),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => continue,
            Err(source) => return Err(CliError::Read { path, source }),
        }
    }

    Ok(None)
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
