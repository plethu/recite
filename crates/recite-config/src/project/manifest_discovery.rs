use std::collections::BTreeMap;
use std::path::Path;

use super::super::diagnostics::{DiscoveryDiagnostic, ProjectDiscoveryError};
use super::super::enumerate::{Coverage, DiscoveredRoot, enumerate_root};
use super::super::glob::{GlobPattern, normalize_relative_pattern, validate_relative_pattern};
use super::search::find_manifest;
use super::{PROJECT_MANIFEST_FORMAT_VERSION, ProjectDiscoveryReport, ProjectManifest};

pub(super) fn discover_project(
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
    for (index, relative) in manifest.discovery.source_roots.iter().enumerate() {
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
    let mut seen = BTreeMap::new();
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

fn normalize_relative(value: &str) -> String {
    let components = value.split('/').filter(|component| *component != ".");
    let normalized = components.collect::<Vec<_>>().join("/");
    if normalized.is_empty() {
        ".".to_owned()
    } else {
        normalized
    }
}
