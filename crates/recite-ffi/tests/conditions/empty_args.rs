use super::*;

#[test]
fn empty_condition_args_are_encoded_as_an_empty_array() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        ":if ready()\n",
        "  > yes@82000000000000000001\n",
        "    Yes.\n",
        ":else\n",
        "  > no@82000000000000000002\n",
        "    No.\n",
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

    unsafe extern "C" fn ready(
        query: *const ReciteConditionQuery,
        _userdata: *mut std::ffi::c_void,
    ) -> ReciteConditionResult {
        let query = unsafe { &*query };
        assert_eq!(
            unsafe { std::slice::from_raw_parts(query.args_msgpack, query.args_len) },
            &[0x90]
        );
        static RESULT: &[u8] = &[
            0x82, 0xa4, b'k', b'i', b'n', b'd', 0xa4, b'b', b'o', b'o', b'l', 0xa5, b'v', b'a',
            b'l', b'u', b'e', 0xc3,
        ];
        ReciteConditionResult {
            ok: 1,
            value_msgpack: RESULT.as_ptr(),
            value_len: RESULT.len(),
            error_message: std::ptr::null(),
        }
    }

    let name = cstr("ready");
    assert_eq!(
        unsafe {
            recite_session_register_condition(
                session,
                name.as_ptr(),
                Some(ready),
                std::ptr::null_mut(),
            )
        },
        ReciteStatus::Ok
    );
    let mut batch = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_session_begin(session, &raw mut batch) },
        ReciteStatus::Ok
    );
    unsafe { recite_buffer_free(&raw mut batch) };
    recite_session_free(session);
    recite_asset_free(asset);
}
