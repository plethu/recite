//! Source widget snapshots and dirty/recovery observation, without flattening.
use super::measure;
use recite_writer_model::{DraftSnapshot, EditOperation, View, Workbench};
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
    for _ in 0..5 {
        let mut source = Rope::from_str(model.document().source());
        source.insert(source.len_chars(), "\n# Applied 🦊\n");
        model.set_draft_snapshot(DraftSnapshot::from_rope(source.clone()));
        let retained = model.recovery();
        measure("source_rope_apply", rows, || model.apply())?;
        assert_eq!(model.document().source(), String::from(&source));
        assert_eq!(
            retained.draft_snapshot().as_str().as_ptr(),
            model.document().source().as_ptr()
        );
        assert!(!model.has_draft());
        model.undo()?;
        assert_eq!(model.document().source(), accepted.as_ref());
    }
    for _ in 0..5 {
        let mut rope = Rope::from_str(model.document().source());
        rope.insert(rope.len_chars(), "\n# Prepared 🦊\n");
        model.set_draft_snapshot(DraftSnapshot::from_rope(rope.clone()));
        let request = measure("source_edit_capture", rows, || {
            model.edit_request(EditOperation::Apply)
        })?
        .ok_or("source edit request")?;
        let mut prepared = measure("source_edit_prepare", rows, || {
            request.prepare(&recite_compiler::authoring::CancellationToken::new())
        })?;
        measure("source_edit_accept", rows, || {
            model.accept_edit(&mut prepared)
        })?;
        assert_eq!(model.document().source(), String::from(&rope));
        assert!(!model.has_draft());
        model.undo()?;
        assert_eq!(model.document().source(), accepted.as_ref());
    }
    model.select(View::Passage("00000000000000000000".into()))?;
    let original = model.draft().to_owned();
    model.set_draft(format!("{original} Prepared history."));
    model.apply()?;
    for _ in 0..5 {
        for operation in [EditOperation::Undo, EditOperation::Redo] {
            let request = measure("passage_history_capture", rows, || {
                model.edit_request(operation)
            })?
            .ok_or("passage history")?;
            let mut prepared = measure("passage_history_prepare", rows, || {
                request.prepare(&recite_compiler::authoring::CancellationToken::new())
            })?;
            measure("passage_history_accept", rows, || {
                model.accept_edit(&mut prepared)
            })?;
            assert_eq!(
                model.draft(),
                if operation == EditOperation::Undo {
                    original.clone()
                } else {
                    format!("{original} Prepared history.")
                }
            );
            assert!(!model.has_draft());
            assert!(model.document().cached_passages().is_some());
            assert!(model.document().cached_script().is_some());
            measure("passage_history_retire", rows, || drop(prepared));
        }
    }
    for _ in 0..5 {
        let before = model.draft().to_owned();
        model.set_draft(format!("{before} Prepared prose."));
        let request = measure("prose_edit_capture", rows, || {
            model.edit_request(EditOperation::Apply)
        })?
        .ok_or("prose edit request")?;
        let mut prepared = measure("prose_edit_prepare", rows, || {
            request.prepare(&recite_compiler::authoring::CancellationToken::new())
        })?;
        measure("prose_edit_accept", rows, || {
            model.accept_edit(&mut prepared)
        })?;
        assert_eq!(model.draft(), format!("{before} Prepared prose."));
        assert!(!model.has_draft());
        measure("prose_edit_retire", rows, || drop(prepared));
        model.undo()?;
        assert_eq!(model.draft(), before);
    }
    for _ in 0..5 {
        let request = measure("navigation_capture", rows, || model.navigation_request());
        let mut prepared = measure("navigation_prepare", rows, || {
            let mut prepared =
                request.prepare(None, &recite_compiler::authoring::CancellationToken::new())?;
            prepared.model_mut().select(View::Source)?;
            Ok::<_, recite_writer_model::WorkbenchError>(prepared)
        })?;
        measure("navigation_accept", rows, || {
            model.accept_navigation(&mut prepared)
        })?;
        assert_eq!(model.view(), &View::Source);
        assert!(!model.has_draft());
        measure("navigation_retire", rows, || drop(prepared));
        model.select(View::Passage("00000000000000000000".into()))?;
    }
    Ok(())
}

#[cfg(feature = "heap-profile")]
pub(super) fn budget(row: &Value) -> Option<u64> {
    let (bytes, allocations) = match row["operation"].as_str()? {
        "prose_edit_capture" => (128, 4),
        "prose_edit_prepare" => (650_000, 1_250),
        "prose_edit_accept" => (128, 1),
        "navigation_capture" => (128, 3),
        "navigation_prepare" => (128, 4),
        "navigation_accept" => (1, 0),
        _ => return None,
    };
    dhat::assert!(
        row["allocations"]
            .as_u64()
            .is_some_and(|count| count <= allocations)
    );
    Some(bytes)
}
