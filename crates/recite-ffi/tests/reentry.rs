#[path = "support/mod.rs"]
mod support;

use std::ffi::c_void;

use support::*;

struct Reentry {
    session: u64,
    snapshot_status: ReciteStatus,
    output_untouched: bool,
    locale_calls: usize,
}

unsafe fn attempt_reentry(userdata: *mut c_void) {
    let state = unsafe { &mut *userdata.cast::<Reentry>() };
    let sentinel = std::ptr::dangling_mut();
    let mut output = ReciteBuffer {
        data: sentinel,
        len: 17,
    };
    state.snapshot_status = unsafe { recite_session_snapshot(state.session, &raw mut output) };
    state.output_untouched = output.data == sentinel && output.len == 17;
    recite_session_free(state.session);
}

unsafe extern "C" fn condition(
    _query: *const ReciteConditionQuery,
    userdata: *mut c_void,
) -> ReciteConditionResult {
    unsafe { attempt_reentry(userdata) };
    static TRUE: [u8; 18] = [
        0x82, 0xa4, b'k', b'i', b'n', b'd', 0xa4, b'b', b'o', b'o', b'l', 0xa5, b'v', b'a', b'l',
        b'u', b'e', 0xc3,
    ];
    ReciteConditionResult {
        ok: 1,
        value_msgpack: TRUE.as_ptr(),
        value_len: TRUE.len(),
        error_message: std::ptr::null(),
    }
}

unsafe extern "C" fn locale(
    _query: *const ReciteLocaleQuery,
    userdata: *mut c_void,
) -> ReciteLocaleResult {
    unsafe { (*userdata.cast::<Reentry>()).locale_calls += 1 };
    unsafe { attempt_reentry(userdata) };
    ReciteLocaleResult {
        ok: 1,
        text: std::ptr::null(),
        selected_arm: -1,
        matched_locale: std::ptr::null(),
        matched_context: std::ptr::null(),
        matched_key: std::ptr::null(),
        attempts: std::ptr::null(),
        attempts_len: 0,
        error_message: std::ptr::null(),
    }
}

#[test]
fn callbacks_cannot_reenter_or_free_the_session_registry() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        ":if ready()\n",
        "  > hello@52000000000000000001\n",
        "    Hello.\n",
        "-> END\n",
    ));
    for use_locale in [false, true] {
        let mut asset = 0;
        assert_eq!(
            unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) },
            ReciteStatus::Ok
        );
        let mut state = Reentry {
            session: 0,
            snapshot_status: ReciteStatus::Ok,
            output_untouched: false,
            locale_calls: 0,
        };
        let locale_name = cstr("fr");
        assert_eq!(
            unsafe {
                recite_session_create(
                    asset,
                    std::ptr::null(),
                    locale_name.as_ptr(),
                    &raw mut state.session,
                )
            },
            ReciteStatus::Ok
        );
        // A session retains the loaded asset independently of its public handle.
        recite_asset_free(asset);
        let name = cstr("ready");
        let userdata = (&raw mut state).cast();
        assert_eq!(
            unsafe {
                recite_session_register_condition(
                    state.session,
                    name.as_ptr(),
                    Some(condition),
                    userdata,
                )
            },
            ReciteStatus::Ok
        );
        if use_locale {
            assert_eq!(
                unsafe {
                    recite_session_set_locale_provider(state.session, Some(locale), userdata)
                },
                ReciteStatus::Ok
            );
        }
        let mut output = ReciteBuffer::null();
        assert_eq!(
            unsafe { recite_session_begin(state.session, &raw mut output) },
            ReciteStatus::Ok
        );
        assert_eq!(decode_batch(&output)["events"][0]["text"], "Hello.");
        assert_eq!(state.snapshot_status, ReciteStatus::Validation);
        assert!(state.output_untouched);
        assert_eq!(state.locale_calls, usize::from(use_locale));
        unsafe { recite_buffer_free(&raw mut output) };
        assert_eq!(
            unsafe { recite_session_snapshot(state.session, &raw mut output) },
            ReciteStatus::Ok
        );
        unsafe { recite_buffer_free(&raw mut output) };
        recite_session_free(state.session);
    }
}
