//! Fixed wide/dense projects; opening, expected outputs and disposal are outside counters.
use recite_core::SourcePosition;
use recite_writer_model::{Document, ProjectContext, SourceCompletions, workload};

pub(super) struct CompletionWorkload {
    document: Document,
    draft: String,
    position: SourcePosition,
    expected_names: Vec<String>,
    revision: i64,
    accepted_source: String,
}

pub(super) struct Completed {
    workload: CompletionWorkload,
    result: SourceCompletions,
}

pub(super) fn changed(per_document: usize) -> CompletionWorkload {
    prepare(per_document, false)
}

pub(super) fn accepted(per_document: usize) -> CompletionWorkload {
    prepare(per_document, true)
}

fn prepare(per_document: usize, accepted: bool) -> CompletionWorkload {
    let documents = workload::project(2_000, per_document)
        .unwrap_or_else(|error| panic!("valid authoring project: {error}"));
    let first = &documents[0];
    let source = first.text();
    let end = source
        .rfind("-> END")
        .unwrap_or_else(|| panic!("final jump"));
    let draft = format!("{}-> beat_\n", &source[..end]);
    let position = must(SourcePosition::new(
        must(draft.lines().count().try_into()),
        9,
    ));
    let document = Document::in_project(
        first.key().clone(),
        if accepted { &draft } else { source },
        ProjectContext {
            documents: documents.clone(),
            schema: None,
        },
    )
    .unwrap_or_else(|error| panic!("valid authoring document: {error}"));
    let revision = document.revision();
    let accepted_source = document.source().to_owned();
    let mut expected_names = (0..per_document.div_ceil(5))
        .map(|id| format!("beat_{id}"))
        .collect::<Vec<_>>();
    expected_names.sort();
    CompletionWorkload {
        document,
        draft,
        position,
        expected_names,
        revision,
        accepted_source,
    }
}

pub(super) fn complete(workload: CompletionWorkload) -> Completed {
    let result = workload
        .document
        .complete_draft(&workload.draft, workload.position)
        .unwrap_or_else(|error| panic!("valid completion: {error}"));
    Completed { workload, result }
}

pub(super) fn validate(Completed { workload, result }: Completed) {
    assert_eq!(workload.document.revision(), workload.revision);
    assert_eq!(workload.document.source(), workload.accepted_source);
    let names = result
        .candidates()
        .iter()
        .map(|candidate| candidate.name())
        .collect::<Vec<_>>();
    assert_eq!(names, workload.expected_names);
    for (index, name) in names.iter().enumerate() {
        let (range, replacement) = result
            .replacement(&workload.draft, index)
            .unwrap_or_else(|error| panic!("valid replacement: {error}"));
        assert_eq!(&workload.draft[range], "beat_");
        assert_eq!(replacement, *name);
    }
}

fn must<T, E: std::fmt::Display>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("valid completion workload: {error}"))
}

pub(super) struct ReplacementWorkload {
    workload: CompletionWorkload,
    completions: SourceCompletions,
}
pub(super) struct Replaced {
    workload: ReplacementWorkload,
    range: std::ops::Range<usize>,
    replacement_bytes: usize,
}
pub(super) fn replacement(per_document: usize) -> ReplacementWorkload {
    let workload = changed(per_document);
    let completions = workload
        .document
        .complete_draft(&workload.draft, workload.position)
        .unwrap_or_else(|error| panic!("prepared replacement: {error}"));
    ReplacementWorkload {
        workload,
        completions,
    }
}
pub(super) fn replace(workload: ReplacementWorkload) -> Replaced {
    let (range, text) = workload
        .completions
        .replacement(workload.completions.source(), 0)
        .unwrap_or_else(|error| panic!("valid replacement: {error}"));
    let replacement_bytes = text.len();
    Replaced {
        workload,
        range,
        replacement_bytes,
    }
}
pub(super) fn validate_replacement(result: Replaced) {
    let Replaced {
        workload,
        range,
        replacement_bytes,
    } = result;
    assert_eq!(&workload.completions.source()[range], "beat_");
    assert_eq!(replacement_bytes, "beat_0".len());
    assert_eq!(
        workload.workload.document.revision(),
        workload.workload.revision
    );
    assert_eq!(
        workload.workload.document.source(),
        workload.workload.accepted_source
    );
}
