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
