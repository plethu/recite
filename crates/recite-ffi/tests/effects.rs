#[path = "support/mod.rs"]
mod support;

use support::*;

#[test]
fn blocking_effect_acknowledge() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        "> before@60000000000000000001\n",
        "  Before.\n",
        "! blocking play_sound(chime)\n",
        "> after@60000000000000000002\n",
        "  After.\n",
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
    let events = value["events"].as_array().unwrap();
    let effect_event = events
        .iter()
        .find(|e| e["kind"] == "effect")
        .expect("batch has a blocking effect");
    assert_eq!(effect_event["mode"], "blocking");
    let effect_id = effect_event["id"].as_str().unwrap().to_owned();
    unsafe { recite_buffer_free(&raw mut batch) };

    let id_cstr = cstr(&effect_id);
    let mut batch2 = ReciteBuffer::null();
    let status = unsafe {
        recite_session_acknowledge_effect(
            session,
            id_cstr.as_ptr(),
            1,
            std::ptr::null(),
            &raw mut batch2,
        )
    };
    assert_eq!(status, ReciteStatus::Ok, "acknowledge succeeds");

    let value2 = decode_batch(&batch2);
    let kinds2 = event_kinds(&value2);
    assert!(
        kinds2.contains(&"line"),
        "post-ack batch has a line: {:?}",
        kinds2
    );

    unsafe { recite_buffer_free(&raw mut batch2) };
    recite_session_free(session);
    recite_asset_free(asset);
}
#[test]
fn effect_acknowledge_rejects_invalid_utf8_failure_reason() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        "! blocking play_sound(chime)\n",
        "> after@6a000000000000000001\n",
        "  After.\n",
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
    let effect_id = value["events"][0]["id"].as_str().unwrap().to_owned();
    unsafe { recite_buffer_free(&raw mut batch) };

    let effect_id = cstr(&effect_id);
    let invalid_reason = [0xff_u8, 0];
    let mut invalid_batch = ReciteBuffer::null();
    let invalid_status = unsafe {
        recite_session_acknowledge_effect(
            session,
            effect_id.as_ptr(),
            0,
            invalid_reason.as_ptr().cast(),
            &raw mut invalid_batch,
        )
    };
    assert_eq!(invalid_status, ReciteStatus::Validation);
    unsafe { recite_buffer_free(&raw mut invalid_batch) };

    let mut ok_batch = ReciteBuffer::null();
    let ok_status = unsafe {
        recite_session_acknowledge_effect(
            session,
            effect_id.as_ptr(),
            1,
            std::ptr::null(),
            &raw mut ok_batch,
        )
    };
    assert_eq!(
        ok_status,
        ReciteStatus::Ok,
        "invalid UTF-8 failure reason must not consume the pending effect"
    );
    assert_eq!(event_kinds(&decode_batch(&ok_batch)), ["line", "end"]);

    unsafe { recite_buffer_free(&raw mut ok_batch) };
    recite_session_free(session);
    recite_asset_free(asset);
}
