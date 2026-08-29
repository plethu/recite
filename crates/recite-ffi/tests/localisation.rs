#[path = "localisation/order.rs"]
mod order;
#[path = "localisation/rollback.rs"]
mod rollback;
mod support;

use std::cell::{Cell, RefCell};
use std::ffi::{CStr, CString};

use support::*;

thread_local! {
    static CALLBACK_STORAGE: RefCell<CallbackStorage> = RefCell::new(CallbackStorage::default());
    static FAIL_NEXT_CALLBACK: Cell<bool> = const { Cell::new(true) };
}

#[derive(Default)]
struct CallbackStorage {
    strings: Vec<CString>,
    attempts: Vec<ReciteLocaleAttempt>,
}

#[derive(Clone, Copy)]
struct ReturnedStrings {
    text: *const std::ffi::c_char,
    matched_locale: *const std::ffi::c_char,
    matched_context: *const std::ffi::c_char,
    matched_key: *const std::ffi::c_char,
}

#[test]
fn locale_callback_translates_start_choose_and_restore() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        "> greeting@71000000000000000001 bind=(name:string=$name)\n",
        "  Hello {name}.\n",
        "> letters@71000000000000000002 bind=(count:int=$count)\n",
        "  You have one letter.\n",
        "  | You have {count} letters.\n",
        "> prompt@71000000000000000003\n",
        "  Pick one.\n",
        "  ? choose@71000000000000000004\n",
        "    Choose this.\n",
        "    -> after\n",
        ":: after\n",
        "! blocking finish()\n",
        "> after@71000000000000000005\n",
        "  Finished.\n",
        "-> END\n",
    ));
    let mut asset = 0;
    assert_eq!(
        unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) },
        ReciteStatus::Ok
    );

    let name = cstr("name");
    let count = cstr("count");
    let ada = cstr("Ada");
    let values = [
        ReciteInterpolationValue {
            name: name.as_ptr(),
            kind: ReciteInterpolationValueKind::String as u32,
            string_value: ada.as_ptr(),
            integer_value: 0,
            float_value: 0.0,
            boolean_value: 0,
        },
        ReciteInterpolationValue {
            name: count.as_ptr(),
            kind: ReciteInterpolationValueKind::Integer as u32,
            string_value: std::ptr::null(),
            integer_value: 2,
            float_value: 0.0,
            boolean_value: 0,
        },
    ];
    let locale = cstr("fr-CA");
    let mut session = 0;
    let mut batch = ReciteBuffer::null();
    assert_eq!(
        unsafe {
            recite_session_create_with_values(
                asset,
                std::ptr::null(),
                locale.as_ptr(),
                values.as_ptr(),
                values.len(),
                &raw mut session,
            )
        },
        ReciteStatus::Ok
    );
    assert_eq!(
        unsafe {
            recite_session_set_locale_provider(session, Some(locale_callback), std::ptr::null_mut())
        },
        ReciteStatus::Ok
    );
    assert_eq!(
        unsafe { recite_session_begin(session, &raw mut batch) },
        ReciteStatus::Ok
    );
    let start = decode_batch(&batch);
    assert_eq!(start["events"][0]["text"], "Bonjour Ada.");
    assert_eq!(start["events"][1]["text"], "Vous avez 2 lettres.");
    assert_eq!(
        start["events"][1]["plural"]["resolution"]["outcome"],
        "translated"
    );
    assert_eq!(
        start["events"][1]["plural"]["resolution"]["attempts"][0]["outcome"],
        "matched"
    );
    assert_eq!(start["events"][2]["choices"][0]["text"], "Choisir ceci.");
    let choice_id = start["events"][2]["choices"][0]["id"].as_str().unwrap();
    let choice_id = cstr(choice_id);
    unsafe { recite_buffer_free(&raw mut batch) };

    let mut snapshot = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_session_snapshot(session, &raw mut snapshot) },
        ReciteStatus::Ok
    );
    let snapshot_bytes =
        unsafe { std::slice::from_raw_parts(snapshot.data, snapshot.len).to_vec() };
    unsafe { recite_buffer_free(&raw mut snapshot) };
    recite_session_free(session);

    let mut restored = 0;
    let mut restored_batch = ReciteBuffer::null();
    assert_eq!(
        unsafe {
            recite_session_restore_with_values_and_locale_provider(
                asset,
                snapshot_bytes.as_ptr(),
                snapshot_bytes.len(),
                values.as_ptr(),
                values.len(),
                Some(locale_callback),
                std::ptr::null_mut(),
                &raw mut restored,
                &raw mut restored_batch,
            )
        },
        ReciteStatus::Ok
    );
    assert!(
        decode_batch(&restored_batch)["events"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    unsafe { recite_buffer_free(&raw mut restored_batch) };

    let mut effect_batch = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_session_choose(restored, choice_id.as_ptr(), &raw mut effect_batch) },
        ReciteStatus::Ok
    );
    let effect = decode_batch(&effect_batch);
    assert_eq!(effect["events"][0]["kind"], "effect");
    let effect_id = cstr(effect["events"][0]["id"].as_str().unwrap());
    unsafe { recite_buffer_free(&raw mut effect_batch) };

    let mut after_batch = ReciteBuffer::null();
    assert_eq!(
        unsafe {
            recite_session_acknowledge_effect(
                restored,
                effect_id.as_ptr(),
                1,
                std::ptr::null(),
                &raw mut after_batch,
            )
        },
        ReciteStatus::Ok
    );
    let after = decode_batch(&after_batch);
    assert_eq!(after["events"][0]["text"], "Terminé.");
    unsafe { recite_buffer_free(&raw mut after_batch) };
    recite_session_free(restored);
    recite_asset_free(asset);
}

#[test]
fn locale_callback_can_return_source_fallback_for_line_and_plural() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        "> greeting@72000000000000000001\n",
        "  Hello.\n",
        "> letters@72000000000000000002 bind=(count:int=$count)\n",
        "  You have one letter.\n",
        "  | You have {count} letters.\n",
        "-> END\n",
    ));
    let mut asset = 0;
    unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) };
    let count_name = cstr("count");
    let values = [ReciteInterpolationValue {
        name: count_name.as_ptr(),
        kind: ReciteInterpolationValueKind::Integer as u32,
        string_value: std::ptr::null(),
        integer_value: 1,
        float_value: 0.0,
        boolean_value: 0,
    }];
    let locale = cstr("fr-FR");
    let mut session = 0;
    let mut batch = ReciteBuffer::null();
    assert_eq!(
        unsafe {
            recite_session_create_with_values(
                asset,
                std::ptr::null(),
                locale.as_ptr(),
                values.as_ptr(),
                values.len(),
                &raw mut session,
            )
        },
        ReciteStatus::Ok
    );
    assert_eq!(
        unsafe {
            recite_session_set_locale_provider(session, Some(locale_callback), std::ptr::null_mut())
        },
        ReciteStatus::Ok
    );
    assert_eq!(
        unsafe { recite_session_begin(session, &raw mut batch) },
        ReciteStatus::Ok
    );
    let fallback = decode_batch(&batch);
    assert_eq!(fallback["events"][0]["text"], "Hello.");
    assert_eq!(fallback["events"][1]["text"], "You have one letter.");
    unsafe { recite_buffer_free(&raw mut batch) };
    recite_session_free(session);
    recite_asset_free(asset);
}

unsafe extern "C" fn fail_once_callback(
    _query: *const ReciteLocaleQuery,
    _userdata: *mut std::ffi::c_void,
) -> ReciteLocaleResult {
    if FAIL_NEXT_CALLBACK.with(|value| value.replace(false)) {
        return ReciteLocaleResult {
            ok: 2,
            text: std::ptr::null(),
            selected_arm: -1,
            matched_locale: std::ptr::null(),
            matched_context: std::ptr::null(),
            matched_key: std::ptr::null(),
            attempts: std::ptr::null(),
            attempts_len: 0,
            error_message: std::ptr::null(),
        };
    }
    unsafe { locale_callback(_query, _userdata) }
}

unsafe extern "C" fn locale_callback(
    query: *const ReciteLocaleQuery,
    _userdata: *mut std::ffi::c_void,
) -> ReciteLocaleResult {
    let query = unsafe { &*query };
    let id = match unsafe { CStr::from_ptr(query.id) }.to_str() {
        Ok(id) => id,
        Err(error) => panic!("fixture query ID should be UTF-8: {error}"),
    };
    let is_plural = query.kind == 1;
    CALLBACK_STORAGE.with(|storage| {
        let mut storage = storage.borrow_mut();
        storage.strings.clear();
        storage.attempts.clear();
        storage.strings.reserve(4);
        let translation = match (id, is_plural) {
            ("71000000000000000001", false) => Some("Bonjour {name}."),
            ("78000000000000000001", false) if query.variant.is_null() => Some("Bonjour."),
            ("78000000000000000001", false) => Some("Bonjour formel."),
            ("75000000000000000001", false) => None,
            ("75000000000000000002", false) => None,
            ("75000000000000000003", false) => Some("Après."),
            ("75000000000000000004", false) => Some("Fini."),
            ("71000000000000000002", true) if query.count != 1 => {
                Some("Vous avez {count} lettres.")
            }
            ("71000000000000000003", false) => None,
            ("71000000000000000004", false) => Some("Choisir ceci."),
            ("71000000000000000005", false) => Some("Terminé."),
            _ => None,
        };
        let Some(translation) = translation else {
            return ReciteLocaleResult {
                ok: 1,
                text: std::ptr::null(),
                selected_arm: -1,
                matched_locale: std::ptr::null(),
                matched_context: std::ptr::null(),
                matched_key: std::ptr::null(),
                attempts: std::ptr::null(),
                attempts_len: 0,
                error_message: std::ptr::null(),
            };
        };
        let text = fixture_cstring(translation);
        storage.strings.push(text);
        let text_ptr = storage.strings[0].as_ptr();
        if is_plural {
            let locale = fixture_cstring("fr-CA");
            let context = fixture_cstring(id);
            let key = fixture_cstring(id);
            storage.strings.extend([locale, context, key]);
            let locale_ptr = storage.strings[1].as_ptr();
            let context_ptr = storage.strings[2].as_ptr();
            let key_ptr = storage.strings[3].as_ptr();
            storage.attempts.push(ReciteLocaleAttempt {
                locale: locale_ptr,
                context: context_ptr,
                key: key_ptr,
                selected_arm: if query.count == 1 { 0 } else { 1 },
                outcome: 3,
            });
        }
        let returned = ReturnedStrings {
            text: text_ptr,
            matched_locale: if is_plural {
                storage.strings[1].as_ptr()
            } else {
                std::ptr::null()
            },
            matched_context: if is_plural {
                storage.strings[2].as_ptr()
            } else {
                std::ptr::null()
            },
            matched_key: if is_plural {
                storage.strings[3].as_ptr()
            } else {
                std::ptr::null()
            },
        };
        ReciteLocaleResult {
            ok: 1,
            text: returned.text,
            selected_arm: if is_plural && query.count != 1 {
                1
            } else if is_plural {
                0
            } else {
                -1
            },
            matched_locale: returned.matched_locale,
            matched_context: returned.matched_context,
            matched_key: returned.matched_key,
            attempts: storage.attempts.as_ptr(),
            attempts_len: storage.attempts.len(),
            error_message: std::ptr::null(),
        }
    })
}

fn fixture_cstring(value: &str) -> CString {
    match CString::new(value) {
        Ok(value) => value,
        Err(error) => panic!("fixture string should not contain NUL: {error}"),
    }
}
