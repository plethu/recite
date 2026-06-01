use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use lsp_types::{InitializeParams, Uri};
use serde_json::Value;

use crate::paths::uri_to_file_path;

#[derive(Clone, Debug)]
pub(crate) struct WorkspaceConfig {
    pub(super) roots: Vec<PathBuf>,
    pub(super) schema_path: Option<PathBuf>,
}

impl WorkspaceConfig {
    pub(crate) fn from_initialize_params(params: &InitializeParams) -> Self {
        let fallback_roots = fallback_roots(params)
            .into_iter()
            .filter_map(|root| resolve_config_path(&root, None))
            .filter_map(|root| fs::canonicalize(root).ok())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        let roots = match initialization_source_roots(params.initialization_options.as_ref()) {
            Some(source_roots) => source_roots
                .into_iter()
                .filter_map(|root| resolve_config_path(&root, fallback_roots.first()))
                .filter_map(|root| fs::canonicalize(root).ok())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>(),
            None => fallback_roots,
        };

        let schema_path = initialization_schema_path(params.initialization_options.as_ref())
            .and_then(|schema| resolve_config_path(&schema, roots.first()));

        Self { roots, schema_path }
    }

    #[allow(dead_code)]
    pub(crate) fn for_roots(roots: Vec<PathBuf>) -> Self {
        Self {
            roots: roots
                .into_iter()
                .filter_map(|root| fs::canonicalize(root).ok())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            schema_path: None,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn with_schema_path(mut self, schema_path: PathBuf) -> Self {
        self.schema_path = Some(schema_path);
        self
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

fn initialization_source_roots(value: Option<&Value>) -> Option<Vec<String>> {
    let object = value?.as_object()?;
    for key in ["sourceRoots", "source_roots"] {
        let Some(value) = object.get(key) else {
            continue;
        };
        let roots = string_array(value);
        if !roots.is_empty() {
            return Some(roots);
        }
    }

    None
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

fn string_array(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .collect()
}

fn resolve_config_path(value: &str, base: Option<&PathBuf>) -> Option<PathBuf> {
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
