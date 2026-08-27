#[path = "support/mod.rs"]
mod support;

use support::*;

#[test]
fn invalid_condition_result_returns_invalid_condition_result_status() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        ":if ready()\n",
        "  > yes@57000000000000000001\n",
        "    Ready.\n",
        ":else\n",
        "  > no@57000000000000000002\n",
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

    unsafe extern "C" fn invalid_msgpack_result(
        _query: *const ReciteConditionQuery,
        _userdata: *mut std::ffi::c_void,
    ) -> ReciteConditionResult {
        static INVALID_MSGPACK: &[u8] = b"not msgpack";
        ReciteConditionResult {
            ok: 1,
            value_msgpack: INVALID_MSGPACK.as_ptr(),
            value_len: INVALID_MSGPACK.len(),
            error_message: std::ptr::null(),
        }
    }

    let name = cstr("ready");
    let register_status = unsafe {
        recite_session_register_condition(
            session,
            name.as_ptr(),
            invalid_msgpack_result,
            std::ptr::null_mut(),
        )
    };
    assert_eq!(register_status, ReciteStatus::Ok);

    let mut batch = ReciteBuffer::null();
    let begin_status = unsafe { recite_session_begin(session, &raw mut batch) };
    assert_eq!(begin_status, ReciteStatus::InvalidConditionResult);

    unsafe { recite_buffer_free(&raw mut batch) };
    recite_session_free(session);
    recite_asset_free(asset);
}

#[test]
fn condition_results_require_exact_ok_and_complete_named_maps() {
    assert_eq!(run_result_case(0), ReciteStatus::Ok);
    // The result map is valid, but this condition expects a bool and receives an enum.
    assert_eq!(run_result_case(1), ReciteStatus::InvalidConditionResult);
    assert_eq!(run_result_case(2), ReciteStatus::InvalidConditionResult);
    assert_eq!(run_result_case(3), ReciteStatus::InvalidConditionResult);
    assert_eq!(run_result_case(4), ReciteStatus::InvalidConditionResult);
    assert_eq!(run_result_case(5), ReciteStatus::InvalidConditionResult);
    assert_eq!(run_result_case(6), ReciteStatus::InvalidConditionResult);
    assert_eq!(run_result_case(7), ReciteStatus::InvalidConditionResult);
    assert_eq!(run_result_case(8), ReciteStatus::ConditionEvaluation);
    assert_eq!(run_result_case(10), ReciteStatus::Ok);

    fn run_result_case(case: u8) -> ReciteStatus {
        let bytes = compile_to_bytes(concat!(
            ":: start default\n",
            ":if ready()\n",
            "  > yes@58000000000000000001\n",
            "    Ready.\n",
            ":else\n",
            "  > no@58000000000000000002\n",
            "    Not ready.\n",
            "-> END\n",
        ));
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

        unsafe extern "C" fn handler(
            _query: *const ReciteConditionQuery,
            userdata: *mut std::ffi::c_void,
        ) -> ReciteConditionResult {
            const BOOL: &[u8] = &[
                0x82, 0xa4, b'k', b'i', b'n', b'd', 0xa4, b'b', b'o', b'o', b'l', 0xa5, b'v', b'a',
                b'l', b'u', b'e', 0xc3,
            ];
            const ENUM: &[u8] = &[
                0x82, 0xa4, b'k', b'i', b'n', b'd', 0xa4, b'e', b'n', b'u', b'm', 0xa7, b'v', b'a',
                b'r', b'i', b'a', b'n', b't', 0xa5, b'a', b'n', b'g', b'r', b'y',
            ];
            const TRAILING: &[u8] = &[
                0x82, 0xa4, b'k', b'i', b'n', b'd', 0xa4, b'b', b'o', b'o', b'l', 0xa5, b'v', b'a',
                b'l', b'u', b'e', 0xc3, 0x00,
            ];
            const DUPLICATE: &[u8] = &[
                0x83, 0xa4, b'k', b'i', b'n', b'd', 0xa4, b'b', b'o', b'o', b'l', 0xa5, b'v', b'a',
                b'l', b'u', b'e', 0xc3, 0xa5, b'v', b'a', b'l', b'u', b'e', 0xc2,
            ];
            const UNKNOWN: &[u8] = &[
                0x82, 0xa4, b'k', b'i', b'n', b'd', 0xa4, b'b', b'o', b'o', b'l', 0xa3, b'v', b'a',
                b'r', 0xc3,
            ];
            const MISSING_KIND: &[u8] = &[0x81, 0xa5, b'v', b'a', b'l', b'u', b'e', 0xc3];
            const WRONG_VALUE_TYPE: &[u8] = &[
                0x82, 0xa4, b'k', b'i', b'n', b'd', 0xa4, b'b', b'o', b'o', b'l', 0xa5, b'v', b'a',
                b'l', b'u', b'e', 0xa1, b'x',
            ];
            const REVERSED_BOOL: &[u8] = &[
                0x82, 0xa5, b'v', b'a', b'l', b'u', b'e', 0xc3, 0xa4, b'k', b'i', b'n', b'd', 0xa4,
                b'b', b'o', b'o', b'l',
            ];
            let case = unsafe { *(userdata as *const u8) };
            match case {
                0 => ReciteConditionResult {
                    ok: 1,
                    value_msgpack: BOOL.as_ptr(),
                    value_len: BOOL.len(),
                    error_message: std::ptr::null(),
                },
                1 => ReciteConditionResult {
                    ok: 1,
                    value_msgpack: ENUM.as_ptr(),
                    value_len: ENUM.len(),
                    error_message: std::ptr::null(),
                },
                2 => ReciteConditionResult {
                    ok: 1,
                    value_msgpack: TRAILING.as_ptr(),
                    value_len: TRAILING.len(),
                    error_message: std::ptr::null(),
                },
                3 => ReciteConditionResult {
                    ok: 1,
                    value_msgpack: DUPLICATE.as_ptr(),
                    value_len: DUPLICATE.len(),
                    error_message: std::ptr::null(),
                },
                4 => ReciteConditionResult {
                    ok: 1,
                    value_msgpack: UNKNOWN.as_ptr(),
                    value_len: UNKNOWN.len(),
                    error_message: std::ptr::null(),
                },
                5 => ReciteConditionResult {
                    ok: 1,
                    value_msgpack: MISSING_KIND.as_ptr(),
                    value_len: MISSING_KIND.len(),
                    error_message: std::ptr::null(),
                },
                6 => ReciteConditionResult {
                    ok: 1,
                    value_msgpack: WRONG_VALUE_TYPE.as_ptr(),
                    value_len: WRONG_VALUE_TYPE.len(),
                    error_message: std::ptr::null(),
                },
                7 => ReciteConditionResult {
                    ok: 1,
                    value_msgpack: std::ptr::null(),
                    value_len: 0,
                    error_message: std::ptr::null(),
                },
                8 => ReciteConditionResult {
                    ok: 0,
                    value_msgpack: std::ptr::null(),
                    value_len: 0,
                    error_message: std::ptr::null(),
                },
                10 => ReciteConditionResult {
                    ok: 1,
                    value_msgpack: REVERSED_BOOL.as_ptr(),
                    value_len: REVERSED_BOOL.len(),
                    error_message: std::ptr::null(),
                },
                _ => ReciteConditionResult {
                    ok: 2,
                    value_msgpack: BOOL.as_ptr(),
                    value_len: BOOL.len(),
                    error_message: std::ptr::null(),
                },
            }
        }

        let name = cstr("ready");
        let mut case = case;
        let register_status = unsafe {
            recite_session_register_condition(
                session,
                name.as_ptr(),
                handler,
                (&raw mut case).cast(),
            )
        };
        assert_eq!(register_status, ReciteStatus::Ok);
        let mut batch = ReciteBuffer::null();
        let status = unsafe { recite_session_begin(session, &raw mut batch) };
        unsafe { recite_buffer_free(&raw mut batch) };
        recite_session_free(session);
        recite_asset_free(asset);
        status
    }

    // `ok` values other than exactly 0 or 1 are rejected before payload decoding.
    assert_eq!(run_result_case(9), ReciteStatus::InvalidConditionResult);
}
