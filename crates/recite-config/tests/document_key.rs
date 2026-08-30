use std::fs;

use recite_config::{DiscoveryDiagnostic, discover_project};
use tempfile::TempDir;

fn write(root: &std::path::Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(
        path.parent()
            .unwrap_or_else(|| panic!("test path has a parent: {path:?}")),
    )
    .unwrap_or_else(|error| panic!("test directory: {error}"));
    fs::write(path, content).unwrap_or_else(|error| panic!("test file: {error}"));
}

#[test]
fn invalid_document_key_diagnostic_keeps_path_reason_and_is_recordable() {
    let diagnostic = DiscoveryDiagnostic::InvalidDocumentKey {
        path: std::path::PathBuf::from("dir\\start.recite"),
        reason: "document key must use slash separators".to_owned(),
    };
    let core = diagnostic.as_core_diagnostic();

    assert_eq!(core.code.as_str(), "RECITE_CONFIG117");
    assert_eq!(core.span.file, "dir\\start.recite");
    assert_eq!(
        core.presentation
            .as_ref()
            .and_then(|presentation| presentation.arguments().get("detail"))
            .and_then(|value| match value {
                recite_core::DiagnosticArgumentValue::String(value) => Some(value.as_str()),
                _ => None,
            }),
        Some("project source has an invalid document key: document key must use slash separators")
    );
    assert!(core.record().is_ok());
}

#[cfg(unix)]
#[test]
fn literal_backslash_filename_cannot_collide_with_valid_document_key() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write(temp.path(), "recite.project.toml", "format_version = 1\n");
    write(temp.path(), "dir/start.recite", ":: valid\n");
    write(temp.path(), r"dir\start.recite", ":: invalid key\n");

    let report =
        discover_project(temp.path()).unwrap_or_else(|error| panic!("project discovery: {error}"));
    assert_eq!(
        report
            .documents()
            .iter()
            .map(|document| document.key().as_str())
            .collect::<Vec<_>>(),
        ["dir/start.recite"]
    );
    let diagnostic = report
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code().as_str() == "RECITE_CONFIG117")
        .unwrap_or_else(|| panic!("invalid document key diagnostic"));
    assert!(matches!(
        diagnostic,
        DiscoveryDiagnostic::InvalidDocumentKey { path, .. }
            if path.ends_with(r"dir\start.recite")
    ));
    assert!(diagnostic.as_core_diagnostic().record().is_ok());
}
