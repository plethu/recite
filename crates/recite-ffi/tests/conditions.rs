#[path = "support/mod.rs"]
mod support;

use support::*;

#[test]
fn missing_condition_handler_returns_error_at_start() {
    // Conditions that appear during the start drain require a registered handler.
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        ":if has_item()\n",
        "  > has@50000000000000000001\n",
        "    Has item.\n",
        ":else\n",
        "  > no@50000000000000000002\n",
        "    No item.\n",
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
    assert_eq!(
        status,
        ReciteStatus::MissingConditionHandler,
        "missing handler at start returns condition error"
    );
    assert_eq!(session, 0, "no handle assigned on error");

    unsafe { recite_buffer_free(&raw mut batch) };
    recite_asset_free(asset);
}
#[test]
fn condition_error_message_prefix_does_not_override_status() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        ":if ready()\n",
        "  > yes@51000000000000000001\n",
        "    Ready.\n",
        ":else\n",
        "  > no@51000000000000000002\n",
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

    unsafe extern "C" fn fail_with_status_like_prefix(
        _query: *const ReciteConditionQuery,
        _userdata: *mut std::ffi::c_void,
    ) -> ReciteConditionResult {
        static MESSAGE: &[u8] = b"[-11] host condition failed\0";
        ReciteConditionResult {
            ok: 0,
            value_msgpack: std::ptr::null(),
            value_len: 0,
            error_message: MESSAGE.as_ptr().cast(),
        }
    }

    let name = cstr("ready");
    let register_status = unsafe {
        recite_session_register_condition(
            session,
            name.as_ptr(),
            fail_with_status_like_prefix,
            std::ptr::null_mut(),
        )
    };
    assert_eq!(register_status, ReciteStatus::Ok);

    let mut batch = ReciteBuffer::null();
    let begin_status = unsafe { recite_session_begin(session, &raw mut batch) };
    assert_eq!(
        begin_status,
        ReciteStatus::ConditionEvaluation,
        "host message prefix must not be decoded as MissingConditionHandler"
    );

    unsafe { recite_buffer_free(&raw mut batch) };
    recite_session_free(session);
    recite_asset_free(asset);
}

#[test]
fn condition_callback_invoked_via_choose_branch() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        "> intro@70000000000000000001\n",
        "  Choose.\n",
        "> branch_prompt@70000000000000000002\n",
        "  Pick:\n",
        "  ? branch@70000000000000000003\n",
        "    Branch.\n",
        "    -> branch\n",
        ":: branch\n",
        ":if flag()\n",
        "  > flag_set@70000000000000000004\n",
        "    Flag is set.\n",
        ":else\n",
        "  > flag_clear@70000000000000000005\n",
        "    Flag is clear.\n",
        "-> END\n",
    ));
    let mut asset: u64 = 0;
    unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) };

    // Start — stops at prompt.
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
    let choices = value["events"][1]["choices"].as_array().unwrap();
    let choice_id = choices[0]["id"].as_str().unwrap().to_owned();
    unsafe { recite_buffer_free(&raw mut batch) };

    // Register condition handler returning Bool(false) → else branch.
    // Use a stack buffer instead of heap allocation to avoid a memory leak.
    unsafe extern "C" fn flag_false(
        _query: *const ReciteConditionQuery,
        userdata: *mut std::ffi::c_void,
    ) -> ReciteConditionResult {
        // userdata points to a caller-owned stack buffer; write the msgpack there.
        let buf = unsafe { &mut *(userdata as *mut [u8; 32]) };
        #[derive(serde::Serialize)]
        struct R {
            kind: &'static str,
            value: bool,
        }
        let bytes = rmp_serde::to_vec_named(&R {
            kind: "bool",
            value: false,
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

    let mut result_buf = [0u8; 32];
    let name = cstr("flag");
    unsafe {
        recite_session_register_condition(
            session,
            name.as_ptr(),
            flag_false,
            result_buf.as_mut_ptr().cast(),
        )
    };

    let id_cstr = cstr(&choice_id);
    let mut batch2 = ReciteBuffer::null();
    let status = unsafe { recite_session_choose(session, id_cstr.as_ptr(), &raw mut batch2) };
    assert_eq!(status, ReciteStatus::Ok, "choose with condition succeeds");

    let value2 = decode_batch(&batch2);
    let kinds2 = event_kinds(&value2);
    assert!(
        kinds2.contains(&"line"),
        "branch produces a line: {:?}",
        kinds2
    );

    let line_text = value2["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == "line")
        .unwrap()["text"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(line_text, "Flag is clear.");

    unsafe { recite_buffer_free(&raw mut batch2) };
    recite_session_free(session);
    recite_asset_free(asset);
}
#[test]
fn condition_args_msgpack_decoded_in_callback() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        "> intro@80000000000000000001\n",
        "  Choose.\n",
        "  ? branch@80000000000000000002\n",
        "    Branch.\n",
        "    -> branch\n",
        ":: branch\n",
        ":if has_item(sword)\n",
        "  > yes@80000000000000000003\n",
        "    Has sword.\n",
        ":else\n",
        "  > no@80000000000000000004\n",
        "    No sword.\n",
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
    let choices = value["events"][0]["choices"].as_array().unwrap();
    let choice_id = choices[0]["id"].as_str().unwrap().to_owned();
    unsafe { recite_buffer_free(&raw mut batch) };

    // Store decoded arg info in a place the callback can write to.
    struct ArgCheck {
        kind: [u8; 16],
        value: [u8; 16],
        decoded: bool,
    }
    let mut check = ArgCheck {
        kind: [0u8; 16],
        value: [0u8; 16],
        decoded: false,
    };
    unsafe extern "C" fn has_item_handler(
        query: *const ReciteConditionQuery,
        userdata: *mut std::ffi::c_void,
    ) -> ReciteConditionResult {
        let q = unsafe { &*query };
        let args_bytes = unsafe { std::slice::from_raw_parts(q.args_msgpack, q.args_len) };

        // Decode args as a JSON value for inspection.
        if let Ok(v) = rmp_serde::from_slice::<serde_json::Value>(args_bytes)
            && let Some(first) = v.as_array().and_then(|a| a.first())
        {
            let kind = first["kind"].as_str().unwrap_or("").as_bytes();
            let val = first["value"].as_str().unwrap_or("").as_bytes();

            // userdata points to ArgCheck; store kind and value into its fields.
            let check = unsafe { &mut *(userdata as *mut ArgCheck) };
            let klen = kind.len().min(check.kind.len());
            check.kind[..klen].copy_from_slice(&kind[..klen]);
            let vlen = val.len().min(check.value.len());
            check.value[..vlen].copy_from_slice(&val[..vlen]);
            check.decoded = true;
        }

        // Return Bool(true); use the result_buf at a known offset after ArgCheck.
        // We store the result bytes in the result_buf that the test owns separately.
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
        // We need a stable buffer. Use a thread_local for simplicity.
        thread_local! {
            static BUF: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
        }
        BUF.with(|b| {
            *b.borrow_mut() = bytes;
        });
        let (ptr, len) = BUF.with(|b| {
            let guard = b.borrow();
            (guard.as_ptr(), guard.len())
        });
        ReciteConditionResult {
            ok: 1,
            value_msgpack: ptr,
            value_len: len,
            error_message: std::ptr::null(),
        }
    }

    let name = cstr("has_item");
    unsafe {
        recite_session_register_condition(
            session,
            name.as_ptr(),
            has_item_handler,
            (&raw mut check).cast(),
        )
    };

    let id_cstr = cstr(&choice_id);
    let mut batch2 = ReciteBuffer::null();
    let status = unsafe { recite_session_choose(session, id_cstr.as_ptr(), &raw mut batch2) };
    assert_eq!(
        status,
        ReciteStatus::Ok,
        "choose with arg condition succeeds"
    );

    assert!(check.decoded, "callback was invoked and decoded args");
    let kind_str = std::str::from_utf8(&check.kind)
        .unwrap()
        .trim_end_matches('\0');
    let val_str = std::str::from_utf8(&check.value)
        .unwrap()
        .trim_end_matches('\0');
    assert_eq!(kind_str, "identifier", "arg kind should be identifier");
    assert_eq!(val_str, "sword", "arg value should be sword");

    unsafe { recite_buffer_free(&raw mut batch2) };
    recite_session_free(session);
    recite_asset_free(asset);
}
