//! Query-worker forks must share schema payloads while preserving query results.
use lsp_types::{CompletionResponse, Position};
use recite_lsp::bench_support::{
    LspBenchmarkConfig, LspBenchmarkDriver, LspDocumentProbe, LspPositionProbe,
};

pub(super) struct Input {
    pub driver: LspBenchmarkDriver,
    probe: LspPositionProbe,
    expected: Vec<String>,
    actions: lsp_types::CodeActionParams,
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
    let actions = lsp_types::CodeActionParams {
        text_document: lsp_types::TextDocumentIdentifier {
            uri: probe.uri.clone(),
        },
        range: lsp_types::Range::new(probe.position, probe.position),
        context: lsp_types::CodeActionContext {
            diagnostics: Vec::new(),
            only: None,
            trigger_kind: None,
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };
    Input {
        actions,
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

pub(super) fn open_schema(count: usize) -> Input {
    let mut input = prepare(count);
    let path = input._directory.path().join("schema.json");
    let schema = LspDocumentProbe {
        uri: format!("file://{}", path.display())
            .parse()
            .unwrap_or_else(|error| panic!("schema URI: {error}")),
        path,
        project_relative_path: "schema.json".into(),
    };
    let _ = input.driver.open_file(&schema);
    input
}

fn document(input: &Input) -> LspDocumentProbe {
    LspDocumentProbe {
        uri: input.probe.uri.clone(),
        path: input.probe.path.clone(),
        project_relative_path: input.probe.project_relative_path.clone(),
    }
}

pub(super) fn change(mut input: Input) -> (Input, usize) {
    let document = document(&input);
    let count = input.driver.change_file(&document);
    (input, count)
}

pub(super) fn validate_change((input, count): (Input, usize)) {
    let published = input
        .driver
        .diagnostics_publication(&document(&input))
        .unwrap_or_else(|| panic!("changed source diagnostics"));
    assert_eq!(published.version, Some(2));
    assert_eq!(published.diagnostics.len(), count);
    assert!(count > 0, "incomplete condition remains diagnostic");
    let fork = input.driver.fork();
    validate((input, fork));
}

pub(super) fn actions(input: Input) -> (Input, Option<lsp_types::CodeActionResponse>) {
    let result = input.driver.code_action(&input.actions);
    (input, result)
}

pub(super) fn validate_actions((input, actions): (Input, Option<lsp_types::CodeActionResponse>)) {
    let actions = actions.unwrap_or_else(|| panic!("schema actions"));
    let expected = std::iter::once(
        "Schema capability (schema ): unavailable schema action (no producer )".to_owned(),
    )
    .chain(input.expected.iter().map(|name| {
        format!("Schema capability (condition {name}): unavailable schema action (no producer )")
    }))
    .collect::<Vec<_>>();
    assert_eq!(actions.len(), expected.len());
    for (action, title) in actions.iter().zip(expected) {
        let lsp_types::CodeActionOrCommand::CodeAction(action) = action else {
            panic!("code action")
        };
        assert_eq!(action.title, title);
        assert_eq!(action.kind, Some(lsp_types::CodeActionKind::REFACTOR));
        assert_eq!(
            action.disabled.as_ref().map(|value| value.reason.as_str()),
            Some("Schema capability unavailable: source owner is unknown.")
        );
    }
}
