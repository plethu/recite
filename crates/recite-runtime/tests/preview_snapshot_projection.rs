#[path = "support/preview.rs"]
mod preview_support;

use preview_support::asset;
use recite_core::ScalarValue;
use recite_runtime::{InterpolationValues, PreviewInputs, PreviewOptions, PreviewSession};

#[test]
fn prompt_projection_mutations_are_refused_without_asset_wide_lookup() {
    let asset = asset(concat!(
        ":: start default\n",
        "> prompt@12345678901234567890 speaker=hazel mood=calm bind=(count:int=$count)\n",
        "  One item.\n  | {count} items.\n",
        "  ? keep@12345678901234567891 echo=selected_text tone=plain\n",
        "    Keep.\n    -> END\n",
    ));
    let mut values = InterpolationValues::new();
    values.insert("count".to_owned(), ScalarValue::from(2_i64));
    let mut source = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("source");
    source.step(PreviewInputs::new().with_interpolation_values(&values));
    let encoded = source
        .snapshot()
        .expect("snapshot")
        .encode()
        .expect("encode");
    for (original, replacement) in [
        ("{count} items.", "{count} units."),
        ("Keep.", "Gone."),
        ("hazel", "rhea?"),
        ("calm", "warm"),
        ("plain", "rough"),
    ] {
        let mutated = replace_text(&encoded, original, replacement);
        let snapshot = recite_runtime::PreviewSnapshot::decode(&mutated).expect("valid wire");
        let mut receiver =
            PreviewSession::new(&asset, None, PreviewOptions::new()).expect("receiver");
        let before = receiver.session().clone();
        assert!(matches!(
            receiver.restore(snapshot),
            Err(recite_runtime::PreviewError::SnapshotStateMismatch)
        ));
        assert_eq!(*receiver.session(), before);
    }
}

#[test]
fn provider_rendered_projection_restores_without_provider_replay() {
    let asset = asset(concat!(
        ":: start default\n",
        "> prompt@12345678901234567890 bind=(count:int=$count)\n",
        "  One item.\n  | {count} items.\n",
        "  ? keep@12345678901234567891\n    Keep.\n    -> END\n",
    ));
    let mut values = InterpolationValues::new();
    values.insert("count".to_owned(), ScalarValue::from(2_i64));
    let mut source = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("source");
    source.step(PreviewInputs::new().with_interpolation_values(&values));
    let mut wire = source
        .snapshot()
        .expect("snapshot")
        .encode()
        .expect("encode");
    let start = wire
        .windows(b"2 items.".len())
        .position(|window| window == b"2 items.");
    assert!(
        start.is_some(),
        "rendered projection is present in the wire fixture"
    );
    let start = start.unwrap_or_default();
    wire[start..start + b"2 items.".len()].copy_from_slice(b"3 items.");
    let snapshot = recite_runtime::PreviewSnapshot::decode(&wire).expect("decode");
    let mut receiver = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("receiver");
    receiver
        .restore(snapshot)
        .expect("provider projection is authoritative");
    let rendered = match receiver.state().status() {
        recite_runtime::PreviewStatus::WaitingForChoice { prompt } => {
            prompt.line().map(|line| line.text.as_str())
        }
        _ => None,
    };
    assert_eq!(rendered, Some("3 items."));
}

#[test]
fn asset_derived_mismatch_rejects_even_with_a_consistent_projection() {
    let asset = asset(concat!(
        ":: start default\n",
        "> prompt@12345678901234567890\n  Prompt.\n",
        "  ? keep@12345678901234567891\n    Keep.\n    -> END\n",
    ));
    let mut source = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("source");
    source.step(PreviewInputs::new());
    let snapshot = source.snapshot().expect("snapshot");
    let mut changed = asset.clone();
    changed.lines[0].source_text = "Changed.".to_owned();
    changed.lines[0].authored_source_text = "Changed.".to_owned();
    let mut receiver =
        PreviewSession::new(&changed, None, PreviewOptions::new()).expect("receiver");
    let before = receiver.session().clone();
    assert!(matches!(
        receiver.restore(snapshot),
        Err(recite_runtime::PreviewError::SnapshotStateMismatch)
    ));
    assert_eq!(*receiver.session(), before);
}

#[test]
fn stale_ready_snapshot_rejects_revision_change_without_restart_requirement() {
    let asset = asset(concat!(
        ":: start default\n",
        "> first@12345678901234567890\n",
        "  First.\n",
        "> future@12345678901234567891\n",
        "  Original future.\n",
        "-> END\n",
    ));
    let source = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("source");
    let snapshot = source.snapshot().expect("snapshot");
    assert!(source.state().restart_required().is_none());

    let mut changed = asset.clone();
    changed.lines[1].source_text = "Changed future.".to_owned();
    changed.lines[1].authored_source_text = "Changed future.".to_owned();
    let mut receiver =
        PreviewSession::new(&changed, None, PreviewOptions::new()).expect("receiver");
    let before = receiver.session().clone();
    assert!(matches!(
        receiver.restore(snapshot),
        Err(recite_runtime::PreviewError::SnapshotStateMismatch)
    ));
    assert_eq!(*receiver.session(), before);
}

fn replace_text(bytes: &[u8], original: &str, replacement: &str) -> Vec<u8> {
    assert_eq!(
        original.len(),
        replacement.len(),
        "fixture replacement length"
    );
    let original = original.as_bytes();
    let mut output = bytes.to_vec();
    let Some(start) = output
        .windows(original.len())
        .position(|window| window == original)
    else {
        return output;
    };
    output[start..start + original.len()].copy_from_slice(replacement.as_bytes());
    output
}
