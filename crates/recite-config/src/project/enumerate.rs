use std::path::{Path, PathBuf};

use super::diagnostics::DiscoveryDiagnostic;
use super::glob::GlobPattern;

/// Whether every configured source root and source file was covered.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Coverage {
    Complete,
    Partial,
}

impl Coverage {
    #[must_use]
    pub const fn is_complete(self) -> bool {
        matches!(self, Self::Complete)
    }
}

/// Canonical source root in manifest declaration order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredRoot {
    pub(super) index: usize,
    pub(super) relative: String,
    pub(super) path: PathBuf,
}

impl DiscoveredRoot {
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    #[must_use]
    pub fn relative(&self) -> &str {
        &self.relative
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Stable project-relative key used by all source consumers.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct DocumentKey(String);

impl DocumentKey {
    pub(super) fn new(value: String) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DocumentKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A valid, UTF-8 `.recite` source discovered under a project root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredDocument {
    key: DocumentKey,
    path: PathBuf,
    root_index: usize,
    text: String,
}

impl DiscoveredDocument {
    #[must_use]
    pub fn key(&self) -> &DocumentKey {
        &self.key
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub const fn root_index(&self) -> usize {
        self.root_index
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

pub(super) fn enumerate_root(
    project_root: &Path,
    root: &DiscoveredRoot,
    excludes: &[GlobPattern],
    documents: &mut Vec<DiscoveredDocument>,
    diagnostics: &mut Vec<DiscoveryDiagnostic>,
) {
    collect_directory(
        project_root,
        &root.path,
        root.index,
        excludes,
        documents,
        diagnostics,
    );
}

fn collect_directory(
    project_root: &Path,
    directory: &Path,
    root_index: usize,
    excludes: &[GlobPattern],
    documents: &mut Vec<DiscoveredDocument>,
    diagnostics: &mut Vec<DiscoveryDiagnostic>,
) {
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => {
            diagnostics.push(DiscoveryDiagnostic::ReadDirectory {
                path: directory.to_owned(),
                message: error.to_string(),
            });
            return;
        }
    };
    let mut readable_entries = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => readable_entries.push(entry),
            Err(error) => diagnostics.push(DiscoveryDiagnostic::ReadDirectory {
                path: directory.to_owned(),
                message: error.to_string(),
            }),
        }
    }
    let mut entries = readable_entries;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let file_name = match entry.file_name().to_str() {
            Some(name) => name.to_owned(),
            None => {
                diagnostics.push(DiscoveryDiagnostic::NonUtf8Path { path });
                continue;
            }
        };
        let relative = match path.strip_prefix(project_root) {
            Ok(path) => path.to_string_lossy().replace('\\', "/"),
            Err(_) => {
                diagnostics.push(DiscoveryDiagnostic::FileOutsideProject {
                    path: path.clone(),
                    target: path,
                });
                continue;
            }
        };
        if is_builtin_excluded(&file_name) || excludes.iter().any(|glob| glob.matches(&relative)) {
            continue;
        }

        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) => {
                diagnostics.push(DiscoveryDiagnostic::ReadDirectory {
                    path: path.clone(),
                    message: error.to_string(),
                });
                continue;
            }
        };
        // Symlink directories are intentionally never traversed. Symlink files
        // are accepted only when their canonical target remains in the project.
        if file_type.is_symlink() {
            match std::fs::canonicalize(&path) {
                Ok(target) if !target.starts_with(project_root) => {
                    diagnostics.push(DiscoveryDiagnostic::FileOutsideProject { path, target });
                    continue;
                }
                Ok(target) if target.is_dir() => continue,
                Ok(_) => {}
                Err(_) => {}
            }
        } else if file_type.is_dir() {
            collect_directory(
                project_root,
                &path,
                root_index,
                excludes,
                documents,
                diagnostics,
            );
            continue;
        } else if !file_type.is_file() {
            continue;
        }

        if !file_name.ends_with(".recite") {
            continue;
        }
        let canonical = match std::fs::canonicalize(&path) {
            Ok(path) if path.starts_with(project_root) => path,
            Ok(target) => {
                diagnostics.push(DiscoveryDiagnostic::FileOutsideProject { path, target });
                continue;
            }
            Err(error) => {
                diagnostics.push(DiscoveryDiagnostic::ReadDirectory {
                    path,
                    message: error.to_string(),
                });
                continue;
            }
        };
        let key = match canonical.strip_prefix(project_root) {
            Ok(relative) => relative.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        let text = match std::fs::read(&canonical).and_then(|bytes| {
            String::from_utf8(bytes).map_err(|error| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, error.utf8_error())
            })
        }) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::InvalidData => {
                diagnostics.push(DiscoveryDiagnostic::NonUtf8Source { path: canonical });
                continue;
            }
            Err(error) => {
                diagnostics.push(DiscoveryDiagnostic::ReadDirectory {
                    path: canonical,
                    message: error.to_string(),
                });
                continue;
            }
        };
        if !documents.iter().any(|document| document.path == canonical) {
            documents.push(DiscoveredDocument {
                key: DocumentKey::new(key),
                path: canonical,
                root_index,
                text,
            });
        }
    }
}

fn is_builtin_excluded(name: &str) -> bool {
    name.starts_with('.')
        || matches!(
            name,
            "target" | "build" | "dist" | "out" | "generated" | "vendor" | "node_modules"
        )
}
