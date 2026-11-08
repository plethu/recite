//! Project-declared dialogue PO catalogue enumeration.
//!
//! Catalogues are selected only by the manifest's `[discovery].catalogs`
//! globs. Selection reuses the source walker's exclusion and containment
//! rules but returns paths rather than text: callers read a catalogue when an
//! operation needs it, so editors do not retain every translation in memory.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use recite_core::DocumentKey;

use super::diagnostics::DiscoveryDiagnostic;
use super::enumerate::is_excluded_relative;
use super::glob::GlobPattern;
use super::project_relative_key;

const CATALOG_SUFFIX: &str = ".po";

/// One project-owned dialogue PO catalogue selected by the manifest.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DiscoveredCatalog {
    key: DocumentKey,
    path: PathBuf,
}

impl DiscoveredCatalog {
    /// Project-relative slash key, disjoint from `.recite` document keys.
    #[must_use]
    pub fn key(&self) -> &DocumentKey {
        &self.key
    }

    /// Canonical filesystem path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Validates one manifest catalogue glob and returns its matcher.
pub(super) fn parse_catalog_pattern(pattern: &str) -> Result<GlobPattern, String> {
    let glob = GlobPattern::parse(pattern)?;
    if !pattern.ends_with(CATALOG_SUFFIX) {
        return Err(format!(
            "catalogue patterns must select `{CATALOG_SUFFIX}` files"
        ));
    }
    Ok(glob)
}

/// Enumerates every catalogue matched by `patterns`, sorted by key.
pub(super) fn enumerate_catalogs(
    project_root: &Path,
    patterns: &[(String, GlobPattern)],
    excludes: &[GlobPattern],
    diagnostics: &mut Vec<DiscoveryDiagnostic>,
) -> Vec<DiscoveredCatalog> {
    // A set keeps the result sorted and deduplicates overlapping patterns.
    let mut found = BTreeSet::new();
    for (source, pattern) in patterns {
        let base = literal_prefix(source);
        let mut walk = Walk {
            project_root,
            pattern,
            excludes,
            diagnostics,
            found: &mut found,
        };
        let directory = project_root.join(&base);
        match std::fs::symlink_metadata(&directory) {
            Ok(metadata) if metadata.is_dir() => walk.directory(&directory),
            Ok(metadata) if metadata.is_file() => walk.file(directory),
            // A pattern that selects nothing yet is not an error: locales are
            // commonly added after the manifest is written.
            Ok(_) | Err(_) => {}
        }
    }
    found.into_iter().collect()
}

/// Leading pattern components without wildcards, as a relative directory.
fn literal_prefix(pattern: &str) -> PathBuf {
    let mut prefix = PathBuf::new();
    for segment in pattern.split('/') {
        if segment.contains(['*', '?']) {
            break;
        }
        prefix.push(segment);
    }
    prefix
}

struct Walk<'a> {
    project_root: &'a Path,
    pattern: &'a GlobPattern,
    excludes: &'a [GlobPattern],
    diagnostics: &'a mut Vec<DiscoveryDiagnostic>,
    found: &'a mut BTreeSet<DiscoveredCatalog>,
}

impl Walk<'_> {
    fn directory(&mut self, directory: &Path) {
        let entries = match std::fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) => {
                self.read_error(directory, &error);
                return;
            }
        };
        let mut entries = entries
            .filter_map(|entry| match entry {
                Ok(entry) => Some(entry),
                Err(error) => {
                    self.read_error(directory, &error);
                    None
                }
            })
            .collect::<Vec<_>>();
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            let Some(relative) = project_relative_key(self.project_root, &path) else {
                self.diagnostics
                    .push(DiscoveryDiagnostic::NonUtf8Path { path });
                continue;
            };
            if is_excluded_relative(&relative, self.excludes) {
                continue;
            }
            match entry.file_type() {
                // Symlink directories are never traversed, matching sources.
                Ok(kind) if kind.is_dir() => self.directory(&path),
                Ok(kind) if kind.is_file() || kind.is_symlink() => self.file(path),
                Ok(_) => {}
                Err(error) => self.read_error(&path, &error),
            }
        }
    }

    fn file(&mut self, path: PathBuf) {
        let Some(relative) = project_relative_key(self.project_root, &path) else {
            self.diagnostics
                .push(DiscoveryDiagnostic::NonUtf8Path { path });
            return;
        };
        if !relative.ends_with(CATALOG_SUFFIX)
            || !self.pattern.matches(&relative)
            || is_excluded_relative(&relative, self.excludes)
        {
            return;
        }
        let canonical = match std::fs::canonicalize(&path) {
            Ok(target) if target.starts_with(self.project_root) && target.is_file() => target,
            Ok(target) if !target.starts_with(self.project_root) => {
                self.diagnostics
                    .push(DiscoveryDiagnostic::FileOutsideProject { path, target });
                return;
            }
            Ok(_) => return,
            Err(error) => {
                self.read_error(&path, &error);
                return;
            }
        };
        let Some(key) = project_relative_key(self.project_root, &canonical) else {
            self.diagnostics
                .push(DiscoveryDiagnostic::NonUtf8Path { path: canonical });
            return;
        };
        match DocumentKey::new(key) {
            Ok(key) => {
                self.found.insert(DiscoveredCatalog {
                    key,
                    path: canonical,
                });
            }
            Err(error) => self
                .diagnostics
                .push(DiscoveryDiagnostic::InvalidDocumentKey {
                    path: canonical,
                    reason: error.to_string(),
                }),
        }
    }

    fn read_error(&mut self, path: &Path, error: &std::io::Error) {
        self.diagnostics.push(DiscoveryDiagnostic::ReadDirectory {
            path: path.to_owned(),
            message: error.to_string(),
        });
    }
}
