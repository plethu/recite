//! Query-worker forks must share schema payloads while preserving query results.
use lsp_types::{CompletionResponse, Position};
use recite_lsp::bench_support::{
    LspBenchmarkConfig, LspBenchmarkDriver, LspDocumentProbe, LspPositionProbe,
};

pub(super) struct Input {
    pub driver: LspBenchmarkDriver,
    probe: LspPositionProbe,
    expected: Vec<String>,
    _directory: tempfile::TempDir,
}
pub(super) fn prepare(count: usize) -> Input {
    let directory = tempfile::tempdir().unwrap_or_else(|error| panic!("workspace: {error}"));
    let schema_path = directory.path().join("schema.json");
    std::fs::write(&schema_path, super::schema::json(count).text)
        .unwrap_or_else(|error| panic!("schema: {error}"));
    let path = directory.path().join("source.recite");
    std::fs::write(
        &path,
        ":: start default\n:if gate_()\n  > line@11111111111111111111\n    Hello.\n-> END\n",
    )
    .unwrap_or_else(|error| panic!("source: {error}"));
    let uri = format!("file://{}", path.display())
        .parse()
        .unwrap_or_else(|error| panic!("URI: {error}"));
    let document = LspDocumentProbe {
        uri,
        path,
        project_relative_path: "source.recite".into(),
    };
    let probe = LspPositionProbe {
        uri: document.uri.clone(),
        path: document.path.clone(),
        project_relative_path: document.project_relative_path.clone(),
        position: Position::new(1, 9),
    };
    let mut driver = LspBenchmarkDriver::new(
        &LspBenchmarkConfig::new(vec![directory.path().to_owned()]).with_schema_path(schema_path),
    );
    // The deliberately incomplete condition provides a schema-completion site.
    let _ = driver.open_file(&document);
    Input {
        driver,
        probe,
        expected: (0..count).map(|i| format!("gate_{i:06}")).collect(),
        _directory: directory,
    }
}
pub(super) fn validate((input, fork): (Input, LspBenchmarkDriver)) {
    let response = fork.completion(&input.probe);
    assert_eq!(response, input.driver.completion(&input.probe));
    let items = match response {
        Some(CompletionResponse::Array(items)) => items,
        Some(CompletionResponse::List(list)) => list.items,
        _ => panic!("schema completion"),
    };
    assert_eq!(
        items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        input.expected
    );
}
