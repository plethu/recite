use super::*;
use recite_core::DocumentKey;
use std::sync::Arc;

#[test]
fn equal_saved_bytes_share_storage_but_different_discovery_bytes_remain_authoritative() {
    let key = DocumentKey::new("scene.recite").unwrap();
    let other = DocumentKey::new("other.recite").unwrap();
    let saved: Arc<str> = Arc::from(":: saved default\n-> END\n".repeat(10_000));
    let mut context = ProjectContext {
        documents: vec![
            SavedDocument::new(key.clone(), saved.as_ref()),
            SavedDocument::new(other.clone(), ":: other\n"),
        ],
        schema: None,
    };
    assert_ne!(context.documents[0].text().as_ptr(), saved.as_ptr());
    let original_other = context.documents[1].clone();
    share_saved_source(&mut context, &key, &saved);
    assert_eq!(context.documents[0].text().as_ptr(), saved.as_ptr());
    assert_eq!(context.documents[1], original_other);
    let changed: Arc<str> = Arc::from(":: changed default\n-> END\n");
    share_saved_source(&mut context, &key, &changed);
    assert_eq!(context.documents[0].text().as_ptr(), saved.as_ptr());
    share_saved_source(
        &mut context,
        &DocumentKey::new("absent.recite").unwrap(),
        &changed,
    );
    assert_eq!(context.documents.len(), 2);
    assert_eq!(saved.as_ref(), context.documents[0].text());
}
