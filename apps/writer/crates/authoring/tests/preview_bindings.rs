use recite_compiler::authoring::{CancellationToken, SavedDocument};
use recite_core::{DocumentKey, ast::InterpolationType};
use recite_writer_model::{Document, ProjectContext};
use std::sync::Arc;

fn source(value: &str) -> String {
    format!(
        ":: start default\n> line@11111111111111111111 bind=(name:string=${value})\n  Hello {{name}}.\n-> END\n"
    )
}
fn context(value: &str) -> Result<ProjectContext, Box<dyn std::error::Error>> {
    Ok(ProjectContext {
        documents: vec![SavedDocument::new(
            DocumentKey::new("other.recite")?,
            source(value),
        )],
        schema: None,
    })
}
#[test]
fn bindings_share_accepted_revisions_and_survive_edit_history()
-> Result<(), Box<dyn std::error::Error>> {
    let mut document = Document::new(source("person"))?;
    let original = document.preview_bindings();
    assert_eq!(original.len(), 1);
    assert_eq!(original[0].value, "person");
    assert_eq!(original[0].value_type, InterpolationType::String);
    assert!(Arc::ptr_eq(&original, &document.preview_bindings()));
    document.replace_text(
        document.revision(),
        "11111111111111111111",
        "Goodbye {name}.",
    )?;
    assert_eq!(document.preview_bindings(), original);
    document.replace_source(document.revision(), source("replacement"))?;
    let changed = document.preview_bindings();
    assert_eq!(changed[0].value, "replacement");
    assert!(!Arc::ptr_eq(&original, &changed));
    assert!(document.undo()?);
    assert_eq!(document.preview_bindings(), original);
    assert!(document.redo()?);
    assert_eq!(document.preview_bindings(), changed);
    assert_eq!(original[0].value, "person");
    Ok(())
}
#[test]
fn project_refresh_invalidates_only_after_accepting_changed_inputs()
-> Result<(), Box<dyn std::error::Error>> {
    let mut document = Document::in_project(
        DocumentKey::new("scene.recite")?,
        source("local"),
        context("other")?,
    )?;
    let first = document.preview_bindings();
    assert_eq!(
        first.iter().map(|b| b.value.as_str()).collect::<Vec<_>>(),
        ["local", "other"]
    );
    document.refresh_project(context("other")?)?;
    assert!(Arc::ptr_eq(&first, &document.preview_bindings()));
    let control = CancellationToken::new();
    control.interrupt();
    assert!(
        document
            .refresh_project_with_control(context("rejected")?, &control)
            .is_err()
    );
    assert!(Arc::ptr_eq(&first, &document.preview_bindings()));
    document.refresh_project(context("accepted")?)?;
    assert_eq!(
        document
            .preview_bindings()
            .iter()
            .map(|b| b.value.as_str())
            .collect::<Vec<_>>(),
        ["accepted", "local"]
    );
    document.refresh_project(ProjectContext::default())?;
    let remaining = document.preview_bindings();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].value, "local");
    assert_eq!(first[1].value, "other");
    Ok(())
}
#[test]
fn duplicate_binding_values_preserve_the_first_source_order_definition()
-> Result<(), Box<dyn std::error::Error>> {
    let first = ":: start\n> a bind=(amount:int=$shared)\n  {amount}\n-> END\n";
    let last = ":: start default\n> b bind=(name:string=$shared)\n  {name}\n-> END\n";
    let mut document = Document::in_project(
        DocumentKey::new("z.recite")?,
        last,
        ProjectContext {
            documents: vec![SavedDocument::new(DocumentKey::new("a.recite")?, first)],
            schema: None,
        },
    )?;
    let original = document.preview_bindings();
    assert_eq!(original.len(), 1);
    assert_eq!(original[0].name, "amount");
    assert_eq!(original[0].value_type, InterpolationType::Integer);
    document.refresh_project(ProjectContext::default())?;
    assert_eq!(
        document.preview_bindings()[0].value_type,
        InterpolationType::String
    );
    assert_eq!(original[0].value_type, InterpolationType::Integer);
    Ok(())
}

#[test]
fn sibling_bindings_and_shadowed_saved_inputs_stay_independent()
-> Result<(), Box<dyn std::error::Error>> {
    let key = DocumentKey::new("scene.recite")?;
    let mut document = Document::in_project(
        key.clone(),
        source("open"),
        ProjectContext {
            documents: vec![SavedDocument::new(key.clone(), source("saved"))],
            schema: None,
        },
    )?;
    let original = document.preview_bindings();
    document.refresh_project(ProjectContext {
        documents: vec![SavedDocument::new(key.clone(), source("changed_saved"))],
        schema: None,
    })?;
    assert!(Arc::ptr_eq(&original, &document.preview_bindings()));
    let sibling = document.open_sibling_with_control(
        DocumentKey::new("sibling.recite")?,
        source("sibling"),
        ProjectContext {
            documents: vec![SavedDocument::new(key, source("saved"))],
            schema: None,
        },
        &CancellationToken::new(),
    )?;
    assert_eq!(
        sibling
            .preview_bindings()
            .iter()
            .map(|b| b.value.as_str())
            .collect::<Vec<_>>(),
        ["saved", "sibling"]
    );
    assert_eq!(document.preview_bindings()[0].value, "open");
    assert!(Arc::ptr_eq(&original, &document.preview_bindings()));
    Ok(())
}

#[test]
fn regional_bindings_reveal_later_definitions_after_remove_and_reorder()
-> Result<(), Box<dyn std::error::Error>> {
    let a = ":: first default\n> a bind=(amount:int=$shared)\n  {amount}\n";
    let b = ":: second\n> b bind=(name:string=$shared)\n  {name}\n";
    let mut document = Document::new(format!("{a}{b}"))?;
    let retained = document.preview_bindings();
    for source in [
        format!("{b}{a}"),
        b.to_owned(),
        format!("\n{a}{b}"),
        format!("{a}{b}{b}"),
    ] {
        document.replace_source(document.revision(), source.clone())?;
        assert_eq!(
            document.preview_bindings(),
            Document::new(source)?.preview_bindings()
        );
    }
    assert_eq!(retained[0].value_type, InterpolationType::Integer);
    Ok(())
}
