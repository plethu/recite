use recite_compiler::authoring::SavedDocument;
use recite_core::DocumentKey;
use recite_writer_model::{Document, ProjectContext, View, Workbench};

#[test]
fn cross_file_preview_uses_the_current_document_and_its_unsaved_overlay()
-> Result<(), Box<dyn std::error::Error>> {
    let source = ":: start default\n> question@11111111111111111111\n  Unsaved question.\n-> a.recite::answer\n";
    let context = ProjectContext {
        documents: vec![
            SavedDocument::new(
                DocumentKey::new("a.recite")?,
                ":: answer\n> response@22222222222222222222\n  From another file.\n-> END\n",
            ),
            SavedDocument::new(
                DocumentKey::new("z.recite")?,
                source.replace("Unsaved", "Saved"),
            ),
        ],
        schema: None,
    };
    let document = Document::in_project(DocumentKey::new("z.recite")?, source, context)?;
    assert!(
        document.diagnostics().is_empty(),
        "{:?}",
        document.diagnostics()
    );
    let mut workbench = Workbench::from_document(document)?;
    workbench.start_preview()?;
    assert_eq!(
        workbench.preview_page().ok_or("page")?.text,
        "Unsaved question."
    );
    workbench.advance_preview(None)?;
    assert_eq!(
        workbench.preview_page().ok_or("page")?.text,
        "From another file."
    );
    Ok(())
}

#[test]
fn recovery_keeps_rejected_prose_separate_from_applied_source()
-> Result<(), Box<dyn std::error::Error>> {
    let mut workbench = Workbench::new(recite_writer_model::FIXTURE)?;
    let original_view = workbench.view().clone();
    workbench.set_draft("  text with unsafe leading space".into());
    assert!(workbench.apply().is_err());
    let recovery = workbench.recovery();
    let mut restored = Workbench::new(recovery.source())?;
    recovery.restore(&mut restored)?;
    assert_eq!(restored.view(), &original_view);
    assert_eq!(restored.draft(), "  text with unsafe leading space");
    assert!(restored.has_draft());
    assert_eq!(restored.document().source(), recite_writer_model::FIXTURE);
    restored.discard();
    restored.select(View::Source)?;
    restored.set_draft("an incomplete source buffer".into());
    let recovery = restored.recovery();
    let mut again = Workbench::new(recovery.source())?;
    recovery.restore(&mut again)?;
    assert_eq!(again.view(), &View::Source);
    assert_eq!(again.draft(), "an incomplete source buffer");
    Ok(())
}

#[test]
fn schema_refresh_retains_drafts_and_undo_but_invalidates_preview()
-> Result<(), Box<dyn std::error::Error>> {
    let source = ":: start default\n> line@11111111111111111111 portrait=calm\n  Hello.\n-> END\n";
    let mut workbench = Workbench::open("scene.recite", source)?;
    workbench.set_draft("An applied edit.".into());
    workbench.apply()?;
    workbench.start_preview()?;
    workbench.set_draft("A field draft.".into());
    let report = recite_core::schema::load_schema_manifest_str(
        "schema.json",
        include_str!("../../../../../fixtures/schema/valid/generated_manifest.json"),
    );
    assert!(report.diagnostics.is_empty());
    let mut schema = report.schema.ok_or("schema")?;
    schema.metadata.remove("portrait");
    workbench.refresh_project(ProjectContext {
        documents: vec![],
        schema: Some(schema),
    })?;
    assert_eq!(workbench.draft(), "A field draft.");
    assert!(workbench.preview_stale());
    assert!(
        workbench
            .document()
            .diagnostics()
            .iter()
            .any(|d| d.message.contains("portrait"))
    );
    workbench.discard();
    workbench.undo()?;
    assert_eq!(workbench.document().source(), source);
    assert!(workbench.start_preview().is_err());
    Ok(())
}
