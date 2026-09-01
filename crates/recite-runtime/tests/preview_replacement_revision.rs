#[path = "support/preview.rs"]
mod preview_support;

use preview_support::asset;
use recite_runtime::{PreviewEvent, PreviewOptions, PreviewSession};

#[test]
fn same_id_revisions_clear_when_the_active_payload_returns() {
    let active = revision_asset("source-a", "A.");
    let replacement = revision_asset("source-b", "B.");
    let mut preview = PreviewSession::new(&active, None, PreviewOptions::new()).expect("active");

    let output = preview.assess_asset(&replacement).expect("assess");
    assert!(matches!(
        output.events(),
        [PreviewEvent::RestartRequired { .. }]
    ));
    let requirement = preview.state().restart_required().expect("requirement");
    assert_eq!(requirement.active_asset(), requirement.replacement_asset());
    assert_ne!(
        requirement.active_revision(),
        requirement.replacement_revision()
    );

    let output = preview.assess_asset(&active).expect("assess");
    assert!(output.events().is_empty());
    assert!(preview.state().restart_required().is_none());
}

#[test]
fn snapshot_between_same_id_replacements_can_return_to_active_payload() {
    let active = revision_asset("source-a", "A.");
    let replacement = revision_asset("source-b", "B.");
    let mut preview = PreviewSession::new(&active, None, PreviewOptions::new()).expect("active");

    preview
        .assess_asset(&replacement)
        .expect("assess replacement");
    let encoded = preview
        .snapshot()
        .expect("snapshot")
        .encode()
        .expect("encode");
    let decoded = recite_runtime::PreviewSnapshot::decode(&encoded).expect("decode");
    let mut restored = PreviewSession::new(&active, None, PreviewOptions::new()).expect("restore");
    restored.restore(decoded).expect("restore snapshot");

    let output = restored
        .assess_asset(&active)
        .expect("assess active payload");
    assert!(output.events().is_empty());
    assert!(restored.state().restart_required().is_none());
}

#[test]
fn same_id_candidate_revision_updates_and_round_trips() {
    let active = revision_asset("source-a", "A.");
    let replacement_b = revision_asset("source-b", "B.");
    let replacement_c = revision_asset("source-c", "C.");
    let mut preview = PreviewSession::new(&active, None, PreviewOptions::new()).expect("active");

    preview.assess_asset(&replacement_b).expect("assess");
    let first = preview.state().restart_required().cloned().expect("first");
    preview.assess_asset(&replacement_c).expect("assess");
    let second = preview.state().restart_required().cloned().expect("second");
    assert_eq!(first.active_revision(), second.active_revision());
    assert_ne!(first.replacement_revision(), second.replacement_revision());

    let encoded = preview
        .snapshot()
        .expect("snapshot")
        .encode()
        .expect("encode");
    let decoded = recite_runtime::PreviewSnapshot::decode(&encoded).expect("decode");
    let mut restored = PreviewSession::new(&active, None, PreviewOptions::new()).expect("restore");
    restored.restore(decoded).expect("restore snapshot");
    assert_eq!(restored.state().restart_required(), Some(&second));
}

#[test]
fn a_new_session_on_replacement_is_a_new_active_revision() {
    let replacement = revision_asset("source-b", "B.");
    let preview = PreviewSession::new(&replacement, None, PreviewOptions::new()).expect("new");
    assert!(preview.state().restart_required().is_none());
    assert_eq!(preview.state().asset_id(), &replacement.header.asset_id);
}

fn revision_asset(source_revision: &str, line: &str) -> recite_core::CompiledDialogue {
    let mut asset = asset(&format!(
        ":: start default\n> line@12345678901234567890\n  {line}\n-> END\n"
    ));
    asset.lines[0].source_text = format!("{line} {source_revision}");
    asset.lines[0].authored_source_text = format!("{line} {source_revision}");
    asset
}
