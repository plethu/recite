use recite_compiler::authoring::{AuthoringError, CancellationToken, SavedDocument, WorkControl};
use recite_core::{DocumentKey, SourcePosition};
use recite_writer_model::{CompletionSnapshot, Document, EditError, ProjectContext};
use std::cell::Cell;

const ACCEPTED: &str = ":: start default\n-> END\n";
const DRAFT: &str = ":: start default\n-> dest\n:: destination\n-> END\n";

#[test]
fn captured_completion_is_thread_safe_and_survives_mutation_and_drop()
-> Result<(), Box<dyn std::error::Error>> {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<CompletionSnapshot>();
    let mut document = Document::new(ACCEPTED)?;
    let snapshot = document.completion_snapshot();
    assert!(snapshot.is_current(&document));
    assert!(snapshot.same_analysis(&document.completion_snapshot()));
    let cursor = SourcePosition::new(2, 8)?;
    let expected = document.complete_draft(DRAFT, cursor)?;
    let original = snapshot.complete(
        ACCEPTED,
        SourcePosition::new(2, 4)?,
        &CancellationToken::new(),
    )?;
    assert_eq!(original.source(), ACCEPTED);
    document.replace_source(document.revision(), ":: changed default\n-> END\n".into())?;
    assert!(!snapshot.is_current(&document));
    assert!(!snapshot.same_analysis(&document.completion_snapshot()));
    drop(document);
    let result =
        std::thread::spawn(move || snapshot.complete(DRAFT, cursor, &CancellationToken::new()))
            .join()
            .unwrap()?;
    assert_eq!(result, expected);
    assert!(
        result
            .candidates()
            .iter()
            .any(|candidate| candidate.name() == "destination")
    );
    Ok(())
}

#[test]
fn context_identity_invalidates_same_byte_queries_without_a_revision_change()
-> Result<(), Box<dyn std::error::Error>> {
    let mut document = Document::new(ACCEPTED)?;
    let snapshot = document.completion_snapshot();
    let version = document.revision();
    document.refresh_project(ProjectContext::default())?;
    assert_eq!(document.revision(), version);
    assert_eq!(document.source(), ACCEPTED);
    assert!(!snapshot.is_current(&document));
    assert!(!snapshot.same_analysis(&document.completion_snapshot()));
    let snapshot = document.completion_snapshot();
    document.refresh_project(ProjectContext {
        documents: vec![SavedDocument::new(
            DocumentKey::new("sibling.recite")?,
            ":: sibling\n-> END\n",
        )],
        schema: None,
    })?;
    assert!(!snapshot.is_current(&document));
    assert!(!snapshot.same_analysis(&document.completion_snapshot()));
    Ok(())
}

struct Control {
    calls: Cell<usize>,
    stop: usize,
}
impl WorkControl for Control {
    fn checkpoint(&self) -> Result<(), recite_compiler::authoring::Interrupted> {
        let call = self.calls.get();
        self.calls.set(call + 1);
        if call >= self.stop {
            Err(recite_compiler::authoring::Interrupted)
        } else {
            Ok(())
        }
    }
}
#[test]
fn captured_queries_can_be_interrupted_at_every_checkpoint()
-> Result<(), Box<dyn std::error::Error>> {
    let document = Document::new(ACCEPTED)?;
    let snapshot = document.completion_snapshot();
    for draft in [ACCEPTED, DRAFT] {
        let control = Control {
            calls: Cell::new(0),
            stop: usize::MAX,
        };
        let cursor = SourcePosition::new(2, 4)?;
        let completed = snapshot.complete(draft, cursor, &control)?;
        assert_eq!(completed.source(), draft);
        assert!(control.calls.get() >= 3);
        for stop in 0..control.calls.get() {
            let control = Control {
                calls: Cell::new(0),
                stop,
            };
            assert!(matches!(
                snapshot.complete(draft, cursor, &control),
                Err(EditError::Authoring(AuthoringError::Interrupted(_)))
            ));
            assert!(snapshot.is_current(&document));
            assert_eq!(document.source(), ACCEPTED);
            assert_eq!(document.revision(), 1);
        }
    }
    Ok(())
}
