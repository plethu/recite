use super::*;

#[test]
fn same_id_different_asset_content_is_rejected() {
    let first = compile_asset_with_id(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
            "  First.\n",
            "-> END\n",
        ),
        "dialogue/same.recitec",
    );
    let second = compile_asset_with_id(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
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
fn mismatched_asset_identity_returns_structured_error() {
    let first = compile_asset_with_id(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
            "  Start.\n",
            "-> END\n",
        ),
        "dialogue/first.recitec",
    );
    let second = compile_asset_with_id(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
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
            "> start_line\n",
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
