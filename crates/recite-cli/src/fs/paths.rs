use std::fs;
use std::path::{Path, PathBuf};

use crate::error::CliError;

pub(crate) fn reject_output_input_alias(
    output: &Path,
    input_files: &[PathBuf],
) -> Result<(), CliError> {
    let Some(output) = canonical_output_path(output) else {
        return Ok(());
    };

    for input in input_files {
        let Ok(input_canonical) = fs::canonicalize(input) else {
            continue;
        };
        if output == input_canonical {
            return Err(CliError::OutputOverwritesInput {
                output: output.clone(),
                input: input.clone(),
            });
        }
    }

    Ok(())
}

pub(super) fn canonical_output_path(output: &Path) -> Option<PathBuf> {
    if let Ok(canonical) = fs::canonicalize(output) {
        return Some(canonical);
    }

    let file_name = output.file_name()?;
    let parent = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    let parent = parent.unwrap_or_else(|| Path::new("."));
    fs::canonicalize(parent)
        .ok()
        .map(|parent| parent.join(file_name))
}

pub(crate) fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub(crate) fn resolve_project_path(project_root: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_owned()
    } else {
        project_root.join(path)
    }
}
