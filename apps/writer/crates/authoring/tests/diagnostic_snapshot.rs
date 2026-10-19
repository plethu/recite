use recite_compiler::authoring::SavedDocument;
use recite_core::DocumentKey;
use recite_writer_model::{Document, ProjectContext};

#[test]
fn diagnostic_views_preserve_source_order_and_retained_reports_across_edits()
-> Result<(), Box<dyn std::error::Error>> {
    let source = (0..10_000)
        .map(|n| format!(":: block_{n}\n-> missing_{n}\n"))
        .collect::<String>();
    let key = DocumentKey::new("a.recite")?;
    let other = SavedDocument::new(DocumentKey::new("b.recite")?, ":: other\n-> absent\n");
    let mut document = Document::in_project(
        key,
        &source,
        ProjectContext {
            documents: vec![other],
            schema: None,
        },
    )?;
    let snapshot = document.diagnostic_snapshot();
    let owned = document.diagnostics();
    assert!(snapshot == snapshot.clone());
    assert_eq!(snapshot.len(), owned.len());
    assert!(!snapshot.is_empty());
    for index in [0, 5_000, 9_999, owned.len() - 1] {
        assert_eq!(snapshot.get(index), owned.get(index));
    }
    assert!(snapshot.get(snapshot.len()).is_none());
    assert!(snapshot.get(usize::MAX).is_none());
    document.replace_source(document.revision(), ":: fixed\n-> END\n")?;
    let changed = document.diagnostic_snapshot();
    assert!(changed != snapshot);
    assert!(changed.len() < snapshot.len());
    assert_eq!(snapshot.get(9_999), owned.get(9_999));
    assert_eq!(snapshot.get(owned.len() - 1), owned.last());
    Ok(())
}
