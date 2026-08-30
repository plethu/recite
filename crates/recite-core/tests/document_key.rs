#![cfg(test)]

use std::collections::BTreeSet;
use std::hash::{DefaultHasher, Hash, Hasher};

use recite_core::{DocumentKey, DocumentKeyError};

#[test]
fn valid_keys_preserve_normalized_slash_syntax() {
    for value in [
        "dialogue/tavern.recite",
        "nested/scene/start.recite",
        "café/été.recite",
    ] {
        let key = DocumentKey::new(value).expect("valid project-relative key");
        assert_eq!(key.as_str(), value);
        assert_eq!(key.to_string(), value);
    }
}

#[test]
fn keys_have_stable_order_and_hash_identity() {
    let keys = [
        DocumentKey::new("z.recite").expect("valid key"),
        DocumentKey::new("a.recite").expect("valid key"),
        DocumentKey::new("nested/a.recite").expect("valid key"),
    ];
    let ordered = keys.iter().cloned().collect::<BTreeSet<_>>();
    assert_eq!(
        ordered.iter().map(DocumentKey::as_str).collect::<Vec<_>>(),
        ["a.recite", "nested/a.recite", "z.recite"]
    );
    let first = DocumentKey::try_from("a.recite").expect("valid key");
    let second = DocumentKey::new("a.recite").expect("valid key");
    let mut first_hash = DefaultHasher::new();
    first.hash(&mut first_hash);
    let mut second_hash = DefaultHasher::new();
    second.hash(&mut second_hash);
    assert_eq!(first_hash.finish(), second_hash.finish());
}

#[test]
fn invalid_key_shapes_report_typed_context() {
    assert!(matches!(DocumentKey::new(""), Err(DocumentKeyError::Empty)));
    assert!(matches!(
        DocumentKey::new("/dialogue.recite"),
        Err(DocumentKeyError::Absolute { value }) if value == "/dialogue.recite"
    ));
    assert!(matches!(
        DocumentKey::new("C:/dialogue.recite"),
        Err(DocumentKeyError::DrivePrefix { value }) if value == "C:/dialogue.recite"
    ));
    assert!(matches!(
        DocumentKey::new("//server/share/dialogue.recite"),
        Err(DocumentKeyError::Unc { value }) if value == "//server/share/dialogue.recite"
    ));
    assert!(matches!(
        DocumentKey::new(r"\\server\share\dialogue.recite"),
        Err(DocumentKeyError::Unc { value }) if value == r"\\server\share\dialogue.recite"
    ));
    assert!(matches!(
        DocumentKey::new(r"dialogue\start.recite"),
        Err(DocumentKeyError::Backslash { value }) if value == r"dialogue\start.recite"
    ));
    assert!(matches!(
        DocumentKey::new("dialogue//start.recite"),
        Err(DocumentKeyError::EmptyComponent { index: 1, .. })
    ));
    assert!(matches!(
        DocumentKey::new("dialogue/./start.recite"),
        Err(DocumentKeyError::CurrentDirectoryComponent { index: 1, .. })
    ));
    assert!(matches!(
        DocumentKey::new("dialogue/../start.recite"),
        Err(DocumentKeyError::ParentDirectoryComponent { index: 1, .. })
    ));
}
