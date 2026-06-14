//! Integration tests for the recite-ffi C ABI surface.
//!
//! Calls the extern "C" functions directly as unsafe Rust to verify the handle
//! model, drain behaviour, snapshot round-trip, and condition callback protocol.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::ffi::CString;

use recite_compiler::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId};
use recite_ffi::{
    ReciteBuffer, ReciteConditionQuery, ReciteConditionResult, ReciteStatus, recite_asset_free,
    recite_asset_load, recite_buffer_free, recite_last_error_message,
    recite_session_acknowledge_effect, recite_session_begin, recite_session_choose,
    recite_session_create, recite_session_free, recite_session_register_condition,
    recite_session_restore, recite_session_snapshot, recite_session_start,
};

fn compile_to_bytes(source: &str) -> Vec<u8> {
    let report = compile_inputs(
        [CompileInput::new("test.recite", source)],
        CompileOptions::new(
            CompilerVersion::new("0.0.1").unwrap(),
            CompiledAssetId::new("test/main.recitec").unwrap(),
            SourceMapId::new("test/main.recitec.map").unwrap(),
            SchemaFingerprint::NoSchema,
        ),
    )
    .expect("compile does not hard fail");
    assert!(
        report.diagnostics.is_empty(),
        "test source should compile cleanly: {:?}",
        report.diagnostics
    );
    report.asset.expect("compiler emits an asset").messagepack
}

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

fn decode_batch(buf: &ReciteBuffer) -> serde_json::Value {
    let bytes = unsafe { std::slice::from_raw_parts(buf.data, buf.len) };
    rmp_serde::from_slice(bytes).expect("valid msgpack batch")
}

fn event_kinds(batch: &serde_json::Value) -> Vec<&str> {
    batch["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["kind"].as_str().unwrap())
        .collect()
}

// ---------------------------------------------------------------------------

#[test]
fn asset_load_and_free_round_trip() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        "> hello@10000000000000000001\n",
        "  Hello.\n",
        "-> END\n",
    ));
    let mut handle: u64 = 0;
    let status = unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut handle) };
    assert_eq!(status, ReciteStatus::Ok);
    assert_ne!(handle, 0);
    recite_asset_free(handle);
}

#[test]
fn status_codes_match_c_abi_design() {
    let cases = [
        (ReciteStatus::Ok, 0),
        (ReciteStatus::Validation, -1),
        (ReciteStatus::AssetLoadOrDecode, -2),
        (ReciteStatus::StaleOrIncompatible, -3),
        (ReciteStatus::SchemaMismatch, -4),
        (ReciteStatus::NoActiveSession, -5),
        (ReciteStatus::SessionAlreadyActive, -6),
        (ReciteStatus::UnknownStartBlock, -7),
        (ReciteStatus::InvalidChoice, -8),
        (ReciteStatus::UnavailableChoice, -9),
        (ReciteStatus::StaleChoice, -10),
        (ReciteStatus::MissingConditionHandler, -11),
        (ReciteStatus::ConditionEvaluation, -12),
        (ReciteStatus::InvalidConditionResult, -13),
        (ReciteStatus::EffectAcknowledgement, -14),
        (ReciteStatus::RejectedRefresh, -15),
        (ReciteStatus::SaveLoadIncompatibility, -16),
        (ReciteStatus::Localisation, -17),
        (ReciteStatus::MissingProjectionHandler, -18),
        (ReciteStatus::ProjectionEvaluation, -19),
        (ReciteStatus::InvalidProjectionResult, -20),
        (ReciteStatus::InvalidHandle, -21),
        (ReciteStatus::DialogueFault, -22),
    ];

    for (status, code) in cases {
        assert_eq!(status as i32, code);
        assert_eq!(ReciteStatus::try_from(code), Ok(status));
    }
    assert_eq!(ReciteStatus::try_from(-23), Err(()));
}

#[test]
fn invalid_bytes_returns_asset_load_error() {
    let garbage = b"not a compiled asset";
    let mut handle: u64 = 0;
    let status = unsafe { recite_asset_load(garbage.as_ptr(), garbage.len(), &raw mut handle) };
    assert_eq!(status, ReciteStatus::AssetLoadOrDecode);
    assert_eq!(handle, 0);
    let msg = unsafe { std::ffi::CStr::from_ptr(recite_last_error_message()) }.to_string_lossy();
    assert!(!msg.is_empty(), "error message should be set");
}

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

#[test]
fn invalid_handle_returns_error() {
    let mut session: u64 = 0;
    let mut batch = ReciteBuffer::null();
    let status = unsafe {
        recite_session_start(
            9999,
            std::ptr::null(),
            std::ptr::null(),
            &raw mut session,
            &raw mut batch,
        )
    };
    assert_eq!(status, ReciteStatus::InvalidHandle);
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

// ---------------------------------------------------------------------------
// Additional tests from reviewer findings
// ---------------------------------------------------------------------------

/// [finding 8] Condition with typed argument: verify args_msgpack decodes with
/// the expected kind and value.
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

/// [finding 9] Snapshot of an ended session cannot be restored: the restored
/// session is already ended so `drain_to_batch` returns `NoActiveSession`.
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

/// [finding 10] Freeing an unknown session handle does not panic.
#[test]
fn session_free_unknown_handle_is_noop() {
    recite_session_free(0xDEADBEEF_DEADBEEF);
}

/// [finding 2] recite_session_create + recite_session_begin: register a
/// condition before begin so it is available for the opening drain.
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

/// [finding 2] Double-begin on the same handle returns SessionAlreadyActive.
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
