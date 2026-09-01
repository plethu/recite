use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::super::diagnostics::DiscoveryDiagnostic;
use super::super::glob::GlobPattern;
use super::{DiscoveredDocument, DiscoveredRoot, DocumentKey, is_excluded_relative};

#[cfg(test)]
mod tests;

pub(super) fn collect_root(
    project_root: &Path,
    root: &DiscoveredRoot,
    excludes: &[GlobPattern],
    documents: &mut Vec<DiscoveredDocument>,
    diagnostics: &mut Vec<DiscoveryDiagnostic>,
    seen: &mut BTreeMap<PathBuf, usize>,
) {
    let mut enumeration = Enumeration {
        project_root,
        source_root: root.path(),
        root_index: root.index(),
        excludes,
        documents,
        diagnostics,
        seen,
    };
    enumeration.collect_directory(root.path());
}

struct Enumeration<'a> {
    project_root: &'a Path,
    source_root: &'a Path,
    root_index: usize,
    excludes: &'a [GlobPattern],
    documents: &'a mut Vec<DiscoveredDocument>,
    diagnostics: &'a mut Vec<DiscoveryDiagnostic>,
    seen: &'a mut BTreeMap<PathBuf, usize>,
}

impl Enumeration<'_> {
    fn collect_directory(&mut self, directory: &Path) {
        let entries = match std::fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) => {
                self.diagnostics.push(DiscoveryDiagnostic::ReadDirectory {
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
                Err(error) => self.diagnostics.push(DiscoveryDiagnostic::ReadDirectory {
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
                    self.diagnostics
                        .push(DiscoveryDiagnostic::NonUtf8Path { path });
                    continue;
                }
            };
            let relative = match path.strip_prefix(self.project_root) {
                Ok(_) => match project_relative_key(self.project_root, &path) {
                    Some(relative) => relative,
                    None => {
                        self.diagnostics
                            .push(DiscoveryDiagnostic::NonUtf8Path { path });
                        continue;
                    }
                },
                Err(_) => {
                    self.diagnostics
                        .push(DiscoveryDiagnostic::FileOutsideProject {
                            path: path.clone(),
                            target: path,
                        });
                    continue;
                }
            };
            if is_excluded_relative(&relative, self.excludes) {
                continue;
            }

            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    self.diagnostics.push(DiscoveryDiagnostic::ReadDirectory {
                        path: path.clone(),
                        message: error.to_string(),
                    });
                    continue;
                }
            };
            // Symlink directories are intentionally never traversed. Symlink files
            // are accepted only when their canonical target remains in this source
            // root and the project.
            if file_type.is_symlink() {
                match std::fs::canonicalize(&path) {
                    Ok(target)
                        if !target.starts_with(self.project_root)
                            || !target.starts_with(self.source_root) =>
                    {
                        self.diagnostics
                            .push(DiscoveryDiagnostic::FileOutsideProject { path, target });
                        continue;
                    }
                    Ok(target) if target.is_dir() => continue,
                    Ok(_) => {}
                    Err(_) => {}
                }
            } else if file_type.is_dir() {
                self.collect_directory(&path);
                continue;
            } else if !file_type.is_file() {
                continue;
            }

            if !file_name.ends_with(".recite") {
                continue;
            }
            let canonical = match std::fs::canonicalize(&path) {
                Ok(path)
                    if path.starts_with(self.project_root)
                        && path.starts_with(self.source_root) =>
                {
                    path
                }
                Ok(target) => {
                    self.diagnostics
                        .push(DiscoveryDiagnostic::FileOutsideProject { path, target });
                    continue;
                }
                Err(error) => {
                    self.diagnostics.push(DiscoveryDiagnostic::ReadDirectory {
                        path,
                        message: error.to_string(),
                    });
                    continue;
                }
            };
            if !canonical
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".recite"))
            {
                continue;
            }
            let key = match project_relative_key(self.project_root, &canonical) {
                Some(key) => key,
                None => {
                    self.diagnostics
                        .push(DiscoveryDiagnostic::NonUtf8Path { path: canonical });
                    continue;
                }
            };
            if is_excluded_relative(&key, self.excludes) {
                continue;
            }
            let text = match std::fs::read(&canonical).and_then(|bytes| {
                String::from_utf8(bytes).map_err(|error| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, error.utf8_error())
                })
            }) {
                Ok(text) => text,
                Err(error) if error.kind() == std::io::ErrorKind::InvalidData => {
                    self.diagnostics
                        .push(DiscoveryDiagnostic::NonUtf8Source { path: canonical });
                    continue;
                }
                Err(error) => {
                    self.diagnostics.push(DiscoveryDiagnostic::ReadDirectory {
                        path: canonical,
                        message: error.to_string(),
                    });
                    continue;
                }
            };
            if let Some(index) = self.seen.get(&canonical).copied() {
                self.documents[index].source_paths.push(path);
            } else {
                let key = match DocumentKey::new(key) {
                    Ok(key) => key,
                    Err(error) => {
                        self.diagnostics
                            .push(DiscoveryDiagnostic::InvalidDocumentKey {
                                path: canonical,
                                reason: error.to_string(),
                            });
                        continue;
                    }
                };
                self.seen.insert(canonical.clone(), self.documents.len());
                self.documents.push(DiscoveredDocument {
                    key,
                    path: canonical,
                    source_paths: vec![path],
                    root_index: self.root_index,
                    text,
                });
            }
        }
    }
}

fn project_relative_key(project_root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(project_root).ok()?;
    let mut key = String::new();
    for component in relative.components() {
        let component = component.as_os_str().to_str()?;
        if !key.is_empty() {
            key.push('/');
        }
        key.push_str(component);
    }
    Some(key)
}
