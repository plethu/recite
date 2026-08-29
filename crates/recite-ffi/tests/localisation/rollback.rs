use super::*;

#[test]
fn locale_variant_is_supplied_on_start_and_restore() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        "> greeting@78000000000000000001\n",
        "  Hello.\n",
        "> prompt@78000000000000000002\n",
        "  Pick one.\n",
        "  ? choose@78000000000000000003\n",
        "    Choose this.\n",
        "    -> after\n",
        ":: after\n",
        "-> END\n",
    ));
    let mut asset = 0;
    assert_eq!(
        unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) },
        ReciteStatus::Ok
    );
    let locale = cstr("fr");
    let variant = cstr("formal");
    let mut session = 0;
    let mut batch = ReciteBuffer::null();
    assert_eq!(
        unsafe {
            recite_session_start_with_locale_provider_and_variant(
                asset,
                std::ptr::null(),
                locale.as_ptr(),
                variant.as_ptr(),
                Some(locale_callback),
                std::ptr::null_mut(),
                &raw mut session,
                &raw mut batch,
            )
        },
        ReciteStatus::Ok
    );
    assert_eq!(decode_batch(&batch)["events"][0]["text"], "Bonjour formel.");
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
    assert_eq!(
        unsafe {
            recite_session_restore_with_values_and_locale_provider_and_variant(
                asset,
                snapshot_bytes.as_ptr(),
                snapshot_bytes.len(),
                std::ptr::null(),
                0,
                variant.as_ptr(),
                Some(locale_callback),
                std::ptr::null_mut(),
                &raw mut restored,
                &raw mut batch,
            )
        },
        ReciteStatus::Ok
    );
    assert!(
        decode_batch(&batch)["events"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    unsafe { recite_buffer_free(&raw mut batch) };
    recite_session_free(restored);
    recite_asset_free(asset);
}

#[test]
fn locale_callback_failure_rolls_back_begin_for_retry() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        "> greeting@74000000000000000001\n",
        "  Hello.\n",
        "-> END\n",
    ));
    let mut asset = 0;
    assert_eq!(
        unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) },
        ReciteStatus::Ok
    );
    let locale = cstr("fr-FR");
    let mut session = 0;
    assert_eq!(
        unsafe {
            recite_session_create(asset, std::ptr::null(), locale.as_ptr(), &raw mut session)
        },
        ReciteStatus::Ok
    );
    assert_eq!(
        unsafe {
            recite_session_set_locale_provider(
                session,
                Some(fail_once_callback),
                std::ptr::null_mut(),
            )
        },
        ReciteStatus::Ok
    );
    let mut first_batch = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_session_begin(session, &raw mut first_batch) },
        ReciteStatus::Localisation
    );
    assert_eq!(first_batch.len, 0);
    let mut second_batch = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_session_begin(session, &raw mut second_batch) },
        ReciteStatus::Ok
    );
    assert_eq!(decode_batch(&second_batch)["events"][0]["text"], "Hello.");
    unsafe { recite_buffer_free(&raw mut second_batch) };
    recite_session_free(session);
    recite_asset_free(asset);
}

#[test]
fn locale_callback_failure_rolls_back_choose_and_acknowledge() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        "> prompt@75000000000000000001\n",
        "  Pick one.\n",
        "  ? choose@75000000000000000002\n",
        "    Choose this.\n",
        "    -> after\n",
        ":: after\n",
        "> after@75000000000000000003\n",
        "  After.\n",
        "! blocking finish()\n",
        "> done@75000000000000000004\n",
        "  Done.\n",
        "-> END\n",
    ));
    let mut asset = 0;
    assert_eq!(
        unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) },
        ReciteStatus::Ok
    );
    let locale = cstr("fr");
    let mut session = 0;
    assert_eq!(
        unsafe {
            recite_session_create(asset, std::ptr::null(), locale.as_ptr(), &raw mut session)
        },
        ReciteStatus::Ok
    );
    assert_eq!(
        unsafe {
            recite_session_set_locale_provider(
                session,
                Some(fail_once_callback),
                std::ptr::null_mut(),
            )
        },
        ReciteStatus::Ok
    );
    FAIL_NEXT_CALLBACK.with(|value| value.set(false));
    let mut batch = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_session_begin(session, &raw mut batch) },
        ReciteStatus::Ok
    );
    let start = decode_batch(&batch);
    let choice_id = cstr(start["events"][0]["choices"][0]["id"].as_str().unwrap());
    unsafe { recite_buffer_free(&raw mut batch) };

    FAIL_NEXT_CALLBACK.with(|value| value.set(true));
    assert_eq!(
        unsafe { recite_session_choose(session, choice_id.as_ptr(), &raw mut batch) },
        ReciteStatus::Localisation
    );
    assert_eq!(
        batch.len, 0,
        "failed choice must not return a partial batch"
    );
    FAIL_NEXT_CALLBACK.with(|value| value.set(false));
    assert_eq!(
        unsafe { recite_session_choose(session, choice_id.as_ptr(), &raw mut batch) },
        ReciteStatus::Ok
    );
    let effect = decode_batch(&batch);
    let effect_id = cstr(effect["events"][1]["id"].as_str().unwrap());
    unsafe { recite_buffer_free(&raw mut batch) };

    FAIL_NEXT_CALLBACK.with(|value| value.set(true));
    assert_eq!(
        unsafe {
            recite_session_acknowledge_effect(
                session,
                effect_id.as_ptr(),
                1,
                std::ptr::null(),
                &raw mut batch,
            )
        },
        ReciteStatus::Localisation
    );
    assert_eq!(
        batch.len, 0,
        "failed acknowledgement must not return a partial batch"
    );
    FAIL_NEXT_CALLBACK.with(|value| value.set(false));
    assert_eq!(
        unsafe {
            recite_session_acknowledge_effect(
                session,
                effect_id.as_ptr(),
                1,
                std::ptr::null(),
                &raw mut batch,
            )
        },
        ReciteStatus::Ok
    );
    assert_eq!(decode_batch(&batch)["events"][0]["text"], "Fini.");
    unsafe { recite_buffer_free(&raw mut batch) };
    recite_session_free(session);
    recite_asset_free(asset);
}

#[test]
fn locale_plural_validator_rejects_invalid_utf8_and_reachable_arms() {
    let invalid_utf8 = [0xff_u8, 0];
    let mut arms = 0;
    assert_eq!(
        unsafe { recite_locale_validate_plural_rule(invalid_utf8.as_ptr().cast(), &raw mut arms) },
        ReciteStatus::Validation
    );
    let header = cstr("nplurals=2; plural=(n == 42 ? 2 : 0);");
    assert_eq!(
        unsafe { recite_locale_validate_plural_rule(header.as_ptr(), &raw mut arms) },
        ReciteStatus::Localisation
    );
}

#[test]
fn native_plural_evaluator_and_placeholder_validator_are_shared_authority() {
    let header = cstr("nplurals=2; plural=(n != 1);");
    let mut arm = 99;
    assert_eq!(
        unsafe { recite_locale_evaluate_plural_rule(header.as_ptr(), 1, &raw mut arm) },
        ReciteStatus::Ok
    );
    assert_eq!(arm, 0);
    assert_eq!(
        unsafe { recite_locale_evaluate_plural_rule(header.as_ptr(), 2, &raw mut arm) },
        ReciteStatus::Ok
    );
    assert_eq!(arm, 1);

    let source = cstr("Hello {name}.");
    let translated = cstr("Bonjour {name}.");
    assert_eq!(
        unsafe {
            recite_locale_validate_translation_placeholders(source.as_ptr(), translated.as_ptr())
        },
        ReciteStatus::Ok
    );
    let missing = cstr("Bonjour.");
    assert_eq!(
        unsafe {
            recite_locale_validate_translation_placeholders(source.as_ptr(), missing.as_ptr())
        },
        ReciteStatus::Localisation
    );
}
