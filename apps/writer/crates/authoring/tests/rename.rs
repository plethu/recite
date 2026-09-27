use recite_compiler::authoring::SavedDocument;
use recite_core::DocumentKey;
use recite_writer_model::{Document, ProjectContext, rename_manifest_source};
#[test]
fn compiler_rename_includes_qualified_references_without_touching_frozen_ids()
-> Result<(), Box<dyn std::error::Error>> {
    let source = ":: start default\n> line@11111111111111111111\n  À neuf.\n-> END\n";
    let document = Document::in_project(
        DocumentKey::new("a.recite")?,
        source,
        ProjectContext {
            documents: vec![SavedDocument::new(
                DocumentKey::new("b.recite")?,
                ":: elsewhere\n-> a.recite::start\n",
            )],
            schema: None,
        },
    )?;
    let plan = document.plan_project_rename("start", "renamed")?;
    assert_eq!(plan.changes.len(), 2);
    assert!(plan.changes[0].after.contains("line@11111111111111111111"));
    assert!(plan.changes[0].after.contains("À neuf."));
    assert!(plan.changes[1].after.contains("a.recite::renamed"));
    Ok(())
}
#[test]
fn manifest_scene_start_rename_preserves_comments_and_unrelated_values()
-> Result<(), Box<dyn std::error::Error>> {
    let source = "format_version = 1\n# keep me\n[[scenes]]\nid = 'arrival'\nasset = 'build/a.recitec'\nblock = 'start' # entry point\nparticipants = ['mara']\n";
    let renamed = rename_manifest_source(source, "start", "renamed")?;
    assert!(renamed.contains("# keep me"));
    assert!(renamed.contains("block = \"renamed\" # entry point"));
    assert!(renamed.contains("participants = ['mara']"));
    assert_eq!(rename_manifest_source(source, "other", "renamed")?, source);
    assert!(rename_manifest_source(source, "start", "END").is_err());
    Ok(())
}
