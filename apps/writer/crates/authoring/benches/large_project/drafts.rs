//! Source widget snapshots and dirty/recovery observation, without flattening.
use super::measure;
use recite_writer_model::{DraftSnapshot, View, Workbench};
use ropey::Rope;
use serde_json::Value;

pub(super) fn run(
    model: &mut Workbench,
    rows: &mut Vec<Value>,
) -> Result<(), Box<dyn std::error::Error>> {
    let accepted = model.document().source_snapshot();
    model.discard();
    for _ in 0..5 {
        measure("select_source_snapshot", rows, || {
            model.select(View::Source)
        })?;
        assert_eq!(model.draft().as_ptr(), accepted.as_ptr());
        assert!(!model.has_draft());
    }
    let mut source = Rope::from_str(model.draft());
    for _ in 0..5 {
        source.insert(source.len_chars(), "💬");
        let snapshot = measure("source_widget_snapshot", rows, || {
            model.set_draft_snapshot(DraftSnapshot::from_rope(source.clone()));
            assert!(model.has_draft());
            model.recovery()
        });
        assert!(snapshot.draft_snapshot().matches_rope(&source));
        assert!(std::sync::Arc::ptr_eq(
            &snapshot.source_snapshot(),
            &accepted
        ));
        let wire = serde_json::to_value(&snapshot)?;
        assert_eq!(wire["draft"], String::from(&source));
        assert_eq!(model.document().source(), accepted.as_ref());
    }
    // Equal byte length exercises dirty checks without relying on a size mismatch.
    let mut source = Rope::from_str(&accepted);
    let last = source.len_chars() - 1;
    let original = source.char(last);
    source.remove(last..last + 1);
    source.insert_char(last, if original == 'x' { 'y' } else { 'x' });
    assert_eq!(source.len_bytes(), accepted.len());
    let snapshot = measure("source_widget_equal_length_snapshot", rows, || {
        model.set_draft_snapshot(DraftSnapshot::from_rope(source.clone()));
        assert!(model.has_draft());
        model.recovery()
    });
    assert!(snapshot.draft_snapshot().matches_rope(&source));
    assert_eq!(
        serde_json::to_value(&snapshot)?["draft"],
        String::from(&source)
    );
    model.discard();
    assert!(!model.has_draft());
    Ok(())
}
