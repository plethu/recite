use std::sync::Arc;

use recite_compiler::authoring::{DocumentVersion, OpenDocument, SavedDocument};
use recite_core::DocumentKey;

#[test]
fn shared_and_independent_inputs_keep_value_equality() {
    let key = DocumentKey::new("a.recite").unwrap();
    let other_key = DocumentKey::new("b.recite").unwrap();
    let text: Arc<str> = Arc::from(":: a\r\n> id\r\n  Café 🦀\r\n");
    let saved = SavedDocument::from_shared(key.clone(), Arc::clone(&text));
    assert_eq!(saved, saved.clone());
    assert_eq!(saved, SavedDocument::new(key.clone(), text.as_ref()));
    assert_ne!(
        saved,
        SavedDocument::from_shared(other_key.clone(), Arc::clone(&text))
    );
    assert_ne!(
        saved,
        SavedDocument::new(key.clone(), text.replace("Café", "Changed"))
    );
    let version = DocumentVersion::new(7);
    let open = OpenDocument::from_shared(key.clone(), version, Arc::clone(&text));
    assert_eq!(open, open.clone());
    assert_eq!(open, OpenDocument::new(key.clone(), version, text.as_ref()));
    assert_ne!(
        open,
        OpenDocument::from_shared(other_key, version, Arc::clone(&text))
    );
    assert_ne!(
        open,
        OpenDocument::from_shared(key.clone(), DocumentVersion::new(8), Arc::clone(&text))
    );
    assert_ne!(
        open,
        OpenDocument::new(key, version, text.replace("Café", "Changed"))
    );
}
