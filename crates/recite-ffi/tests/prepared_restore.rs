mod support;

use std::ffi::c_void;

use recite_ffi::{
    ReciteBuffer, ReciteConditionQuery, ReciteConditionResult, ReciteStatus, recite_asset_free,
    recite_asset_load, recite_buffer_free, recite_session_begin, recite_session_create,
    recite_session_free, recite_session_prepare_restore, recite_session_register_condition,
    recite_session_restore, recite_session_snapshot, recite_session_start,
};
use support::{compile_to_bytes, cstr, decode_batch, event_kinds};

#[derive(Default)]
struct ResultStorage {
    bytes: Vec<u8>,
    calls: usize,
}

unsafe extern "C" fn alternating_condition(
    _query: *const ReciteConditionQuery,
    userdata: *mut c_void,
) -> ReciteConditionResult {
    // Session-owned storage survives callback return. Replacing the preceding
    // result at the next invocation is permitted by the condition contract.
    let storage = unsafe { &mut *userdata.cast::<ResultStorage>() };
    storage.calls += 1;
    storage.bytes.clear();
    storage
        .bytes
        .extend_from_slice(b"\x82\xa4kind\xa4bool\xa5value");
    storage
        .bytes
        .push(if storage.calls == 1 { 0xc3 } else { 0xc2 });
    ReciteConditionResult {
        ok: 1,
        value_msgpack: storage.bytes.as_ptr(),
        value_len: storage.bytes.len(),
        error_message: std::ptr::null(),
    }
}

#[test]
fn restore_prepares_before_conditions_and_failed_begin_can_be_retried() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        ":if ready()\n",
        "  > first@aa000000000000000001\n    First yes.\n",
        ":else\n",
        "  > first_no@aa000000000000000002\n    First no.\n",
        ":if ready()\n",
        "  > second@aa000000000000000003\n    Second yes.\n",
        ":else\n",
        "  > second_no@aa000000000000000004\n    Second no.\n",
        "-> END\n",
    ));
    let mut asset = 0;
    let mut original = 0;
    let mut snapshot = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) },
        ReciteStatus::Ok
    );
    assert_eq!(
        unsafe {
            recite_session_create(asset, std::ptr::null(), std::ptr::null(), &raw mut original)
        },
        ReciteStatus::Ok
    );
    assert_eq!(
        unsafe { recite_session_snapshot(original, &raw mut snapshot) },
        ReciteStatus::Ok
    );
    recite_session_free(original);

    let mut restored = 0;
    assert_eq!(
        unsafe {
            recite_session_prepare_restore(asset, snapshot.data, snapshot.len, &raw mut restored)
        },
        ReciteStatus::Ok
    );
    let mut batch = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_session_begin(restored, &raw mut batch) },
        ReciteStatus::MissingConditionHandler,
        "preparation succeeds before host conditions exist"
    );
    let mut after_failure = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_session_snapshot(restored, &raw mut after_failure) },
        ReciteStatus::Ok
    );
    assert_eq!(
        unsafe { std::slice::from_raw_parts(snapshot.data, snapshot.len) },
        unsafe { std::slice::from_raw_parts(after_failure.data, after_failure.len) },
        "a failed begin must preserve the checkpoint"
    );

    let mut storage = ResultStorage::default();
    let name = cstr("ready");
    assert_eq!(
        unsafe {
            recite_session_register_condition(
                restored,
                name.as_ptr(),
                Some(alternating_condition),
                (&raw mut storage).cast(),
            )
        },
        ReciteStatus::Ok
    );
    assert_eq!(
        unsafe { recite_session_begin(restored, &raw mut batch) },
        ReciteStatus::Ok
    );
    let value = decode_batch(&batch);
    assert_eq!(event_kinds(&value), ["line", "line", "end"]);
    assert_eq!(value["events"][0]["text"], "First yes.");
    assert_eq!(value["events"][1]["text"], "Second no.");
    assert_eq!(storage.calls, 2);
    assert_eq!(
        unsafe { recite_session_begin(restored, &raw mut after_failure) },
        ReciteStatus::SessionAlreadyActive
    );
    unsafe {
        recite_buffer_free(&raw mut snapshot);
        recite_buffer_free(&raw mut after_failure);
        recite_buffer_free(&raw mut batch);
    }
    recite_session_free(restored);
    recite_asset_free(asset);
}

#[test]
fn failed_restore_does_not_publish_a_handle() {
    let bytes = compile_to_bytes(":: start default\n-> END\n");
    let mut asset = 0;
    assert_eq!(
        unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) },
        ReciteStatus::Ok
    );
    let mut output = 42;
    assert_eq!(
        unsafe { recite_session_prepare_restore(asset, std::ptr::null(), 0, &raw mut output) },
        ReciteStatus::Validation
    );
    assert_eq!(output, 42);
    assert_eq!(
        unsafe {
            recite_session_prepare_restore(asset, bytes.as_ptr(), bytes.len(), &raw mut output)
        },
        ReciteStatus::SaveLoadIncompatibility
    );
    assert_eq!(output, 42);
    let mut session = 0;
    let mut batch = ReciteBuffer::null();
    let mut ended = ReciteBuffer::null();
    assert_eq!(
        unsafe {
            recite_session_create(asset, std::ptr::null(), std::ptr::null(), &raw mut session)
        },
        ReciteStatus::Ok
    );
    assert_eq!(
        unsafe { recite_session_begin(session, &raw mut batch) },
        ReciteStatus::Ok
    );
    assert_eq!(
        unsafe { recite_session_snapshot(session, &raw mut ended) },
        ReciteStatus::Ok
    );
    assert_eq!(
        unsafe { recite_session_prepare_restore(asset, ended.data, ended.len, &raw mut output) },
        ReciteStatus::NoActiveSession
    );
    assert_eq!(
        output, 42,
        "ended checkpoints must not occupy a session owner"
    );
    unsafe {
        recite_buffer_free(&raw mut ended);
        recite_buffer_free(&raw mut batch);
    }
    recite_session_free(session);
    recite_asset_free(asset);
}

#[test]
fn convenience_begin_failure_preserves_caller_outputs_and_the_original_session() {
    let bytes = compile_to_bytes(":: start default\n:if ready()\n  -> END\n:else\n  -> END\n");
    let mut asset = 0;
    let mut original = 0;
    let mut snapshot = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) },
        ReciteStatus::Ok
    );
    assert_eq!(
        unsafe {
            recite_session_create(asset, std::ptr::null(), std::ptr::null(), &raw mut original)
        },
        ReciteStatus::Ok
    );
    assert_eq!(
        unsafe { recite_session_snapshot(original, &raw mut snapshot) },
        ReciteStatus::Ok
    );
    let mut handle = original;
    let mut batch = ReciteBuffer {
        data: snapshot.data,
        len: snapshot.len,
    };
    assert_eq!(
        unsafe {
            recite_session_start(
                asset,
                std::ptr::null(),
                std::ptr::null(),
                &raw mut handle,
                &raw mut batch,
            )
        },
        ReciteStatus::MissingConditionHandler
    );
    assert_eq!(handle, original);
    assert_eq!((batch.data, batch.len), (snapshot.data, snapshot.len));
    assert_eq!(
        unsafe {
            recite_session_restore(
                asset,
                snapshot.data,
                snapshot.len,
                &raw mut handle,
                &raw mut batch,
            )
        },
        ReciteStatus::MissingConditionHandler
    );
    assert_eq!(handle, original);
    assert_eq!((batch.data, batch.len), (snapshot.data, snapshot.len));
    let mut unchanged = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_session_snapshot(original, &raw mut unchanged) },
        ReciteStatus::Ok,
        "a failed convenience call must not free an existing caller handle"
    );
    assert_eq!(
        unsafe { std::slice::from_raw_parts(snapshot.data, snapshot.len) },
        unsafe { std::slice::from_raw_parts(unchanged.data, unchanged.len) }
    );
    unsafe {
        recite_buffer_free(&raw mut snapshot);
        recite_buffer_free(&raw mut unchanged);
    }
    recite_session_free(original);
    recite_asset_free(asset);
}
