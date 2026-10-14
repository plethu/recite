use super::*;
use recite_writer_model::Document;

#[test]
fn completion_results_are_bound_to_document_revision_draft_and_cursor()
-> Result<(), Box<dyn std::error::Error>> {
    let source = ":: start default\n-> destination\n:: destination\n-> END\n";
    let document = Document::new(source)?;
    let position = SourcePosition::new(2, 5)?;
    let cached = SourceCompletionCache {
        document: document.source_snapshot(),
        revision: document.revision(),
        position,
        items: Arc::new(document.complete_draft(source, position)?),
    };
    assert!(cached.matches(
        &document.source_snapshot(),
        document.revision(),
        position,
        source
    ));
    let reopened = Document::new(source)?;
    assert!(!cached.matches(
        &reopened.source_snapshot(),
        reopened.revision(),
        position,
        source
    ));
    assert!(!cached.matches(
        &document.source_snapshot(),
        document.revision() + 1,
        position,
        source
    ));
    assert!(!cached.matches(
        &document.source_snapshot(),
        document.revision(),
        SourcePosition::new(2, 4)?,
        source
    ));
    assert!(!cached.matches(
        &document.source_snapshot(),
        document.revision(),
        position,
        ":: different\n"
    ));
    Ok(())
}
