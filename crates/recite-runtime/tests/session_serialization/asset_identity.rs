use super::*;
use recite_core::{CompiledArgument, SchemaFingerprint, canonical_source_fingerprint};
use recite_runtime::DialogueSchemaFingerprintSnapshot;

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
fn same_header_different_compiled_payload_is_rejected() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "! deferred grant_item(original_item)\n",
            "> start_line@3a011ecfcf4c5ed87289\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");
    next(&asset, &mut session).expect("collects deferred effect and emits line");
    let snapshot = snapshot_session(&session);

    let mut modified = asset.clone();
    modified.effects[0].args[0] = CompiledArgument::Identifier("changed_item".to_owned());

    assert_eq!(asset.header, modified.header);
    assert_eq!(asset.sources, modified.sources);
    assert!(matches!(
        restore_session(&modified, snapshot),
        Err(DialogueError::AssetContentMismatch { reason, .. })
            if reason.contains("compiled payload fingerprint")
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
    incompatible_asset.header.compiler_version =
        recite_core::CompilerVersion::new("0.0.2").expect("valid compiler version");

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
fn unsupported_session_snapshot_formats_are_rejected() {
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
    for snapshot_format_version in [0, 2] {
        let mut snapshot = snapshot_session(&session);
        snapshot.snapshot_format_version = snapshot_format_version;

        assert_eq!(
            restore_session(&asset, snapshot),
            Err(DialogueError::UnsupportedSessionSnapshotFormat {
                snapshot_format_version,
            })
        );
    }
}

#[test]
fn unsupported_messagepack_snapshot_formats_are_rejected_before_shape_decode() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line@8c7dafa16b4a172020b1\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    for snapshot_format_version in [0, 2] {
        let bytes = rmp_serde::to_vec(&[snapshot_format_version])
            .expect("encodes unsupported compact prefix");

        assert_eq!(
            decode_session_messagepack(&asset, &bytes),
            Err(DialogueError::UnsupportedSessionSnapshotFormat {
                snapshot_format_version,
            })
        );
    }
}

#[test]
fn messagepack_snapshot_without_payload_fingerprint_is_rejected() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line@4ecb6bdcfbdc7a839198\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    let session = start_scene(&asset, None).expect("starts");
    let mut snapshot =
        serde_json::to_value(snapshot_session(&session)).expect("serializes snapshot");
    let fields = snapshot
        .as_object_mut()
        .expect("snapshot serializes as a map");
    assert!(fields.remove("compiled_payload_fingerprint").is_some());
    let bytes =
        rmp_serde::to_vec_named(&snapshot).expect("encodes snapshot without payload fingerprint");

    let error = decode_session_messagepack(&asset, &bytes).expect_err("missing fingerprint");
    assert!(matches!(
        error,
        DialogueError::SessionSnapshotDecodeFailed { .. }
    ));
}
