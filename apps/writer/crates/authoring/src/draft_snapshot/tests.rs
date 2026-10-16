use super::*;
use std::io::{self, Write};

fn rope_storage(snapshot: &DraftSnapshot) -> &Arc<RopeDraft> {
    match &snapshot.0 {
        Contents::Rope(draft) => draft,
        Contents::Text(_) | Contents::Shared(_) => panic!("expected a Rope snapshot"),
    }
}

fn assert_lazy(snapshot: &DraftSnapshot) {
    assert!(rope_storage(snapshot).text.get().is_none());
}

#[test]
fn string_input_keeps_its_allocation_and_shares_clones() {
    let text = "existing String 🦊\r\n".repeat(100);
    let pointer = text.as_ptr();
    let snapshot = DraftSnapshot::from(text);
    let cloned = snapshot.clone();
    assert_eq!(snapshot.as_str().as_ptr(), pointer);
    assert_eq!(cloned.as_str().as_ptr(), pointer);
    assert!(snapshot == cloned);
}

#[test]
fn shared_source_keeps_its_allocation_and_matches_other_representations() {
    let source: Arc<str> = Arc::from("shared source 🦊\r\n".repeat(100));
    let snapshot = DraftSnapshot::from(source.clone());
    let rope = DraftSnapshot::from_rope(Rope::from_str(&source));
    let flat = DraftSnapshot::from(source.to_string());
    let separate = DraftSnapshot::from(Arc::<str>::from(source.as_ref()));
    assert_eq!(snapshot.as_str().as_ptr(), source.as_ptr());
    assert_eq!(snapshot.clone().as_str().as_ptr(), source.as_ptr());
    assert!(snapshot == snapshot.clone() && snapshot == separate);
    assert_eq!(snapshot, rope);
    assert_eq!(rope, snapshot);
    assert_eq!(snapshot, flat);
    assert_eq!(flat, snapshot);
    assert!(snapshot.matches_rope(&Rope::from_str(&source)));
    assert!(!snapshot.matches_rope(&Rope::from_str("changed")));
    assert_eq!(
        serde_json::to_vec(&snapshot).unwrap(),
        serde_json::to_vec(source.as_ref()).unwrap()
    );
    assert_eq!(format!("{snapshot:?}"), format!("{source:?}"));
    assert_lazy(&rope);
}

#[test]
fn logical_equality_ignores_representation_and_materialization() {
    let text = "same bytes é💬\r\n".repeat(100);
    let source = Rope::from_str(&text);
    let lazy = DraftSnapshot::from_rope(source.clone());
    let distinct = DraftSnapshot::from_rope(Rope::from_str(&text));
    let flat = DraftSnapshot::from(text.clone());
    assert!(lazy == lazy.clone());
    assert!(lazy == distinct);
    assert!(lazy == flat);
    assert!(flat == lazy);
    assert!(lazy.matches_rope(&source));
    assert!(flat.matches_rope(&source));
    assert!(lazy.matches_rope(&Rope::from_str(&text)));
    assert_lazy(&lazy);
    assert_lazy(&distinct);

    assert_eq!(lazy.as_str(), text);
    assert!(lazy == distinct && distinct == flat);
    assert_lazy(&distinct);
    for changed in [format!("{text}x"), text.replacen("same", "Some", 1)] {
        let changed = Rope::from_str(&changed);
        let changed_snapshot = DraftSnapshot::from_rope(changed.clone());
        assert!(!lazy.matches_rope(&changed));
        assert!(lazy != changed_snapshot && flat != changed_snapshot);
        assert_lazy(&changed_snapshot);
    }
}

#[test]
fn clones_share_snapshot_without_observing_later_editor_edits() -> Result<(), serde_json::Error> {
    let original = "immutable 💬\r\n".repeat(200);
    let mut editor = Rope::from_str(&original);
    let snapshot = DraftSnapshot::from_rope(editor.clone());
    let recovery = snapshot.clone();
    assert!(Arc::ptr_eq(
        rope_storage(&snapshot),
        rope_storage(&recovery)
    ));
    editor.insert(editor.len_chars() / 2, "later edit");
    assert!(snapshot == recovery);
    assert!(!snapshot.matches_rope(&editor));
    assert_eq!(
        serde_json::to_vec(&recovery)?,
        serde_json::to_vec(&original)?
    );
    assert_lazy(&snapshot);
    Ok(())
}

#[test]
fn ordinary_json_strings_stream_exact_bytes_without_materializing() -> Result<(), serde_json::Error>
{
    for text in [
        String::new(),
        "plain text".into(),
        "é💬\r\n\r\t\u{85}\u{2028}\u{2029}\"\\/".repeat(200),
        (0u8..=31).map(char::from).collect::<String>().repeat(100),
        format!("{}\"\\\r\n💬{}", "a".repeat(1023), "é".repeat(1023)),
    ] {
        let snapshot = DraftSnapshot::from_rope(Rope::from_str(&text));
        let encoded = serde_json::to_vec(&snapshot)?;
        assert_eq!(encoded, serde_json::to_vec(&text)?);
        assert_lazy(&snapshot);
        let restored: DraftSnapshot = serde_json::from_slice(&encoded)?;
        assert!(matches!(&restored.0, Contents::Text(_)));
        assert!(restored == snapshot);
        assert_lazy(&snapshot);
    }
    for invalid in ["null", "42", "[]", "{}"] {
        assert!(serde_json::from_str::<DraftSnapshot>(invalid).is_err());
    }
    Ok(())
}

#[test]
fn version_one_recovery_field_remains_an_ordinary_string() -> Result<(), serde_json::Error> {
    #[derive(Serialize, Deserialize)]
    enum View {
        Source,
        Passage(String),
        Block(String),
    }
    #[derive(Serialize)]
    struct Recovery<'a, T> {
        source: &'a str,
        view: &'a View,
        draft: T,
    }
    let text = "unapplied \"é💬\"\r\n\u{0}".repeat(100);
    let snapshot = DraftSnapshot::from_rope(Rope::from_str(&text));
    for view in [
        View::Source,
        View::Passage("id".into()),
        View::Block("root".into()),
    ] {
        let actual = Recovery {
            source: "accepted",
            view: &view,
            draft: &snapshot,
        };
        let previous = Recovery {
            source: "accepted",
            view: &view,
            draft: text.as_str(),
        };
        assert_eq!(serde_json::to_vec(&actual)?, serde_json::to_vec(&previous)?);
        assert_lazy(&snapshot);
    }
    Ok(())
}

#[test]
fn debug_uses_standard_content_formatters_without_materializing() {
    let text = "inspectable \"é💬\"\\\r\n\u{0}".repeat(100);
    let rope = Rope::from_str(&text);
    let snapshot = DraftSnapshot::from_rope(rope.clone());
    assert_eq!(format!("{snapshot:?}"), format!("{rope:?}"));
    assert_lazy(&snapshot);

    let flat = DraftSnapshot::from(text.clone());
    assert_eq!(format!("{flat:?}"), format!("{text:?}"));
}

#[test]
fn explicit_string_access_materializes_once_and_releases_with_last_snapshot() {
    let text = "materialize only when asked 🦊\r\n".repeat(100);
    let snapshot = DraftSnapshot::from_rope(Rope::from_str(&text));
    let cloned = snapshot.clone();
    let storage = Arc::downgrade(rope_storage(&snapshot));
    assert_lazy(&snapshot);
    let pointer = snapshot.as_str().as_ptr();
    assert_eq!(cloned.as_str().as_ptr(), pointer);
    assert_eq!(cloned.as_str(), text);
    assert!(rope_storage(&cloned).text.get().is_some());
    drop(snapshot);
    assert!(storage.upgrade().is_some());
    drop(cloned);
    assert!(storage.upgrade().is_none());
}

#[test]
fn successful_source_apply_releases_the_materialized_rope_draft() {
    let mut model = crate::Workbench::new(crate::FIXTURE).unwrap();
    model.select(crate::View::Source).unwrap();
    let text = format!("{}# Applied source\n", crate::FIXTURE);
    let draft = DraftSnapshot::from_rope(Rope::from_str(&text));
    let storage = Arc::downgrade(rope_storage(&draft));
    model.set_draft_snapshot(draft);
    model.apply().unwrap();
    assert!(storage.upgrade().is_none());
    assert_eq!(model.draft(), text);
    assert_eq!(model.draft().as_ptr(), model.document().source().as_ptr());
    assert!(!model.has_draft());
}

#[test]
fn streaming_write_failure_leaves_snapshot_lazy_and_retryable() -> Result<(), serde_json::Error> {
    struct Failed;
    impl Write for Failed {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("write failed"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let text = "recovery retry \"💬\"\r\n".repeat(200);
    let snapshot = DraftSnapshot::from_rope(Rope::from_str(&text));
    assert!(serde_json::to_writer(Failed, &snapshot).is_err());
    assert_lazy(&snapshot);
    assert_eq!(serde_json::to_vec(&snapshot)?, serde_json::to_vec(&text)?);
    assert_lazy(&snapshot);
    Ok(())
}
