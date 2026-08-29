use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use recite_core::ProjectManifestSource;

use super::diagnostics::{DiscoveryDiagnostic, ProjectDiscoveryError};
use super::enumerate::{Coverage, DiscoveredDocument, DiscoveredRoot, enumerate_root};
use super::glob::{GlobPattern, normalize_relative_pattern, validate_relative_pattern};

pub const PROJECT_MANIFEST_FILE: &str = "recite.project.toml";
pub const PROJECT_MANIFEST_FORMAT_VERSION: u32 = 1;

/// Shared project manifest plus its canonical filesystem interpretation.
#[derive(Clone, Debug)]
pub struct ProjectManifest {
    project_root: PathBuf,
    manifest_path: PathBuf,
    source: ProjectManifestSource,
    roots: Vec<DiscoveredRoot>,
    excludes: Vec<String>,
    exclude_patterns: Vec<GlobPattern>,
}

impl ProjectManifest {
    #[must_use]
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    #[must_use]
    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    #[must_use]
    pub fn source(&self) -> &ProjectManifestSource {
        &self.source
    }

    #[must_use]
    pub fn roots(&self) -> &[DiscoveredRoot] {
        &self.roots
    }

    #[must_use]
    pub fn excludes(&self) -> &[String] {
        &self.excludes
    }

    /// Whether a canonical path is an eligible source path under this
    /// manifest's built-in and configured exclusion rules.
    #[must_use]
    pub fn allows_path(&self, path: &Path) -> bool {
        self.allows_event_path(path)
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".recite"))
    }

    /// Whether a watcher event is inside a configured root and not excluded.
    /// Unlike [`Self::allows_path`], this also accepts directories and missing
    /// paths so delete/rename events can wake a rebuild.
    #[must_use]
    pub fn allows_event_path(&self, path: &Path) -> bool {
        let Ok(relative) = path.strip_prefix(&self.project_root) else {
            return false;
        };
        let relative = relative.to_string_lossy().replace('\\', "/");
        !relative.is_empty()
            && !relative.split('/').any(|name| name.starts_with('.'))
            && !relative.split('/').any(is_builtin_excluded)
            && !self
                .exclude_patterns
                .iter()
                .any(|pattern| pattern.matches(&relative))
            && self
                .roots
                .iter()
                .any(|root| path.starts_with(root.path()) || root.path().starts_with(path))
    }
}

/// Deterministic project source index. Valid documents survive independent
/// coverage failures so an LSP can remain useful while publishing diagnostics.
#[derive(Clone, Debug)]
pub struct ProjectDiscoveryReport {
    manifest: ProjectManifest,
    documents: Vec<DiscoveredDocument>,
    diagnostics: Vec<DiscoveryDiagnostic>,
    coverage: Coverage,
}

impl ProjectDiscoveryReport {
    #[must_use]
    pub fn manifest(&self) -> &ProjectManifest {
        &self.manifest
    }

    #[must_use]
    pub fn documents(&self) -> &[DiscoveredDocument] {
        &self.documents
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[DiscoveryDiagnostic] {
        &self.diagnostics
    }

    #[must_use]
    pub const fn coverage(&self) -> Coverage {
        self.coverage
    }

    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.coverage.is_complete()
    }
}

/// Find and index the first `recite.project.toml` at or above an invocation
/// path. A malformed first manifest is an error; discovery never falls through
/// to a parent project after finding one.
pub fn discover_project(
    path: impl AsRef<Path>,
) -> Result<ProjectDiscoveryReport, ProjectDiscoveryError> {
    let (project_root, manifest_path) = find_manifest(path.as_ref())?;
    let source_text =
        std::fs::read(&manifest_path).map_err(|error| ProjectDiscoveryError::Read {
            path: manifest_path.clone(),
            message: error.to_string(),
        })?;
    let source_text =
        String::from_utf8(source_text).map_err(|_| ProjectDiscoveryError::NonUtf8 {
            path: manifest_path.clone(),
        })?;
    let loaded = recite_core::ProjectManifest::load_str_with_spans(
        manifest_path.to_string_lossy().into_owned(),
        &source_text,
    );
    let source = loaded
        .source
        .ok_or_else(|| ProjectDiscoveryError::Malformed {
            path: manifest_path.clone(),
            detail: loaded
                .diagnostics
                .first()
                .map(|diagnostic| diagnostic.message.clone())
                .unwrap_or_else(|| "manifest parser returned no source".to_owned()),
            diagnostics: loaded.diagnostics,
        })?;
    let manifest = source.manifest();
    match manifest.format_version {
        Some(PROJECT_MANIFEST_FORMAT_VERSION) => {}
        Some(found) => {
            return Err(ProjectDiscoveryError::UnsupportedFormatVersion {
                path: manifest_path,
                found,
                expected: PROJECT_MANIFEST_FORMAT_VERSION,
            });
        }
        None => {
            return Err(ProjectDiscoveryError::MissingFormatVersion {
                path: manifest_path,
                expected: PROJECT_MANIFEST_FORMAT_VERSION,
            });
        }
    }

    let mut roots = Vec::new();
    let configured_roots = &manifest.discovery.source_roots;
    for (index, relative) in configured_roots.iter().enumerate() {
        if let Err(reason) = validate_relative_pattern(relative) {
            return Err(ProjectDiscoveryError::InvalidSourceRoot {
                path: manifest_path.clone(),
                root: relative.clone(),
                reason,
            });
        }
        let candidate = project_root.join(relative);
        let canonical = match std::fs::canonicalize(&candidate) {
            Ok(path) => path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(ProjectDiscoveryError::InvalidSourceRoot {
                    path: manifest_path.clone(),
                    root: relative.clone(),
                    reason: format!("root does not exist: {error}"),
                });
            }
            Err(error) => {
                return Err(ProjectDiscoveryError::InvalidSourceRoot {
                    path: manifest_path.clone(),
                    root: relative.clone(),
                    reason: format!("root cannot be resolved: {error}"),
                });
            }
        };
        if !canonical.starts_with(&project_root) {
            return Err(ProjectDiscoveryError::InvalidSourceRoot {
                path: manifest_path.clone(),
                root: relative.clone(),
                reason: "root resolves outside the project".to_owned(),
            });
        }
        if !canonical.is_dir() {
            return Err(ProjectDiscoveryError::InvalidSourceRoot {
                path: manifest_path.clone(),
                root: relative.clone(),
                reason: "root is not a directory".to_owned(),
            });
        }
        if roots
            .iter()
            .any(|root: &DiscoveredRoot| root.path() == canonical)
        {
            return Err(ProjectDiscoveryError::DuplicateRoot {
                path: canonical,
                manifest: manifest_path.clone(),
            });
        }
        roots.push(DiscoveredRoot {
            index,
            relative: normalize_relative(relative),
            path: canonical,
        });
    }

    let mut excludes = Vec::new();
    let mut normalized_excludes = Vec::new();
    for pattern in &manifest.discovery.excludes {
        if let Err(reason) = validate_relative_pattern(pattern) {
            return Err(ProjectDiscoveryError::InvalidExclude {
                path: manifest_path.clone(),
                pattern: pattern.clone(),
                reason,
            });
        }
        let pattern = normalize_relative_pattern(pattern);
        excludes.push(GlobPattern::parse(&pattern).map_err(|reason| {
            ProjectDiscoveryError::InvalidExclude {
                path: manifest_path.clone(),
                pattern: pattern.clone(),
                reason,
            }
        })?);
        normalized_excludes.push(pattern);
    }

    let mut diagnostics = Vec::new();
    for (index, root) in roots.iter().enumerate() {
        if let Some(owner) = roots[..index].iter().find(|earlier| {
            root.path().starts_with(earlier.path()) || earlier.path().starts_with(root.path())
        }) {
            diagnostics.push(DiscoveryDiagnostic::OverlappingRoot {
                path: root.path().to_owned(),
                owner: owner.path().to_owned(),
            });
        }
    }

    let mut documents = Vec::new();
    let mut seen = BTreeSet::new();
    for root in &roots {
        enumerate_root(
            &project_root,
            root,
            &excludes,
            &mut documents,
            &mut diagnostics,
            &mut seen,
        );
    }
    documents.sort_by(|left, right| left.key().cmp(right.key()));
    diagnostics.sort_by_key(ToString::to_string);
    let coverage = if diagnostics
        .iter()
        .any(|diagnostic| !diagnostic.is_warning())
    {
        Coverage::Partial
    } else {
        Coverage::Complete
    };

    Ok(ProjectDiscoveryReport {
        manifest: ProjectManifest {
            project_root,
            manifest_path,
            source,
            roots,
            excludes: normalized_excludes,
            exclude_patterns: excludes,
        },
        documents,
        diagnostics,
        coverage,
    })
}

fn is_builtin_excluded(name: &str) -> bool {
    name.starts_with('.')
        || matches!(
            name,
            "target" | "build" | "dist" | "out" | "generated" | "vendor" | "node_modules"
        )
}

fn find_manifest(path: &Path) -> Result<(PathBuf, PathBuf), ProjectDiscoveryError> {
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
            Ok(metadata) if metadata.is_file() => {
                return Ok((directory, candidate));
            }
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

fn normalize_relative(value: &str) -> String {
    let components = value.split('/').filter(|component| *component != ".");
    let normalized = components.collect::<Vec<_>>().join("/");
    if normalized.is_empty() {
        ".".to_owned()
    } else {
        normalized
    }
}
