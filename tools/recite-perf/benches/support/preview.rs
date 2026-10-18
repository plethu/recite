use recite_writer_model::{Document, Preview, ProjectContext, workload};

pub(super) fn prepare(passages: usize) -> Document {
    let inputs = workload::project(passages, 500)
        .unwrap_or_else(|error| panic!("preview fixture keys: {error}"));
    let document = Document::in_project(
        inputs[0].key().clone(),
        inputs[0].text(),
        ProjectContext {
            documents: inputs.clone(),
            schema: None,
        },
    )
    .unwrap_or_else(|error| panic!("preview fixture: {error}"));
    assert!(document.diagnostics().is_empty());
    document
}
pub(super) fn validate((document, mut preview): (Document, Preview)) {
    assert_eq!(preview.revision(), document.revision());
    let page = preview
        .advance(None)
        .unwrap_or_else(|error| panic!("preview fixture output: {error}"));
    assert_eq!(
        page.text,
        "The courier waits beside the café. Beacon0 remains unanswered."
    );
    assert!(!page.ended);
    assert!(page.choices.is_empty());
    assert!(page.effects.is_empty());
}
