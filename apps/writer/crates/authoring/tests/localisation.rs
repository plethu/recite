use recite_compiler::authoring::SavedDocument;
use recite_core::DocumentKey;
use recite_writer_model::{Document, ProjectContext};

#[test]
fn extraction_covers_project_with_current_overlay_and_context()
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
    let report = document.extract_catalogue();
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let catalogue = report.catalog.ok_or("catalogue")?;
    assert_eq!(catalogue.entries.len(), 2);
    assert!(
        catalogue
            .entries
            .iter()
            .any(|e| e.source_text == "Unsaved question." && e.context == "11111111111111111111")
    );
    assert!(
        catalogue
            .entries
            .iter()
            .any(|e| e.source_text == "From another file.")
    );
    assert!(catalogue.to_pot_string().contains("#. file: a.recite"));
    assert_eq!(document.source(), source);
    Ok(())
}

#[test]
fn extraction_reports_source_errors_instead_of_partial_catalogues()
-> Result<(), Box<dyn std::error::Error>> {
    let document =
        Document::new(":: start default\n> line@11111111111111111111\n  Hello.\n-> missing\n")?;
    let report = document.extract_catalogue();
    assert!(report.catalog.is_none());
    assert!(!report.diagnostics.is_empty());
    Ok(())
}

#[test]
fn example_schema_text_is_included() -> Result<(), Box<dyn std::error::Error>> {
    let schema = recite_core::schema::load_schema_manifest_str(
        "schema.json",
        r#"{
        "schema_version": 1, "speakers": { "mara": { "display_name": "Mara" } }
    }"#,
    )
    .schema
    .ok_or("schema")?;
    let document = Document::in_project(
        DocumentKey::new("scene.recite")?,
        ":: start default\n> line@11111111111111111111\n  Hello.\n-> END\n",
        ProjectContext {
            documents: vec![],
            schema: Some(schema),
        },
    )?;
    let report = document.extract_catalogue();
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    assert!(
        report
            .catalog
            .ok_or("catalogue")?
            .entries
            .iter()
            .any(|e| e.context.starts_with("dialogue_speaker:"))
    );
    Ok(())
}
