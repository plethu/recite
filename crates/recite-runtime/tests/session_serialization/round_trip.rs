use super::*;

#[test]
fn messagepack_round_trip_resumes_line_progress_without_asset_payload() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> first\n",
            "  First.\n",
            "> second\n",
            "  Second.\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(next(&asset, &mut session), "first", "First.");
    let bytes = encode_session_messagepack(&session).expect("encodes session");
    let mut restored =
        decode_session_messagepack(&asset, &bytes).expect("restores from messagepack");

    assert_line(next(&asset, &mut restored), "second", "Second.");
    assert_eq!(next(&asset, &mut restored), Ok(empty_end()));
}

#[test]
fn structured_snapshot_records_locale_and_compact_runtime_location() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    let locale = LocaleId::new("en-GB").expect("valid locale");
    let mut session = start_scene_with_options(
        &asset,
        None,
        DialogueSessionOptions::new().with_locale(locale.clone()),
    )
    .expect("starts with options");
    assert_line(next(&asset, &mut session), "start_line", "Start.");

    let snapshot = snapshot_session(&session);
    assert_eq!(snapshot.snapshot_format_version, 1);
    assert_eq!(snapshot.asset_id, "dialogue/main.recitec");
    assert_eq!(snapshot.compiler_version, "0.0.1");
    assert_eq!(snapshot.source_map_id, "dialogue/main.recitec.map");
    assert_eq!(snapshot.sources.len(), 1);
    assert_eq!(snapshot.current_block, 0);
    assert_eq!(snapshot.current_range.start, 0);
    assert_eq!(snapshot.locale.as_deref(), Some("en-GB"));
    assert!(snapshot.pending_prompt.is_none());
    assert!(snapshot.deferred_effects.is_empty());

    let restored = restore_session(&asset, snapshot).expect("restores snapshot");
    assert_eq!(restored.locale(), Some(&locale));
}
