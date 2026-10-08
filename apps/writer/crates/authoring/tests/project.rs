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
fn unchanged_effective_project_preserves_preview_revision_and_undo()
-> Result<(), Box<dyn std::error::Error>> {
    let key = DocumentKey::new("scene.recite")?;
    let source = ":: start default\n> line@11111111111111111111\n  Hello.\n-> END\n";
    let context = ProjectContext {
        documents: vec![SavedDocument::new(key.clone(), source)],
        schema: None,
    };
    let document = Document::in_project(key.clone(), source, context.clone())?;
    let mut workbench = Workbench::from_document(document)?;
    workbench.set_draft("An applied edit.".into());
    workbench.apply()?;
    workbench.start_preview()?;
    let revision = workbench.document().revision();
    let page = workbench.preview_page().ok_or("preview page")?.text.clone();
    workbench.refresh_project(context)?;
    workbench.refresh_project(ProjectContext {
        documents: vec![SavedDocument::new(key, workbench.document().source())],
        schema: None,
    })?;
    assert_eq!(workbench.document().revision(), revision);
    assert!(!workbench.preview_stale());
    assert_eq!(workbench.preview_page().ok_or("preview page")?.text, page);
    workbench.undo()?;
    assert_eq!(workbench.document().source(), source);
    Ok(())
}

#[test]
fn changed_project_input_refreshes_cross_file_diagnostics_and_navigation()
-> Result<(), Box<dyn std::error::Error>> {
    let source = ":: start default\n-> other.recite::destination\n";
    let other = DocumentKey::new("other.recite")?;
    let context = ProjectContext {
        documents: vec![SavedDocument::new(
            other.clone(),
            ":: destination\n-> END\n",
        )],
        schema: None,
    };
    let mut document = Document::in_project(DocumentKey::new("scene.recite")?, source, context)?;
    assert!(document.diagnostics().is_empty());
    let revision = document.revision();
    document.refresh_project(ProjectContext {
        documents: vec![SavedDocument::new(other, ":: replacement\n-> END\n")],
        schema: None,
    })?;
    assert!(document.revision() > revision);
    assert!(
        document
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message.contains("destination"))
    );
    assert_eq!(
        document.project_sections(),
        vec![
            ("other.recite".into(), vec!["replacement".into()]),
            ("scene.recite".into(), vec!["start".into()])
        ]
    );
    Ok(())
}

#[test]
fn rejected_duplicate_context_preserves_the_accepted_document()
-> Result<(), Box<dyn std::error::Error>> {
    let key = DocumentKey::new("scene.recite")?;
    let source = ":: start default\n-> END\n";
    let saved = SavedDocument::new(key.clone(), source);
    let mut document = Document::in_project(
        key,
        source,
        ProjectContext {
            documents: vec![saved.clone()],
            schema: None,
        },
    )?;
    let revision = document.revision();
    let diagnostics = document.diagnostics();
    assert!(
        document
            .refresh_project(ProjectContext {
                documents: vec![saved.clone(), saved],
                schema: None
            })
            .is_err()
    );
    assert_eq!(document.revision(), revision);
    assert_eq!(document.diagnostics(), diagnostics);
    assert_eq!(document.source(), source);
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
