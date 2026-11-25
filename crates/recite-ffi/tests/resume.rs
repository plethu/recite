//! `recite_session_prepare_resume`: restoring saves against patched assets.

#[path = "support/mod.rs"]
mod support;

use support::*;

#[test]
fn prepare_resume_reports_a_fallback_with_saved_effects_and_no_handle() {
    let base = concat!(
        ":: start default\n",
        "! deferred remember(start)\n",
        "> @t1jmfck\n",
        "  Save here.\n",
        "  ? @q2fjxbx\n",
        "    Continue.\n",
        "    -> END\n",
    );
    let base_bytes = compile_to_bytes(base);
    let changed_bytes = compile_to_bytes(
        &base
            .replace("> @t1jmfck", "> @z8wq4tn")
            .replace("? @q2fjxbx", "? @k3vbn7d"),
    );
    let mut base_asset = 0;
    let mut changed_asset = 0;
    for (bytes, asset) in [
        (&base_bytes, &raw mut base_asset),
        (&changed_bytes, &raw mut changed_asset),
    ] {
        assert_eq!(
            unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), asset) },
            ReciteStatus::Ok
        );
    }
    let mut session = 0;
    let mut batch = ReciteBuffer::null();
    assert_eq!(
        unsafe {
            recite_session_start(
                base_asset,
                std::ptr::null(),
                std::ptr::null(),
                &raw mut session,
                &raw mut batch,
            )
        },
        ReciteStatus::Ok
    );
    unsafe { recite_buffer_free(&raw mut batch) };
    let mut snapshot = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_session_snapshot(session, &raw mut snapshot) },
        ReciteStatus::Ok
    );
    recite_session_free(session);

    let mut resumed = 7;
    let mut report = ReciteBuffer::null();
    let status = unsafe {
        recite_session_prepare_resume(
            changed_asset,
            snapshot.data,
            snapshot.len,
            RECITE_RESUME_LEVEL_RELOCATED,
            &raw mut resumed,
            &raw mut report,
        )
    };
    assert_eq!(status, ReciteStatus::Ok);
    assert_eq!(resumed, 0, "a fallback allocates no session");
    let value = decode_batch(&report);
    assert_eq!(value["outcome"], "fallback");
    assert_eq!(value["reason"], "landmark_removed");
    assert_eq!(value["reason_landmark"]["kind"], "prompt");
    assert_eq!(value["block_id"], "start");
    assert_eq!(value["block_available"], true);
    assert_eq!(value["ended"], false);
    assert_eq!(value["deferred_effects"][0]["function"], "remember");
    assert!(value["pending_effect"].is_null());
    unsafe { recite_buffer_free(&raw mut report) };

    let status = unsafe {
        recite_session_prepare_resume(
            base_asset,
            snapshot.data,
            snapshot.len,
            99,
            &raw mut resumed,
            &raw mut report,
        )
    };
    assert_eq!(status, ReciteStatus::Validation);

    let status = unsafe {
        recite_session_prepare_resume(
            base_asset,
            snapshot.data,
            snapshot.len,
            RECITE_RESUME_LEVEL_EXACT,
            &raw mut resumed,
            &raw mut report,
        )
    };
    assert_eq!(status, ReciteStatus::Ok);
    assert_ne!(resumed, 0);
    assert_eq!(decode_batch(&report)["level"], "exact");
    recite_session_free(resumed);
    unsafe {
        recite_buffer_free(&raw mut report);
        recite_buffer_free(&raw mut snapshot);
    }
    recite_asset_free(base_asset);
    recite_asset_free(changed_asset);
}
