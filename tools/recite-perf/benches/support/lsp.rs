//! Prepared real workspaces; operation counters exclude discovery and file writes.
use lsp_types::{
    CompletionResponse, GotoDefinitionResponse, Location, Position, PublishDiagnosticsParams, Range,
};
use recite_lsp::bench_support::{
    LspBenchmarkConfig, LspBenchmarkDriver, LspDocumentProbe, LspPositionProbe,
};
use recite_writer_model::workload;

pub(super) struct Input {
    pub driver: LspBenchmarkDriver,
    pub position: LspPositionProbe,
    pub document: LspDocumentProbe,
    definition: Location,
    reference: Location,
    names: Vec<String>,
    _directory: tempfile::TempDir,
}

pub(super) fn prepare(per_document: usize) -> Input {
    let directory = tempfile::tempdir().unwrap_or_else(|error| panic!("workspace: {error}"));
    let documents =
        workload::project(1_000, per_document).unwrap_or_else(|error| panic!("project: {error}"));
    for document in &documents {
        std::fs::write(
            directory.path().join(document.key().as_str()),
            document.text(),
        )
        .unwrap_or_else(|error| panic!("source: {error}"));
    }
    let first = &documents[0];
    let source = first.text();
    let jump = source
        .rfind("-> beat_")
        .unwrap_or_else(|| panic!("local jump"));
    let target = format!("beat_{}", per_document.div_ceil(5) - 1);
    let header = source
        .find(&format!(":: {target}\n"))
        .unwrap_or_else(|| panic!("target"));
    let path = directory.path().join(first.key().as_str());
    let uri = format!("file://{}", path.display())
        .parse()
        .unwrap_or_else(|error| panic!("URI: {error}"));
    let document = LspDocumentProbe {
        uri,
        path,
        project_relative_path: first.key().as_str().into(),
    };
    let position = LspPositionProbe {
        uri: document.uri.clone(),
        path: document.path.clone(),
        project_relative_path: document.project_relative_path.clone(),
        position: Position::new(line(source, jump), 4),
    };
    let location = |offset| Location {
        uri: document.uri.clone(),
        range: Range::new(
            Position::new(line(source, offset), 3),
            Position::new(
                line(source, offset),
                u32::try_from(3 + target.len()).unwrap_or_else(|error| panic!("column: {error}")),
            ),
        ),
    };
    let definition = location(header);
    let reference = location(jump);
    let mut names = (0..1_000 / per_document)
        .flat_map(|document| {
            (0..per_document.div_ceil(5))
                .map(move |beat| format!("beat_{}", document * per_document + beat))
        })
        .collect::<Vec<_>>();
    names.sort();
    let mut driver =
        LspBenchmarkDriver::new(&LspBenchmarkConfig::new(vec![directory.path().to_owned()]));
    assert_eq!(driver.open_file(&document), 0);
    Input {
        driver,
        position,
        document,
        definition,
        reference,
        names,
        _directory: directory,
    }
}
fn line(source: &str, offset: usize) -> u32 {
    source[..offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        .try_into()
        .unwrap_or_else(|error| panic!("line: {error}"))
}
pub(super) fn validate_completion((input, result): (Input, Option<CompletionResponse>)) {
    let items = match result {
        Some(CompletionResponse::Array(items)) => items,
        Some(CompletionResponse::List(list)) => list.items,
        _ => panic!("completion result"),
    };
    let labels = items
        .iter()
        .map(|item| item.label.as_str())
        .collect::<Vec<_>>();
    assert_eq!(labels, input.names);
    for item in items {
        assert_eq!(item.kind, Some(lsp_types::CompletionItemKind::REFERENCE));
        assert!(item.detail.is_some());
    }
}
pub(super) fn validate_definition((input, result): (Input, Option<GotoDefinitionResponse>)) {
    let Some(GotoDefinitionResponse::Scalar(location)) = result else {
        panic!("one definition")
    };
    assert_eq!(location, input.definition);
}
pub(super) fn validate_references((input, result): (Input, Option<Vec<Location>>)) {
    let mut actual = result.unwrap_or_else(|| panic!("references"));
    let mut expected = vec![input.definition, input.reference];
    actual.sort_by_key(|location| location.range.start);
    expected.sort_by_key(|location| location.range.start);
    assert_eq!(actual, expected);
}
pub(super) fn validate_publication((input, result): (Input, Option<PublishDiagnosticsParams>)) {
    let result = result.unwrap_or_else(|| panic!("publication"));
    assert_eq!(result.uri, input.document.uri);
    assert!(result.diagnostics.is_empty());
}
