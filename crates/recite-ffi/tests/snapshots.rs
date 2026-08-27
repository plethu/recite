#[path = "support/mod.rs"]
mod support;

use support::*;

#[test]
fn snapshot_restore_round_trip() {
    // Snapshot at a pending prompt; restore must return empty batch (prompt still pending).
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        "> save_prompt@40000000000000000001\n",
        "  Save here.\n",
        "  ? go@40000000000000000002\n",
        "    Continue.\n",
        "    -> after\n",
        ":: after\n",
        "> after_line@40000000000000000003\n",
        "  Restored.\n",
        "-> END\n",
    ));
    let mut asset: u64 = 0;
    unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) };

    // Start — session stops at prompt.
    let mut session1: u64 = 0;
    let mut batch1 = ReciteBuffer::null();
    unsafe {
        recite_session_start(
            asset,
            std::ptr::null(),
            std::ptr::null(),
            &raw mut session1,
            &raw mut batch1,
        )
    };
    let decoded1 = decode_batch(&batch1);
    let kinds1 = event_kinds(&decoded1);
    assert_eq!(kinds1, ["prompt"], "start stops at prompt");
    unsafe { recite_buffer_free(&raw mut batch1) };

    // Snapshot.
    let mut snap = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_session_snapshot(session1, &raw mut snap) },
        ReciteStatus::Ok
    );
    recite_session_free(session1);

    // Restore — empty batch because prompt is still pending.
    let mut session2: u64 = 0;
    let mut batch2 = ReciteBuffer::null();
    let restore_status = unsafe {
        recite_session_restore(
            asset,
            snap.data,
            snap.len,
            &raw mut session2,
            &raw mut batch2,
        )
    };
    assert_eq!(restore_status, ReciteStatus::Ok, "restore succeeds");
    assert_ne!(session2, 0);

    // Restored batch is empty (prompt pending, not re-emitted).
    let value2 = decode_batch(&batch2);
    assert_eq!(
        value2["events"].as_array().unwrap().len(),
        0,
        "restored batch is empty"
    );

    // Can still make the choice from the restored session.
    let choice_cstr = cstr("40000000000000000002");
    let mut batch3 = ReciteBuffer::null();
    let choose_status =
        unsafe { recite_session_choose(session2, choice_cstr.as_ptr(), &raw mut batch3) };
    assert_eq!(choose_status, ReciteStatus::Ok, "choose after restore");
    let decoded3 = decode_batch(&batch3);
    let kinds3 = event_kinds(&decoded3);
    assert!(kinds3.contains(&"line"), "post-restore choice gives a line");

    unsafe { recite_buffer_free(&raw mut snap) };
    unsafe { recite_buffer_free(&raw mut batch2) };
    unsafe { recite_buffer_free(&raw mut batch3) };
    recite_session_free(session2);
    recite_asset_free(asset);
}
#[test]
fn restore_from_ended_session_returns_no_active_session() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        "> line@90000000000000000001\n",
        "  Done.\n",
        "-> END\n",
    ));
    let mut asset: u64 = 0;
    unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) };

    // Run to end.
    let mut session: u64 = 0;
    let mut batch = ReciteBuffer::null();
    unsafe {
        recite_session_start(
            asset,
            std::ptr::null(),
            std::ptr::null(),
            &raw mut session,
            &raw mut batch,
        )
    };
    let value = decode_batch(&batch);
    assert!(event_kinds(&value).contains(&"end"), "session reaches end");
    unsafe { recite_buffer_free(&raw mut batch) };

    // Snapshot the ended session state.
    let mut snap = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_session_snapshot(session, &raw mut snap) },
        ReciteStatus::Ok,
        "can snapshot ended session"
    );
    recite_session_free(session);

    // Restoring an ended-session snapshot must return NoActiveSession.
    let mut session2: u64 = 0;
    let mut batch2 = ReciteBuffer::null();
    let restore_status = unsafe {
        recite_session_restore(
            asset,
            snap.data,
            snap.len,
            &raw mut session2,
            &raw mut batch2,
        )
    };
    assert_eq!(
        restore_status,
        ReciteStatus::NoActiveSession,
        "restoring ended session snapshot returns NoActiveSession"
    );

    unsafe { recite_buffer_free(&raw mut snap) };
    unsafe { recite_buffer_free(&raw mut batch2) };
    recite_asset_free(asset);
}

#[test]
fn restore_schema_fingerprint_mismatch_returns_schema_mismatch_status() {
    let source = concat!(
        ":: start default\n",
        "> save_prompt@91000000000000000001\n",
        "  Save here.\n",
        "  ? continue@91000000000000000002\n",
        "    Continue.\n",
        "    -> END\n",
    );
    let first_bytes = compile_to_bytes_with_schema(
        source,
        recite_core::SchemaFingerprint::Fingerprint(recite_core::canonical_source_fingerprint(
            "schema-a",
        )),
    );
    let second_bytes = compile_to_bytes_with_schema(
        source,
        recite_core::SchemaFingerprint::Fingerprint(recite_core::canonical_source_fingerprint(
            "schema-b",
        )),
    );

    let mut first_asset = 0;
    let mut second_asset = 0;
    assert_eq!(
        unsafe {
            recite_asset_load(
                first_bytes.as_ptr(),
                first_bytes.len(),
                &raw mut first_asset,
            )
        },
        ReciteStatus::Ok
    );
    assert_eq!(
        unsafe {
            recite_asset_load(
                second_bytes.as_ptr(),
                second_bytes.len(),
                &raw mut second_asset,
            )
        },
        ReciteStatus::Ok
    );

    let mut session = 0;
    let mut batch = ReciteBuffer::null();
    assert_eq!(
        unsafe {
            recite_session_start(
                first_asset,
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

    let mut restored_session = 0;
    let mut restored_batch = ReciteBuffer::null();
    let status = unsafe {
        recite_session_restore(
            second_asset,
            snapshot.data,
            snapshot.len,
            &raw mut restored_session,
            &raw mut restored_batch,
        )
    };
    assert_eq!(status, ReciteStatus::SchemaMismatch);
    assert_eq!(restored_session, 0);
    let message = recite_last_error_message();
    assert!(!message.is_null());
    let message = unsafe { std::ffi::CStr::from_ptr(message) }.to_string_lossy();
    assert!(message.contains("schema fingerprint"), "{message}");

    unsafe {
        recite_buffer_free(&raw mut snapshot);
        recite_buffer_free(&raw mut restored_batch);
    }
    recite_asset_free(first_asset);
    recite_asset_free(second_asset);
}

#[test]
fn restore_non_schema_asset_content_mismatch_returns_save_load_status() {
    let first_bytes = compile_to_bytes_with_schema(
        concat!(
            ":: start default\n",
            "> line@92000000000000000001\n",
            "  First.\n",
            "-> END\n",
        ),
        recite_core::SchemaFingerprint::NoSchema,
    );
    let second_bytes = compile_to_bytes_with_schema(
        concat!(
            ":: start default\n",
            "> line@92000000000000000001\n",
            "  Changed.\n",
            "-> END\n",
        ),
        recite_core::SchemaFingerprint::NoSchema,
    );
    let mut first_asset = 0;
    let mut second_asset = 0;
    assert_eq!(
        unsafe {
            recite_asset_load(
                first_bytes.as_ptr(),
                first_bytes.len(),
                &raw mut first_asset,
            )
        },
        ReciteStatus::Ok
    );
    assert_eq!(
        unsafe {
            recite_asset_load(
                second_bytes.as_ptr(),
                second_bytes.len(),
                &raw mut second_asset,
            )
        },
        ReciteStatus::Ok
    );
    let mut session = 0;
    let mut batch = ReciteBuffer::null();
    assert_eq!(
        unsafe {
            recite_session_create(
                first_asset,
                std::ptr::null(),
                std::ptr::null(),
                &raw mut session,
            )
        },
        ReciteStatus::Ok
    );
    // There is no condition in this asset, but begin establishes a valid session state.
    assert_eq!(
        unsafe { recite_session_begin(session, &raw mut batch) },
        ReciteStatus::Ok
    );
    unsafe { recite_buffer_free(&raw mut batch) };
    let mut snapshot = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_session_snapshot(session, &raw mut snapshot) },
        ReciteStatus::Ok
    );
    recite_session_free(session);
    let mut restored_session = 0;
    let mut restored_batch = ReciteBuffer::null();
    assert_eq!(
        unsafe {
            recite_session_restore(
                second_asset,
                snapshot.data,
                snapshot.len,
                &raw mut restored_session,
                &raw mut restored_batch,
            )
        },
        ReciteStatus::SaveLoadIncompatibility
    );
    unsafe {
        recite_buffer_free(&raw mut snapshot);
        recite_buffer_free(&raw mut restored_batch);
    }
    recite_asset_free(first_asset);
    recite_asset_free(second_asset);
}
