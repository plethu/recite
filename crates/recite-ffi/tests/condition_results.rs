#[path = "support/mod.rs"]
mod support;

use std::ffi::c_void;

use recite_ffi::{
    ReciteBuffer, ReciteConditionQuery, ReciteConditionResult, ReciteStatus, recite_asset_free,
    recite_asset_load, recite_buffer_free, recite_session_begin, recite_session_create,
    recite_session_free, recite_session_register_condition,
};
use support::compile_to_bytes;

const BOOL: &[u8] = b"\x82\xa4kind\xa4bool\xa5value\xc3";

#[test]
fn condition_results_require_complete_named_maps_without_extra_or_duplicate_fields() {
    let cases: &[(&str, &[u8], ReciteStatus)] = &[
        ("boolean", BOOL, ReciteStatus::Ok),
        (
            "reversed fields",
            b"\x82\xa5value\xc3\xa4kind\xa4bool",
            ReciteStatus::Ok,
        ),
        (
            "wrong result kind",
            b"\x82\xa4kind\xa4enum\xa7variant\xa5angry",
            ReciteStatus::InvalidConditionResult,
        ),
        (
            "unknown kind",
            b"\x82\xa4kind\xa3bad\xa5value\xc3",
            ReciteStatus::InvalidConditionResult,
        ),
        (
            "trailing bytes",
            b"\x82\xa4kind\xa4bool\xa5value\xc3\x00",
            ReciteStatus::InvalidConditionResult,
        ),
        (
            "duplicate value",
            b"\x83\xa4kind\xa4bool\xa5value\xc3\xa5value\xc2",
            ReciteStatus::InvalidConditionResult,
        ),
        (
            "duplicate kind",
            b"\x83\xa4kind\xa4bool\xa4kind\xa4bool\xa5value\xc3",
            ReciteStatus::InvalidConditionResult,
        ),
        (
            "unknown field",
            b"\x82\xa4kind\xa4bool\xa3var\xc3",
            ReciteStatus::InvalidConditionResult,
        ),
        (
            "missing kind",
            b"\x81\xa5value\xc3",
            ReciteStatus::InvalidConditionResult,
        ),
        (
            "missing value",
            b"\x81\xa4kind\xa4bool",
            ReciteStatus::InvalidConditionResult,
        ),
        (
            "wrong value type",
            b"\x82\xa4kind\xa4bool\xa5value\xa1x",
            ReciteStatus::InvalidConditionResult,
        ),
        (
            "positional record",
            b"\x92\xa4bool\xc3",
            ReciteStatus::InvalidConditionResult,
        ),
        (
            "extra variant",
            b"\x83\xa4kind\xa4bool\xa5value\xc3\xa7variant\xa1x",
            ReciteStatus::InvalidConditionResult,
        ),
        (
            "malformed bytes",
            b"not msgpack",
            ReciteStatus::InvalidConditionResult,
        ),
    ];
    for &(label, bytes, expected) in cases {
        assert_eq!(
            run_result(ResultInput {
                ok: 1,
                bytes: Some(bytes)
            }),
            expected,
            "{label}"
        );
    }
}

#[test]
fn condition_results_require_exact_success_flag_and_non_null_payload() {
    assert_eq!(
        run_result(ResultInput { ok: 1, bytes: None }),
        ReciteStatus::InvalidConditionResult
    );
    assert_eq!(
        run_result(ResultInput { ok: 0, bytes: None }),
        ReciteStatus::ConditionEvaluation
    );
    assert_eq!(
        run_result(ResultInput {
            ok: 2,
            bytes: Some(BOOL)
        }),
        ReciteStatus::InvalidConditionResult
    );
}

#[test]
fn enum_results_accept_named_variants_in_either_field_order() {
    let source = concat!(
        ":: start default\n:match ready()\n  :case angry\n",
        "    > matched@58000000000000000001\n      Matched.\n",
        "  :case _\n    -> END\n-> END\n",
    );
    for bytes in [
        b"\x82\xa4kind\xa4enum\xa7variant\xa5angry".as_slice(),
        b"\x82\xa7variant\xa5angry\xa4kind\xa4enum".as_slice(),
    ] {
        assert_eq!(
            run_scene(
                source,
                ResultInput {
                    ok: 1,
                    bytes: Some(bytes)
                }
            ),
            ReciteStatus::Ok
        );
    }
}

struct ResultInput<'a> {
    ok: u8,
    bytes: Option<&'a [u8]>,
}

unsafe extern "C" fn handler(
    _: *const ReciteConditionQuery,
    userdata: *mut c_void,
) -> ReciteConditionResult {
    // SAFETY: run_result owns the input and its borrowed bytes until begin
    // returns, after which it frees the session before releasing that storage.
    let input = unsafe { &*userdata.cast::<ResultInput<'_>>() };
    let (value_msgpack, value_len) = input
        .bytes
        .map_or((std::ptr::null(), 0), |bytes| (bytes.as_ptr(), bytes.len()));
    ReciteConditionResult {
        ok: input.ok,
        value_msgpack,
        value_len,
        error_message: std::ptr::null(),
    }
}

fn run_result(input: ResultInput<'_>) -> ReciteStatus {
    run_scene(
        concat!(
            ":: start default\n:if ready()\n",
            "  > yes@58000000000000000001\n    Ready.\n",
            ":else\n  > no@58000000000000000002\n    Not ready.\n-> END\n",
        ),
        input,
    )
}

fn run_scene(source: &str, mut input: ResultInput<'_>) -> ReciteStatus {
    let bytes = compile_to_bytes(source);
    let mut asset = 0;
    assert_eq!(
        unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) },
        ReciteStatus::Ok
    );
    let mut session = 0;
    assert_eq!(
        unsafe {
            recite_session_create(asset, std::ptr::null(), std::ptr::null(), &raw mut session)
        },
        ReciteStatus::Ok
    );
    assert_eq!(
        unsafe {
            recite_session_register_condition(
                session,
                c"ready".as_ptr(),
                Some(handler),
                (&raw mut input).cast(),
            )
        },
        ReciteStatus::Ok
    );
    let mut batch = ReciteBuffer::null();
    let status = unsafe { recite_session_begin(session, &raw mut batch) };
    unsafe {
        recite_buffer_free(&raw mut batch);
    }
    recite_session_free(session);
    recite_asset_free(asset);
    status
}
