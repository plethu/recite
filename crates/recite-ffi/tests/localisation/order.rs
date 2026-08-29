use std::cell::RefCell;
use std::ffi::{CString, c_char, c_void};

use super::support::*;

thread_local! {
    static ORDER_CALLBACK_STORAGE: RefCell<OrderCallbackStorage> =
        RefCell::new(OrderCallbackStorage::default());
}

#[derive(Default)]
struct OrderCallbackStorage {
    strings: Vec<CString>,
    attempts: Vec<ReciteLocaleAttempt>,
}

#[test]
fn c_abi_locale_attempts_preserve_context_first_order_and_trace() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        "> letters@79000000000000000001 bind=(count:int=$count)\n",
        "  One letter.\n",
        "  | Many letters.\n",
        "-> END\n",
    ));
    let mut asset = 0;
    assert_eq!(
        unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) },
        ReciteStatus::Ok
    );

    let count_name = cstr("count");
    let values = [ReciteInterpolationValue {
        name: count_name.as_ptr(),
        kind: ReciteInterpolationValueKind::Integer as u32,
        string_value: std::ptr::null(),
        integer_value: 2,
        float_value: 0.0,
        boolean_value: 0,
    }];
    let locale = cstr("fr-CA");
    let variant = cstr("formal");
    let plural_header = cstr("nplurals=2; plural=(n != 1);");
    let mut nplurals = 0;
    assert_eq!(
        unsafe { recite_locale_validate_plural_rule(plural_header.as_ptr(), &raw mut nplurals) },
        ReciteStatus::Ok
    );
    assert_eq!(nplurals, 2);

    let mut session = 0;
    let mut batch = ReciteBuffer::null();
    assert_eq!(
        unsafe {
            recite_session_start_with_values_and_locale_provider_and_variant(
                asset,
                std::ptr::null(),
                locale.as_ptr(),
                variant.as_ptr(),
                values.as_ptr(),
                values.len(),
                Some(ordered_locale_callback),
                std::ptr::null_mut(),
                &raw mut session,
                &raw mut batch,
            )
        },
        ReciteStatus::Ok
    );

    let output = decode_batch(&batch);
    let line = &output["events"][0];
    assert_eq!(line["text"], "Beaucoup de lettres.");
    let resolution = &line["plural"]["resolution"];
    assert_eq!(resolution["matched_locale"], "fr");
    assert_eq!(resolution["matched_context"], "79000000000000000001");
    assert_eq!(resolution["matched_key"], "79000000000000000001");
    assert_eq!(resolution["matched_arm"], 1);

    let attempts = resolution["attempts"].as_array().expect("attempts array");
    assert_eq!(attempts.len(), 4);
    let expected = [
        (
            "fr-CA",
            "79000000000000000001&formal",
            "missing_translation",
        ),
        ("fr", "79000000000000000001&formal", "missing_translation"),
        ("fr-CA", "79000000000000000001", "missing_entry"),
        ("fr", "79000000000000000001", "matched"),
    ];
    for (attempt, (locale, context, outcome)) in attempts.iter().zip(expected) {
        assert_eq!(attempt["locale"], locale);
        assert_eq!(attempt["context"], context);
        assert_eq!(attempt["key"], "79000000000000000001");
        assert_eq!(attempt["selected_arm"], 1);
        assert_eq!(attempt["outcome"], outcome);
    }

    unsafe { recite_buffer_free(&raw mut batch) };
    recite_session_free(session);
    recite_asset_free(asset);
}

unsafe extern "C" fn ordered_locale_callback(
    _query: *const ReciteLocaleQuery,
    _userdata: *mut c_void,
) -> ReciteLocaleResult {
    ORDER_CALLBACK_STORAGE.with(|storage| {
        let mut storage = storage.borrow_mut();
        storage.strings.clear();
        storage.attempts.clear();
        storage.strings.reserve(20);
        storage.attempts.reserve(4);

        // Empty and fuzzy catalogue records both continue as
        // MissingTranslation. The C ABI has no conflict outcome: a host
        // conflict must not match, so the third candidate is a non-matching
        // MissingEntry without inventing a new ABI outcome.
        let text = push_string(&mut storage, "Beaucoup de lettres.");
        let matched_locale = push_string(&mut storage, "fr");
        let matched_context = push_string(&mut storage, "79000000000000000001");
        let matched_key = push_string(&mut storage, "79000000000000000001");
        for (locale, context, outcome) in [
            ("fr-CA", "79000000000000000001&formal", 2),
            ("fr", "79000000000000000001&formal", 2),
            ("fr-CA", "79000000000000000001", 1),
            ("fr", "79000000000000000001", 3),
        ] {
            let locale = push_string(&mut storage, locale);
            let context = push_string(&mut storage, context);
            let key = push_string(&mut storage, "79000000000000000001");
            storage.attempts.push(ReciteLocaleAttempt {
                locale,
                context,
                key,
                selected_arm: 1,
                outcome,
            });
        }

        ReciteLocaleResult {
            ok: 1,
            text,
            selected_arm: 1,
            matched_locale,
            matched_context,
            matched_key,
            attempts: storage.attempts.as_ptr(),
            attempts_len: storage.attempts.len(),
            error_message: std::ptr::null(),
        }
    })
}

fn push_string(storage: &mut OrderCallbackStorage, value: &str) -> *const c_char {
    let value = match CString::new(value) {
        Ok(value) => value,
        Err(_) => panic!("fixture string has no NUL"),
    };
    storage.strings.push(value);
    storage.strings[storage.strings.len() - 1].as_ptr()
}
