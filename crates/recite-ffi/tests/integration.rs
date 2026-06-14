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
    recite_session_acknowledge_effect, recite_session_choose, recite_session_free,
    recite_session_register_condition, recite_session_restore, recite_session_snapshot,
    recite_session_start,
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
    unsafe extern "C" fn flag_false(
        _query: *const ReciteConditionQuery,
        _userdata: *mut std::ffi::c_void,
    ) -> ReciteConditionResult {
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
        let boxed = bytes.into_boxed_slice();
        let len = boxed.len();
        let ptr = Box::into_raw(boxed) as *const u8;
        ReciteConditionResult {
            ok: 1,
            value_msgpack: ptr,
            value_len: len,
            error_message: std::ptr::null(),
        }
    }

    let name = cstr("flag");
    unsafe {
        recite_session_register_condition(session, name.as_ptr(), flag_false, std::ptr::null_mut())
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
