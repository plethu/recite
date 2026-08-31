use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use lsp_types::{InitializeParams, Uri};
use serde_json::Value;

use recite_config::{ProjectDiscoveryReport, discover_project};

use crate::paths::uri_to_file_path;

#[derive(Clone, Debug)]
pub(crate) struct WorkspaceConfig {
    pub(super) fallback_roots: Vec<PathBuf>,
    pub(super) roots: Vec<PathBuf>,
    pub(super) schema_path: Option<PathBuf>,
    pub(super) schema_override_path: Option<PathBuf>,
    pub(super) discovery: Option<ProjectDiscoveryReport>,
    pub(super) discovery_diagnostics: Vec<recite_core::Diagnostic>,
    pub(super) discovery_failed_roots: BTreeSet<PathBuf>,
    pub(super) discovery_start: Option<PathBuf>,
    pub(super) discovery_manifest_path: Option<PathBuf>,
}

impl WorkspaceConfig {
    pub(crate) fn from_initialize_params(params: &InitializeParams) -> Self {
        let fallback_roots = fallback_roots(params)
            .into_iter()
            .filter_map(|root| resolve_config_path(&root, None))
            .filter_map(|root| fs::canonicalize(root).ok())
            .collect::<Vec<_>>();
        let mut reports = BTreeMap::new();
        let mut discovery_diagnostics = Vec::new();
        let mut discovery_failed_roots = BTreeSet::new();
        let mut discovery_starts = BTreeMap::new();
        let mut failed_manifest_paths = BTreeSet::new();
        for root in &fallback_roots {
            let start = discovery_start(root);
            discovery_starts.insert(root.clone(), start.clone());
            match discover_project(root) {
                Ok(report) => {
                    reports
                        .entry(report.manifest().manifest_path().to_owned())
                        .or_insert((start, report));
                }
                Err(recite_config::ProjectDiscoveryError::NotFound { .. }) => {}
                Err(error) => {
                    discovery_failed_roots.insert(root.clone());
                    let diagnostics = error.diagnostics();
                    discovery_diagnostics.extend(diagnostics.iter().cloned());
                    if let Some(path) = error.manifest_path() {
                        failed_manifest_paths.insert(path.to_owned());
                    }
                }
            }
        }
        let (discovery_start, discovery) = reports.into_values().next().map_or_else(
            || {
                (
                    fallback_roots
                        .first()
                        .and_then(|root| discovery_starts.get(root))
                        .cloned(),
                    None,
                )
            },
            |(start, report)| (Some(start), Some(report)),
        );
        let discovery_manifest_path = failed_manifest_paths
            .iter()
            .next()
            .cloned()
            .or_else(|| {
                discovery
                    .as_ref()
                    .map(|report| report.manifest().manifest_path().to_owned())
            })
            .or_else(|| {
                discovery_start
                    .as_deref()
                    .map(|start| start.join(recite_config::PROJECT_MANIFEST_FILE))
            });
        discovery_diagnostics.sort_by(|left, right| {
            left.code
                .as_str()
                .cmp(right.code.as_str())
                .then_with(|| left.message.cmp(&right.message))
        });
        let roots = discovery
            .as_ref()
            .map(|report| {
                report
                    .manifest()
                    .roots()
                    .iter()
                    .map(|root| root.path().to_owned())
                    .collect()
            })
            .unwrap_or_else(|| fallback_roots.clone());
        let schema_base = discovery
            .as_ref()
            .map(|report| report.manifest().project_root().to_owned())
            .or_else(|| fallback_roots.first().cloned());
        let schema_override_path =
            initialization_schema_path(params.initialization_options.as_ref())
                .and_then(|schema| resolve_config_path(&schema, schema_base.as_deref()));
        let schema_path = schema_override_path
            .clone()
            .or_else(|| discovery.as_ref().and_then(schema_path_for_discovery));

        Self {
            fallback_roots: fallback_roots.clone(),
            roots,
            schema_path,
            schema_override_path,
            discovery,
            discovery_diagnostics,
            discovery_failed_roots,
            discovery_start,
            discovery_manifest_path,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn for_roots(roots: Vec<PathBuf>) -> Self {
        let roots = roots
            .into_iter()
            .filter_map(|root| fs::canonicalize(root).ok())
            .collect::<Vec<_>>();
        Self {
            fallback_roots: roots.clone(),
            discovery_start: roots.first().cloned(),
            roots,
            schema_path: None,
            schema_override_path: None,
            discovery: None,
            discovery_diagnostics: Vec::new(),
            discovery_failed_roots: BTreeSet::new(),
            discovery_manifest_path: None,
        }
    }

    #[cfg(any(test, feature = "bench-support"))]
    pub(crate) fn with_schema_path(mut self, schema_path: PathBuf) -> Self {
        self.schema_path = Some(schema_path.clone());
        self.schema_override_path = Some(schema_path);
        self
    }
}

fn discovery_start(root: &Path) -> PathBuf {
    if root.is_dir() {
        root.to_owned()
    } else {
        root.parent().map_or_else(|| root.to_owned(), PathBuf::from)
    }
}

#[allow(deprecated)]
fn fallback_roots(params: &InitializeParams) -> Vec<String> {
    if let Some(workspace_folders) = &params.workspace_folders {
        let roots = workspace_folders
            .iter()
            .map(|folder| folder.uri.as_str().to_owned())
            .collect::<Vec<_>>();
        if !roots.is_empty() {
            return roots;
        }
    }

    if let Some(root_uri) = &params.root_uri {
        return vec![root_uri.as_str().to_owned()];
    }

    params.root_path.clone().into_iter().collect()
}

fn initialization_schema_path(value: Option<&Value>) -> Option<String> {
    let object = value?.as_object()?;
    for key in ["schemaManifest", "schema_manifest", "schema"] {
        let Some(value) = object.get(key).and_then(Value::as_str) else {
            continue;
        };
        if !value.trim().is_empty() {
            return Some(value.to_owned());
        }
    }

    None
}

pub(super) fn schema_path_for_discovery(report: &ProjectDiscoveryReport) -> Option<PathBuf> {
    let schema = report
        .manifest()
        .source()
        .manifest()
        .project
        .schema
        .as_deref()?;
    resolve_config_path(schema, Some(report.manifest().project_root()))
}

fn resolve_config_path(value: &str, base: Option<&Path>) -> Option<PathBuf> {
    if let Ok(uri) = value.parse::<Uri>()
        && let Some(path) = uri_to_file_path(&uri)
    {
        return Some(path);
    }

    let path = PathBuf::from(value);
    if path.is_absolute() {
        Some(path)
    } else {
        base.map(|base| base.join(path))
    }
}
