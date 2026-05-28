use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::error::CliError;

pub(super) fn collect_project_sources(project_root: &Path) -> Result<Vec<PathBuf>, CliError> {
    let mut files = Vec::new();
    collect_project_sources_inner(project_root, project_root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_project_sources_inner(
    project_root: &Path,
    directory: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), CliError> {
    for entry in fs::read_dir(directory).map_err(|source| CliError::ReadDir {
        path: directory.to_owned(),
        source,
    })? {
        let entry = entry.map_err(|source| CliError::ReadDir {
            path: directory.to_owned(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() {
            if should_skip_directory(project_root, &path) {
                continue;
            }
            collect_project_sources_inner(project_root, &path, files)?;
        } else if is_project_recite_source(project_root, &path) {
            files.push(path);
        }
    }
    Ok(())
}

fn should_skip_directory(project_root: &Path, path: &Path) -> bool {
    path.strip_prefix(project_root)
        .ok()
        .and_then(|relative| relative.components().next_back())
        .is_some_and(|component| match component {
            Component::Normal(name) => {
                let name = name.to_string_lossy();
                name == "target" || name.starts_with('.')
            }
            _ => false,
        })
}

pub(super) fn is_project_recite_source(project_root: &Path, path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "recite")
        && path.strip_prefix(project_root).is_ok_and(|relative| {
            relative.components().all(|component| match component {
                Component::Normal(name) => {
                    let name = name.to_string_lossy();
                    !name.starts_with('.') && name != "target"
                }
                _ => true,
            })
        })
}

pub(super) fn is_generated_output_path(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "recitec")
        || path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with('.') && name.ends_with(".tmp"))
}
