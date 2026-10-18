use super::*;
use std::cell::Cell;

struct Control {
    calls: Cell<usize>,
    stop_at: usize,
}
impl WorkControl for Control {
    fn checkpoint(&self) -> Result<(), recite_compiler::authoring::Interrupted> {
        let call = self.calls.get();
        self.calls.set(call + 1);
        if call >= self.stop_at {
            Err(recite_compiler::authoring::Interrupted)
        } else {
            Ok(())
        }
    }
}
fn control(stop_at: usize) -> Control {
    Control {
        calls: Cell::new(0),
        stop_at,
    }
}
fn context() -> Result<ProjectContext, EditError> {
    Ok(ProjectContext {
        documents: vec![
            SavedDocument::new(DocumentKey::new("a.recite")?, ":: start default\n-> END\n"),
            SavedDocument::new(
                DocumentKey::new("b.recite")?,
                ":: answer\n-> a.recite::start\n",
            ),
            SavedDocument::new(DocumentKey::new("removed.recite")?, ":: unused\n-> END\n"),
        ],
        schema: None,
    })
}
fn assert_cold_equivalent(sibling: &Document, cold: &Document) {
    assert_eq!(sibling.source(), cold.source());
    assert_eq!(sibling.revision(), cold.revision());
    assert_eq!(
        sibling.kernel.snapshot().documents(),
        cold.kernel.snapshot().documents()
    );
    assert_eq!(sibling.diagnostics(), cold.diagnostics());
    assert_eq!(sibling.project_sections(), cold.project_sections());
    assert!(!sibling.can_undo());
    assert_eq!(sibling.history.bytes(), 0);
}

#[test]
fn sibling_uses_authoritative_inputs_and_independent_history() -> Result<(), EditError> {
    let mut original = Document::in_project(
        DocumentKey::new("a.recite")?,
        ":: start default\n-> END\n",
        context()?,
    )?;
    original.replace_source(original.revision(), ":: renamed default\n-> END\n")?;
    let accepted = original.kernel.snapshot().clone();
    let mut updated = context()?;
    updated.documents.pop();
    updated.documents[0] = SavedDocument::new(original.key().clone(), original.source());
    let key = DocumentKey::new("b.recite")?;
    // A recovered unsaved source shadows this sibling's saved input.
    let source =
        ":: answer\n> reply@11111111111111111111\n  Recovered prose.\n-> a.recite::renamed\n";
    let mut sibling = original.open_sibling_with_control(
        key.clone(),
        source,
        updated.clone(),
        &CancellationToken::new(),
    )?;
    let cold = Document::in_project(key, source, updated)?;
    assert_cold_equivalent(&sibling, &cold);
    assert_eq!(original.kernel.snapshot(), &accepted);
    assert_eq!(
        sibling.project_sections().len(),
        2,
        "removed inputs must not survive"
    );
    assert!(sibling.diagnostics().is_empty());
    assert!(
        std::ptr::eq(
            original
                .kernel
                .snapshot()
                .document(original.key())
                .ok_or(EditError::Position)?
                .source_index(),
            sibling
                .kernel
                .snapshot()
                .document(original.key())
                .ok_or(EditError::Position)?
                .source_index(),
        ),
        "unchanged source analysis must be shared across independent documents"
    );
    sibling.replace_source(sibling.revision(), source.replace("Recovered", "Applied"))?;
    assert_eq!(original.source(), ":: renamed default\n-> END\n");
    assert_eq!(original.kernel.snapshot(), &accepted);
    original.undo()?;
    assert!(sibling.source().contains("Applied prose"));
    sibling.undo()?;
    assert_eq!(sibling.source(), source);
    assert_eq!(original.source(), ":: start default\n-> END\n");
    // Even the previous open file disappears when the supplied context excludes it.
    let key = DocumentKey::new("b.recite")?;
    let empty = ProjectContext::default();
    let sibling = original.open_sibling_with_control(
        key.clone(),
        source,
        empty.clone(),
        &CancellationToken::new(),
    )?;
    assert_cold_equivalent(&sibling, &Document::in_project(key, source, empty)?);
    assert_eq!(sibling.project_sections().len(), 1);
    assert!(!sibling.diagnostics().is_empty());
    Ok(())
}

#[test]
fn sibling_schema_change_and_same_key_match_cold_opening() -> Result<(), Box<dyn std::error::Error>>
{
    let mut original = Document::in_project(
        DocumentKey::new("a.recite")?,
        ":: start default\n-> END\n",
        context()?,
    )?;
    original.replace_source(original.revision(), ":: edited default\n-> END\n")?;
    let accepted = original.kernel.snapshot().clone();
    let report = recite_core::schema::load_schema_manifest_str(
        "schema.json",
        include_str!("../../../../../../fixtures/schema/valid/generated_manifest.json"),
    );
    assert!(report.diagnostics.is_empty());
    let mut schema = report.schema.ok_or("schema")?;
    schema.metadata.remove("portrait");
    let mut changed = context()?;
    changed.schema = Some(schema);
    let source = ":: answer\n> reply@11111111111111111111 portrait=calm\n  Hello.\n-> END\n";
    for (key, inputs) in [
        (DocumentKey::new("b.recite")?, changed),
        (original.key().clone(), context()?),
    ] {
        let sibling = original.open_sibling_with_control(
            key.clone(),
            source,
            inputs.clone(),
            &CancellationToken::new(),
        )?;
        assert_cold_equivalent(&sibling, &Document::in_project(key, source, inputs)?);
        assert_eq!(original.kernel.snapshot(), &accepted);
    }
    Ok(())
}

#[test]
fn sibling_interruption_never_changes_the_original() -> Result<(), EditError> {
    let original = Document::in_project(
        DocumentKey::new("a.recite")?,
        ":: start default\n-> END\n",
        context()?,
    )?;
    let accepted = original.kernel.snapshot().clone();
    let key = DocumentKey::new("b.recite")?;
    let source = ":: answer\n-> a.recite::start\n";
    let complete = control(usize::MAX);
    original.open_sibling_with_control(key.clone(), source, context()?, &complete)?;
    assert!(complete.calls.get() > 1);
    for stop_at in 0..complete.calls.get() {
        assert!(matches!(
            original.open_sibling_with_control(key.clone(), source, context()?, &control(stop_at)),
            Err(EditError::Authoring(AuthoringError::Interrupted(_)))
        ));
        assert_eq!(original.kernel.snapshot(), &accepted);
        assert_eq!(original.source(), ":: start default\n-> END\n");
        assert_eq!(original.revision(), 1);
    }
    Ok(())
}

#[test]
fn refused_analysis_does_not_publish_prepared_prose_projections() -> Result<(), EditError> {
    let mut document = Document::new(crate::FIXTURE)?;
    let script = document.script_snapshot()?;
    let passages = document.passage_snapshot()?;
    document.version = i64::MAX;
    assert!(matches!(
        document.replace_text(i64::MAX, &passages[0].id, "A proposed café edit."),
        Err(EditError::RevisionExhausted)
    ));
    assert_eq!(document.source(), crate::FIXTURE);
    assert!(!document.can_undo());
    assert!(std::sync::Arc::ptr_eq(
        &script,
        &document.script_snapshot()?
    ));
    assert!(std::sync::Arc::ptr_eq(
        &passages,
        &document.passage_snapshot()?
    ));
    Ok(())
}

#[test]
fn exhausted_revision_does_not_disable_read_only_draft_completion()
-> Result<(), Box<dyn std::error::Error>> {
    let mut document = super::Document::new(":: start default\n-> END\n")?;
    document.version = i64::MAX;
    let draft = ":: start default\n-> dest\n:: destination\n-> END\n";
    let result = document.complete_draft(draft, recite_core::SourcePosition::new(2, 8)?)?;
    assert!(
        result
            .candidates()
            .iter()
            .any(|c| c.name() == "destination")
    );
    let index = result
        .candidates()
        .iter()
        .position(|c| c.name() == "destination")
        .ok_or("destination")?;
    let (range, replacement) = result.replacement(draft, index)?;
    assert_eq!(&draft[range], "dest");
    assert_eq!(replacement, "destination");
    assert_eq!(document.version, i64::MAX);
    assert_eq!(document.source(), ":: start default\n-> END\n");
    Ok(())
}

#[test]
fn retained_analysis_does_not_publish_an_interrupted_acceptance() -> Result<(), EditError> {
    let next = format!("{}# proposed source\n", crate::FIXTURE);
    let mut reference = Document::new(crate::FIXTURE)?;
    let complete = control(usize::MAX);
    reference.accept_with_control(next.clone().into(), &complete)?;
    assert!(complete.calls.get() > 1);
    for stop_at in 0..complete.calls.get() {
        let mut document = Document::new(crate::FIXTURE)?;
        let retained = document.completion_snapshot();
        let script = document.script_snapshot()?;
        let passages = document.passage_snapshot()?;
        assert!(matches!(
            document.accept_with_control(next.clone().into(), &control(stop_at)),
            Err(EditError::Authoring(AuthoringError::Interrupted(_)))
        ));
        assert!(retained.is_current(&document));
        assert_eq!(document.source(), crate::FIXTURE);
        assert_eq!(document.revision(), 1);
        assert!(!document.can_undo());
        assert!(std::sync::Arc::ptr_eq(
            &script,
            &document.script_snapshot()?
        ));
        assert!(std::sync::Arc::ptr_eq(
            &passages,
            &document.passage_snapshot()?
        ));
    }
    Ok(())
}
