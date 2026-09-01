use std::path::{Path, PathBuf};

use super::super::diagnostics::ProjectDiscoveryError;
use super::PROJECT_MANIFEST_FILE;

pub(super) fn find_manifest(path: &Path) -> Result<(PathBuf, PathBuf), ProjectDiscoveryError> {
    let starting_path = if path.as_os_str().is_empty() {
        Path::new(".")
    } else {
        path
    };
    let canonical_start = if starting_path.exists() {
        std::fs::canonicalize(starting_path).map_err(|error| ProjectDiscoveryError::Read {
            path: starting_path.to_owned(),
            message: error.to_string(),
        })?
    } else {
        let mut existing = starting_path.to_owned();
        while !existing.exists() {
            let Some(parent) = existing.parent() else {
                return Err(ProjectDiscoveryError::NotFound {
                    start: starting_path.to_owned(),
                });
            };
            if parent == existing {
                return Err(ProjectDiscoveryError::NotFound {
                    start: starting_path.to_owned(),
                });
            }
            existing = parent.to_owned();
        }
        std::fs::canonicalize(existing).map_err(|error| ProjectDiscoveryError::Read {
            path: starting_path.to_owned(),
            message: error.to_string(),
        })?
    };
    let mut directory = if canonical_start.is_dir() {
        canonical_start
    } else {
        canonical_start
            .parent()
            .unwrap_or(&canonical_start)
            .to_owned()
    };
    loop {
        let candidate = directory.join(PROJECT_MANIFEST_FILE);
        match std::fs::metadata(&candidate) {
            Ok(metadata) if metadata.is_file() => return Ok((directory, candidate)),
            Ok(_) => {
                return Err(ProjectDiscoveryError::Read {
                    path: candidate,
                    message: "manifest path is not a regular file".to_owned(),
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(ProjectDiscoveryError::Read {
                    path: candidate,
                    message: error.to_string(),
                });
            }
        }
        let Some(parent) = directory.parent() else {
            break;
        };
        if parent == directory {
            break;
        }
        directory = parent.to_owned();
    }
    Err(ProjectDiscoveryError::NotFound {
        start: starting_path.to_owned(),
    })
}
