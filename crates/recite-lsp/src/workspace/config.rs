use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use lsp_types::{InitializeParams, Uri};
use serde_json::Value;

use recite_config::{ProjectDiscoveryReport, discover_project};

use crate::paths::uri_to_file_path;

#[derive(Clone, Debug)]
pub(crate) struct WorkspaceConfig {
    pub(super) fallback_roots: Vec<PathBuf>,
    pub(super) schema_override_path: Option<PathBuf>,
    pub(super) discoveries: Vec<WorkspaceDiscovery>,
    pub(super) schema_paths: BTreeMap<String, PathBuf>,
}

#[derive(Clone, Debug)]
pub(super) struct WorkspaceDiscovery {
    pub(super) root: PathBuf,
    pub(super) state: WorkspaceDiscoveryState,
}

#[derive(Clone, Debug)]
pub(super) enum WorkspaceDiscoveryState {
    Manifest(Box<ProjectDiscoveryReport>),
    Manifestless,
    Failed {
        manifest_path: PathBuf,
        text: String,
        diagnostics: Vec<recite_core::Diagnostic>,
    },
}

impl WorkspaceConfig {
    pub(crate) fn from_initialize_params(params: &InitializeParams) -> Self {
        let fallback_roots = fallback_roots(params)
            .into_iter()
            .filter_map(|root| resolve_config_path(&root, None))
            .filter_map(|root| fs::canonicalize(root).ok())
            .collect::<Vec<_>>();
        let discoveries = discover_workspace_roots(&fallback_roots);
        let reports = discoveries
            .iter()
            .filter_map(|discovery| match &discovery.state {
                WorkspaceDiscoveryState::Manifest(report) => Some(report),
                WorkspaceDiscoveryState::Manifestless | WorkspaceDiscoveryState::Failed { .. } => {
                    None
                }
            });
        let discovery = reports
            .min_by_key(|report| report.manifest().manifest_path().to_owned())
            .cloned();
        let schema_base = discovery
            .as_ref()
            .map(|report| report.manifest().project_root().to_owned())
            .or_else(|| fallback_roots.first().cloned());
        let schema_override_path =
            initialization_schema_path(params.initialization_options.as_ref())
                .and_then(|schema| resolve_config_path(&schema, schema_base.as_deref()));
        let schema_paths = discoveries
            .iter()
            .filter_map(|discovery| match &discovery.state {
                WorkspaceDiscoveryState::Manifest(report) => {
                    schema_path_for_discovery(report).map(|path| {
                        (
                            crate::paths::stable_path_identity(report.manifest().project_root()),
                            path,
                        )
                    })
                }
                WorkspaceDiscoveryState::Manifestless | WorkspaceDiscoveryState::Failed { .. } => {
                    None
                }
            })
            .collect();

        Self {
            fallback_roots: fallback_roots.clone(),
            schema_override_path,
            discoveries,
            schema_paths,
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
            schema_override_path: None,
            discoveries: roots
                .clone()
                .iter()
                .cloned()
                .map(|root| WorkspaceDiscovery {
                    root,
                    state: WorkspaceDiscoveryState::Manifestless,
                })
                .collect(),
            schema_paths: BTreeMap::new(),
        }
    }

    #[cfg(any(test, feature = "bench-support"))]
    pub(crate) fn with_schema_path(mut self, schema_path: PathBuf) -> Self {
        self.schema_override_path = Some(schema_path);
        self
    }
}

pub(super) fn discover_workspace_roots(roots: &[PathBuf]) -> Vec<WorkspaceDiscovery> {
    roots
        .iter()
        .cloned()
        .map(|root| WorkspaceDiscovery {
            state: discover_workspace_root(&root, roots),
            root,
        })
        .collect()
}

fn discover_workspace_root(root: &Path, roots: &[PathBuf]) -> WorkspaceDiscoveryState {
    match discover_project(root) {
        Ok(report) => WorkspaceDiscoveryState::Manifest(Box::new(report)),
        Err(recite_config::ProjectDiscoveryError::NotFound { .. }) => {
            WorkspaceDiscoveryState::Manifestless
        }
        Err(error) => {
            let Some(manifest_path) = error.manifest_path().map(Path::to_owned) else {
                return WorkspaceDiscoveryState::Manifestless;
            };
            // A malformed manifest found above this explicit root belongs to
            // the deepest configured ancestor, not to this independent root.
            if !manifest_path.starts_with(root)
                && roots.iter().any(|candidate| {
                    manifest_path.starts_with(candidate)
                        && candidate.components().count() < root.components().count()
                })
            {
                return WorkspaceDiscoveryState::Manifestless;
            }
            let text = fs::read_to_string(&manifest_path).unwrap_or_default();
            WorkspaceDiscoveryState::Failed {
                manifest_path,
                text,
                diagnostics: error.diagnostics(),
            }
        }
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
