#[path = "support/mod.rs"]
mod support;

use support::*;

#[test]
fn session_begin_from_non_owner_thread_is_rejected() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        ":if ready()\n",
        "  > yes@52000000000000000001\n",
        "    Ready.\n",
        ":else\n",
        "  > no@52000000000000000002\n",
        "    Not ready.\n",
        "-> END\n",
    ));
    let mut asset: u64 = 0;
    unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) };

    let mut session: u64 = 0;
    let create_status = unsafe {
        recite_session_create(asset, std::ptr::null(), std::ptr::null(), &raw mut session)
    };
    assert_eq!(create_status, ReciteStatus::Ok);

    static CALLBACK_RAN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    unsafe extern "C" fn ready_true(
        _query: *const ReciteConditionQuery,
        _userdata: *mut std::ffi::c_void,
    ) -> ReciteConditionResult {
        CALLBACK_RAN.store(true, std::sync::atomic::Ordering::Relaxed);
        ReciteConditionResult {
            ok: 0,
            value_msgpack: std::ptr::null(),
            value_len: 0,
            error_message: std::ptr::null(),
        }
    }

    let name = cstr("ready");
    let register_status = unsafe {
        recite_session_register_condition(
            session,
            name.as_ptr(),
            Some(ready_true),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(register_status, ReciteStatus::Ok);

    let status = run_on_non_owner_thread(move || {
        let mut batch = ReciteBuffer::null();
        let status = unsafe { recite_session_begin(session, &raw mut batch) };
        unsafe { recite_buffer_free(&raw mut batch) };
        status
    });

    assert_eq!(status, ReciteStatus::Validation);
    assert!(
        !CALLBACK_RAN.load(std::sync::atomic::Ordering::Relaxed),
        "condition callback must not run from the wrong thread"
    );

    recite_session_free(session);
    recite_asset_free(asset);
}
#[test]
fn session_register_condition_from_non_owner_thread_is_rejected() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        "> line@53000000000000000001\n",
        "  Hi.\n",
        "-> END\n",
    ));
    let mut asset: u64 = 0;
    unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) };

    let mut session: u64 = 0;
    let create_status = unsafe {
        recite_session_create(asset, std::ptr::null(), std::ptr::null(), &raw mut session)
    };
    assert_eq!(create_status, ReciteStatus::Ok);

    unsafe extern "C" fn unused_handler(
        _query: *const ReciteConditionQuery,
        _userdata: *mut std::ffi::c_void,
    ) -> ReciteConditionResult {
        ReciteConditionResult {
            ok: 0,
            value_msgpack: std::ptr::null(),
            value_len: 0,
            error_message: std::ptr::null(),
        }
    }

    let status = run_on_non_owner_thread(move || {
        let name = cstr("ready");
        unsafe {
            recite_session_register_condition(
                session,
                name.as_ptr(),
                Some(unused_handler),
                std::ptr::null_mut(),
            )
        }
    });

    assert_eq!(status, ReciteStatus::Validation);

    recite_session_free(session);
    recite_asset_free(asset);
}
#[test]
fn session_choose_from_non_owner_thread_is_rejected() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        "> prompt@54000000000000000001\n",
        "  Pick.\n",
        "  ? go@54000000000000000002\n",
        "    Go.\n",
        "    -> after\n",
        ":: after\n",
        "> done@54000000000000000003\n",
        "  Done.\n",
        "-> END\n",
    ));
    let mut asset: u64 = 0;
    unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) };

    let mut session: u64 = 0;
    let mut batch = ReciteBuffer::null();
    let start_status = unsafe {
        recite_session_start(
            asset,
            std::ptr::null(),
            std::ptr::null(),
            &raw mut session,
            &raw mut batch,
        )
    };
    assert_eq!(start_status, ReciteStatus::Ok);
    let value = decode_batch(&batch);
    let choice_id = value["events"][0]["choices"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    unsafe { recite_buffer_free(&raw mut batch) };

    let status = run_on_non_owner_thread(move || {
        let choice = cstr(&choice_id);
        let mut batch = ReciteBuffer::null();
        let status = unsafe { recite_session_choose(session, choice.as_ptr(), &raw mut batch) };
        unsafe { recite_buffer_free(&raw mut batch) };
        status
    });

    assert_eq!(status, ReciteStatus::Validation);

    recite_session_free(session);
    recite_asset_free(asset);
}
#[test]
fn session_acknowledge_effect_from_non_owner_thread_is_rejected() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        "! blocking play_sound(chime)\n",
        "> after@55000000000000000001\n",
        "  After.\n",
        "-> END\n",
    ));
    let mut asset: u64 = 0;
    unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) };

    let mut session: u64 = 0;
    let mut batch = ReciteBuffer::null();
    let start_status = unsafe {
        recite_session_start(
            asset,
            std::ptr::null(),
            std::ptr::null(),
            &raw mut session,
            &raw mut batch,
        )
    };
    assert_eq!(start_status, ReciteStatus::Ok);
    let value = decode_batch(&batch);
    let effect_id = value["events"][0]["id"].as_str().unwrap().to_owned();
    unsafe { recite_buffer_free(&raw mut batch) };

    let status = run_on_non_owner_thread(move || {
        let effect = cstr(&effect_id);
        let mut batch = ReciteBuffer::null();
        let status = unsafe {
            recite_session_acknowledge_effect(
                session,
                effect.as_ptr(),
                1,
                std::ptr::null(),
                &raw mut batch,
            )
        };
        unsafe { recite_buffer_free(&raw mut batch) };
        status
    });

    assert_eq!(status, ReciteStatus::Validation);

    recite_session_free(session);
    recite_asset_free(asset);
}
#[test]
fn session_snapshot_from_non_owner_thread_is_rejected() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        "> prompt@56000000000000000001\n",
        "  Pick.\n",
        "  ? go@56000000000000000002\n",
        "    Go.\n",
        "    -> after\n",
        ":: after\n",
        "> done@56000000000000000003\n",
        "  Done.\n",
        "-> END\n",
    ));
    let mut asset: u64 = 0;
    unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) };

    let mut session: u64 = 0;
    let mut batch = ReciteBuffer::null();
    let start_status = unsafe {
        recite_session_start(
            asset,
            std::ptr::null(),
            std::ptr::null(),
            &raw mut session,
            &raw mut batch,
        )
    };
    assert_eq!(start_status, ReciteStatus::Ok);
    unsafe { recite_buffer_free(&raw mut batch) };

    let status = run_on_non_owner_thread(move || {
        let mut snapshot = ReciteBuffer::null();
        let status = unsafe { recite_session_snapshot(session, &raw mut snapshot) };
        unsafe { recite_buffer_free(&raw mut snapshot) };
        status
    });

    assert_eq!(status, ReciteStatus::Validation);

    recite_session_free(session);
    recite_asset_free(asset);
}
