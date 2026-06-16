#[path = "support/mod.rs"]
mod support;

use support::*;

#[test]
fn session_start_drains_lines_to_end() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        "> line1@20000000000000000001\n",
        "  Line one.\n",
        "> line2@20000000000000000002\n",
        "  Line two.\n",
        "-> END\n",
    ));
    let mut asset: u64 = 0;
    unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) };

    let mut session: u64 = 0;
    let mut batch = ReciteBuffer::null();
    let status = unsafe {
        recite_session_start(
            asset,
            std::ptr::null(),
            std::ptr::null(),
            &raw mut session,
            &raw mut batch,
        )
    };
    assert_eq!(status, ReciteStatus::Ok);
    assert_ne!(session, 0);

    let value = decode_batch(&batch);
    assert_eq!(
        event_kinds(&value),
        ["line", "line", "end"],
        "batch: {:?}",
        value
    );

    unsafe { recite_buffer_free(&raw mut batch) };
    recite_session_free(session);
    recite_asset_free(asset);
}
#[test]
fn session_start_stops_at_prompt() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        "> intro@30000000000000000001\n",
        "  Choose.\n",
        "> choice_prompt@30000000000000000002\n",
        "  Pick:\n",
        "  ? a@30000000000000000003\n",
        "    A.\n",
        "    -> a_block\n",
        "  ? b@30000000000000000004\n",
        "    B.\n",
        "    -> b_block\n",
        ":: a_block\n",
        "> a_line@30000000000000000005\n",
        "  Chose A.\n",
        "-> END\n",
        ":: b_block\n",
        "> b_line@30000000000000000006\n",
        "  Chose B.\n",
        "-> END\n",
    ));
    let mut asset: u64 = 0;
    unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) };

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
    assert_eq!(event_kinds(&value), ["line", "prompt"], "{:?}", value);

    let choices = value["events"][1]["choices"].as_array().unwrap();
    assert_eq!(choices.len(), 2);
    let choice_id = choices[0]["id"].as_str().unwrap().to_owned();
    unsafe { recite_buffer_free(&raw mut batch) };

    let choice_cstr = cstr(&choice_id);
    let mut batch2 = ReciteBuffer::null();
    let status = unsafe { recite_session_choose(session, choice_cstr.as_ptr(), &raw mut batch2) };
    assert_eq!(status, ReciteStatus::Ok, "choose succeeds");

    let value2 = decode_batch(&batch2);
    let kinds2 = event_kinds(&value2);
    assert!(
        kinds2.contains(&"line"),
        "post-choice batch contains a line: {:?}",
        kinds2
    );

    unsafe { recite_buffer_free(&raw mut batch2) };
    recite_session_free(session);
    recite_asset_free(asset);
}

#[test]
fn start_choose_snapshot_restore_choose_end_lifecycle() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        "> first_prompt@4a000000000000000001\n",
        "  First choice.\n",
        "  ? left@4a000000000000000002\n",
        "    Left.\n",
        "    -> second\n",
        ":: second\n",
        "> second_prompt@4a000000000000000003\n",
        "  Second choice.\n",
        "  ? finish@4a000000000000000004\n",
        "    Finish.\n",
        "    -> done\n",
        ":: done\n",
        "> done_line@4a000000000000000005\n",
        "  Done.\n",
        "-> END\n",
    ));
    let mut asset: u64 = 0;
    unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) };

    let mut session1: u64 = 0;
    let mut batch1 = ReciteBuffer::null();
    let start_status = unsafe {
        recite_session_start(
            asset,
            std::ptr::null(),
            std::ptr::null(),
            &raw mut session1,
            &raw mut batch1,
        )
    };
    assert_eq!(start_status, ReciteStatus::Ok);
    assert_eq!(event_kinds(&decode_batch(&batch1)), ["prompt"]);
    unsafe { recite_buffer_free(&raw mut batch1) };

    let first_choice = cstr("4a000000000000000002");
    let mut batch2 = ReciteBuffer::null();
    let choose_status =
        unsafe { recite_session_choose(session1, first_choice.as_ptr(), &raw mut batch2) };
    assert_eq!(choose_status, ReciteStatus::Ok);
    assert_eq!(event_kinds(&decode_batch(&batch2)), ["prompt"]);
    unsafe { recite_buffer_free(&raw mut batch2) };

    let mut snapshot = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_session_snapshot(session1, &raw mut snapshot) },
        ReciteStatus::Ok
    );
    recite_session_free(session1);

    let mut session2: u64 = 0;
    let mut restore_batch = ReciteBuffer::null();
    let restore_status = unsafe {
        recite_session_restore(
            asset,
            snapshot.data,
            snapshot.len,
            &raw mut session2,
            &raw mut restore_batch,
        )
    };
    assert_eq!(restore_status, ReciteStatus::Ok);
    assert_eq!(
        event_kinds(&decode_batch(&restore_batch)),
        Vec::<&str>::new()
    );
    unsafe { recite_buffer_free(&raw mut restore_batch) };

    let second_choice = cstr("4a000000000000000004");
    let mut final_batch = ReciteBuffer::null();
    let final_choose_status =
        unsafe { recite_session_choose(session2, second_choice.as_ptr(), &raw mut final_batch) };
    assert_eq!(final_choose_status, ReciteStatus::Ok);
    assert_eq!(event_kinds(&decode_batch(&final_batch)), ["line", "end"]);

    unsafe { recite_buffer_free(&raw mut snapshot) };
    unsafe { recite_buffer_free(&raw mut final_batch) };
    recite_session_free(session2);
    recite_asset_free(asset);
}

#[test]
fn session_create_register_begin_sequence() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        ":if ready()\n",
        "  > yes@aa000000000000000001\n",
        "    Ready.\n",
        ":else\n",
        "  > no@aa000000000000000002\n",
        "    Not ready.\n",
        "-> END\n",
    ));
    let mut asset: u64 = 0;
    unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) };

    // Create the session without running any traversal.
    let mut session: u64 = 0;
    let create_status = unsafe {
        recite_session_create(asset, std::ptr::null(), std::ptr::null(), &raw mut session)
    };
    assert_eq!(create_status, ReciteStatus::Ok);
    assert_ne!(session, 0);

    // Register the condition handler before begin.
    let mut result_buf = [0u8; 32];
    unsafe extern "C" fn ready_true(
        _query: *const ReciteConditionQuery,
        userdata: *mut std::ffi::c_void,
    ) -> ReciteConditionResult {
        let buf = unsafe { &mut *(userdata as *mut [u8; 32]) };
        #[derive(serde::Serialize)]
        struct R {
            kind: &'static str,
            value: bool,
        }
        let bytes = rmp_serde::to_vec_named(&R {
            kind: "bool",
            value: true,
        })
        .unwrap_or_default();
        let len = bytes.len().min(buf.len());
        buf[..len].copy_from_slice(&bytes[..len]);
        ReciteConditionResult {
            ok: 1,
            value_msgpack: buf.as_ptr(),
            value_len: len,
            error_message: std::ptr::null(),
        }
    }
    let name = cstr("ready");
    unsafe {
        recite_session_register_condition(
            session,
            name.as_ptr(),
            ready_true,
            result_buf.as_mut_ptr().cast(),
        )
    };

    // Begin the session — the condition is now available for the opening drain.
    let mut batch = ReciteBuffer::null();
    let begin_status = unsafe { recite_session_begin(session, &raw mut batch) };
    assert_eq!(begin_status, ReciteStatus::Ok, "begin succeeds");

    let value = decode_batch(&batch);
    let kinds = event_kinds(&value);
    assert!(kinds.contains(&"line"), "opening block produces a line");
    let text = value["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == "line")
        .unwrap()["text"]
        .as_str()
        .unwrap();
    assert_eq!(
        text, "Ready.",
        "ready() returned true so we get the if branch"
    );

    unsafe { recite_buffer_free(&raw mut batch) };
    recite_session_free(session);
    recite_asset_free(asset);
}
#[test]
fn session_begin_twice_returns_already_active() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        "> line@bb000000000000000001\n",
        "  Hi.\n",
        "-> END\n",
    ));
    let mut asset: u64 = 0;
    unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) };

    let mut session: u64 = 0;
    unsafe { recite_session_create(asset, std::ptr::null(), std::ptr::null(), &raw mut session) };

    let mut batch1 = ReciteBuffer::null();
    let s1 = unsafe { recite_session_begin(session, &raw mut batch1) };
    assert_eq!(s1, ReciteStatus::Ok);
    unsafe { recite_buffer_free(&raw mut batch1) };

    let mut batch2 = ReciteBuffer::null();
    let s2 = unsafe { recite_session_begin(session, &raw mut batch2) };
    assert_eq!(
        s2,
        ReciteStatus::SessionAlreadyActive,
        "second begin returns already-active"
    );
    unsafe { recite_buffer_free(&raw mut batch2) };

    recite_session_free(session);
    recite_asset_free(asset);
}
