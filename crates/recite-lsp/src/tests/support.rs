use std::fs;
use std::path::Path;

mod config;
mod harness;

pub(super) use harness::Harness;

use lsp_types::TextDocumentContentChangeEvent;
use recite_config::UiLocale;
use recite_ui::UiCatalog;
use serde_json::json;

use crate::workspace::{LspWorkspace, WorkspaceConfig};

pub(super) fn test_workspace(config: WorkspaceConfig) -> LspWorkspace {
    match UiCatalog::load(&UiLocale::default()) {
        Ok(catalog) => LspWorkspace::with_ui_catalog(config, catalog)
            .unwrap_or_else(|error| panic!("test authoring state is invalid: {error}")),
        Err(error) => panic!("test default UI catalog is invalid: {error}"),
    }
}

pub(super) fn full_change(text: &str) -> TextDocumentContentChangeEvent {
    TextDocumentContentChangeEvent {
        range: None,
        range_length: None,
        text: text.to_owned(),
    }
}

pub(super) fn uri(value: &str) -> lsp_types::Uri {
    match value.parse::<lsp_types::Uri>() {
        Ok(uri) => uri,
        Err(error) => panic!("invalid test URI {value}: {error}"),
    }
}

pub(super) fn file_uri(path: &Path) -> lsp_types::Uri {
    match crate::paths::file_path_to_uri(path) {
        Some(uri) => uri,
        None => panic!(
            "path cannot be represented as a file URI: {}",
            path.display()
        ),
    }
}

pub(super) fn harness_for_root(root: &Path) -> Harness {
    let root_uri = file_uri(root);
    Harness::start_with_result(json!({
        "capabilities": {
            "general": {
                "positionEncodings": ["utf-16"]
            }
        },
        "rootUri": root_uri.as_str()
    }))
    .0
}

pub(super) fn block_names(workspace: &LspWorkspace) -> Vec<String> {
    workspace
        .snapshot()
        .summaries()
        .iter()
        .flat_map(|summary| summary.blocks.iter().map(|block| block.name.clone()))
        .collect()
}

pub(super) fn write_file(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent()
        && let Err(error) = fs::create_dir_all(parent)
    {
        panic!("failed to create {}: {error}", parent.display());
    }
    if let Err(error) = fs::write(&path, contents) {
        panic!("failed to write {}: {error}", path.display());
    }
}
