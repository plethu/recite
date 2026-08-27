use super::*;
use recite_core::{SchemaFingerprint, canonical_source_fingerprint};
use recite_runtime::{
    DialogueSchemaFingerprintSnapshot, DialogueSessionFrameSnapshot,
    DialogueSessionPendingPromptSnapshot, DialogueSessionRangeSnapshot, DialogueSessionSnapshot,
    DialogueSessionSourceSnapshot, SESSION_SNAPSHOT_FORMAT_VERSION_V0,
};
use serde::Serialize;

#[test]
fn same_id_different_asset_content_is_rejected() {
    let first = compile_asset_with_id(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line@3a011ecfcf4c5ed87289\n",
            "  First.\n",
            "-> END\n",
        ),
        "dialogue/same.recitec",
    );
    let second = compile_asset_with_id(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line@8a5a460c1266a6c5f9df\n",
            "  Changed.\n",
            "-> END\n",
        ),
        "dialogue/same.recitec",
    );
    let session = start_scene(&first, None).expect("starts");

    assert!(matches!(
        restore_session(&second, snapshot_session(&session)),
        Err(DialogueError::AssetContentMismatch { .. })
    ));
}

#[test]
fn schema_fingerprint_mismatch_returns_structured_error() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line@3a011ecfcf4c5ed87289\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    let session = start_scene(&asset, None).expect("starts");
    let mut incompatible_asset = asset.clone();
    incompatible_asset.header.schema_fingerprint =
        SchemaFingerprint::Fingerprint(canonical_source_fingerprint("different schema"));

    let error = restore_session(&incompatible_asset, snapshot_session(&session))
        .expect_err("schema mismatch is rejected");
    let DialogueError::SchemaMismatch {
        asset_id,
        expected_schema_fingerprint,
        actual_schema_fingerprint,
    } = error
    else {
        panic!("expected a structured schema mismatch");
    };
    assert_eq!(asset_id, "dialogue/main.recitec");
    assert_eq!(
        expected_schema_fingerprint,
        DialogueSchemaFingerprintSnapshot::NoSchema
    );
    assert!(matches!(
        actual_schema_fingerprint,
        DialogueSchemaFingerprintSnapshot::Fingerprint(_)
    ));
}

#[test]
fn mismatched_asset_identity_returns_structured_error() {
    let first = compile_asset_with_id(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line@0ee5f0964c10f6c097e2\n",
            "  Start.\n",
            "-> END\n",
        ),
        "dialogue/first.recitec",
    );
    let second = compile_asset_with_id(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line@98291b3b81e255db8d44\n",
            "  Start.\n",
            "-> END\n",
        ),
        "dialogue/second.recitec",
    );
    let session = start_scene(&first, None).expect("starts");

    assert!(matches!(
        restore_session(&second, snapshot_session(&session)),
        Err(DialogueError::AssetMismatch { .. })
    ));
}

#[test]
fn mismatched_asset_version_returns_structured_error() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line@c358dff16d0753a90d8b\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    let session = start_scene(&asset, None).expect("starts");
    let mut snapshot = snapshot_session(&session);
    snapshot.asset_format_version = 99;

    assert!(matches!(
        restore_session(&asset, snapshot),
        Err(DialogueError::AssetMismatch {
            expected_format_version: 99,
            actual_format_version: 0,
            ..
        })
    ));
}

#[test]
fn previous_session_snapshot_format_is_rejected() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line@e9a0744c9109baec57e7\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    let session = start_scene(&asset, None).expect("starts");
    let mut snapshot = snapshot_session(&session);
    snapshot.snapshot_format_version = 0;

    assert_eq!(
        restore_session(&asset, snapshot),
        Err(DialogueError::UnsupportedSessionSnapshotFormat {
            snapshot_format_version: 0,
        })
    );
}

#[test]
fn previous_messagepack_snapshot_format_is_rejected_before_shape_decode() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line@8c7dafa16b4a172020b1\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    let session = start_scene(&asset, None).expect("starts");
    let mut snapshot = snapshot_session(&session);
    snapshot.snapshot_format_version = SESSION_SNAPSHOT_FORMAT_VERSION_V0;
    let bytes = rmp_serde::to_vec(&V0DialogueSessionSnapshot::from(snapshot))
        .expect("encodes v0-shaped snapshot");

    assert_eq!(
        decode_session_messagepack(&asset, &bytes),
        Err(DialogueError::UnsupportedSessionSnapshotFormat {
            snapshot_format_version: SESSION_SNAPSHOT_FORMAT_VERSION_V0,
        })
    );
}

#[derive(Serialize)]
struct V0DialogueSessionSnapshot {
    snapshot_format_version: u16,
    asset_id: String,
    asset_format_version: u16,
    asset_compiler_compatibility_version: u16,
    compiler_version: String,
    source_map_id: String,
    schema_fingerprint: DialogueSchemaFingerprintSnapshot,
    sources: Vec<DialogueSessionSourceSnapshot>,
    current_block: u32,
    current_range: DialogueSessionRangeSnapshot,
    next_statement: u32,
    continuation_stack: Vec<DialogueSessionFrameSnapshot>,
    pending_prompt: Option<DialogueSessionPendingPromptSnapshot>,
    previous_prompt_choices: Vec<String>,
    selected_choice_history: Vec<String>,
    deferred_effects: Vec<DialogueDeferredEffectSnapshot>,
    locale: Option<String>,
    trace_counter: u64,
    ended: bool,
}

impl From<DialogueSessionSnapshot> for V0DialogueSessionSnapshot {
    fn from(snapshot: DialogueSessionSnapshot) -> Self {
        Self {
            snapshot_format_version: snapshot.snapshot_format_version,
            asset_id: snapshot.asset_id,
            asset_format_version: snapshot.asset_format_version,
            asset_compiler_compatibility_version: snapshot.asset_compiler_compatibility_version,
            compiler_version: snapshot.compiler_version,
            source_map_id: snapshot.source_map_id,
            schema_fingerprint: snapshot.schema_fingerprint,
            sources: snapshot.sources,
            current_block: snapshot.current_block,
            current_range: snapshot.current_range,
            next_statement: snapshot.next_statement,
            continuation_stack: snapshot.continuation_stack,
            pending_prompt: snapshot.pending_prompt,
            previous_prompt_choices: snapshot.previous_prompt_choices,
            selected_choice_history: snapshot.selected_choice_history,
            deferred_effects: snapshot.deferred_effects,
            locale: snapshot.locale,
            trace_counter: snapshot.trace_counter,
            ended: snapshot.ended,
        }
    }
}
