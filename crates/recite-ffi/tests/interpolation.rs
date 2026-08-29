#[path = "support/mod.rs"]
mod support;

use support::*;

fn interpolation_source() -> &'static str {
    concat!(
        ":: start default\n",
        "> greeting@91000000000000000001 bind=(name:string=$name) bind=(ready:bool=$ready)\n",
        "  Hello {name}; ready={ready}.\n",
        "> letters@91000000000000000002 bind=(count:int=$remaining)\n",
        "  You have one letter.\n",
        "  | You have {count} letters.\n",
        "> prompt@91000000000000000003 bind=(name:string=$name)\n",
        "  Pick for {name}.\n",
        "  ? choose@91000000000000000004 bind=(name:string=$name) bind=(ready:bool=$ready)\n",
        "    Choose {name} ({ready}).\n",
        "    -> after\n",
        ":: after\n",
        "> remaining@91000000000000000005 bind=(remaining:int=$remaining)\n",
        "  Remaining: {remaining}.\n",
        "-> END\n",
    )
}

fn typed_values<'a>(
    name: &'a std::ffi::CString,
    ready: &'a std::ffi::CString,
    remaining: &'a std::ffi::CString,
    ada: &'a std::ffi::CString,
) -> [ReciteInterpolationValue; 3] {
    [
        ReciteInterpolationValue {
            name: name.as_ptr(),
            kind: ReciteInterpolationValueKind::String as u32,
            string_value: ada.as_ptr(),
            integer_value: 0,
            float_value: 0.0,
            boolean_value: 0,
        },
        ReciteInterpolationValue {
            name: ready.as_ptr(),
            kind: ReciteInterpolationValueKind::Boolean as u32,
            string_value: std::ptr::null(),
            integer_value: 0,
            float_value: 0.0,
            boolean_value: 1,
        },
        ReciteInterpolationValue {
            name: remaining.as_ptr(),
            kind: ReciteInterpolationValueKind::Integer as u32,
            string_value: std::ptr::null(),
            integer_value: 2,
            float_value: 0.0,
            boolean_value: 0,
        },
    ]
}

#[test]
fn typed_values_drive_lines_plural_and_choices() {
    let bytes = compile_to_bytes(interpolation_source());
    let mut asset = 0;
    assert_eq!(
        unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) },
        ReciteStatus::Ok
    );

    let name = cstr("name");
    let ready = cstr("ready");
    let remaining = cstr("remaining");
    let ada = cstr("Ada");
    let values = typed_values(&name, &ready, &remaining, &ada);
    let mut session = 0;
    let mut batch = ReciteBuffer::null();
    assert_eq!(
        unsafe {
            recite_session_start_with_values(
                asset,
                std::ptr::null(),
                std::ptr::null(),
                values.as_ptr(),
                values.len(),
                &raw mut session,
                &raw mut batch,
            )
        },
        ReciteStatus::Ok
    );

    let first = decode_batch(&batch);
    assert_eq!(event_kinds(&first), ["line", "line", "prompt"]);
    assert_eq!(first["events"][0]["text"], "Hello Ada; ready=true.");
    assert_eq!(first["events"][1]["text"], "You have 2 letters.");
    let choice_id = first["events"][2]["choices"][0]["id"]
        .as_str()
        .expect("choice has an ID")
        .to_owned();
    assert_eq!(
        first["events"][2]["choices"][0]["text"],
        "Choose Ada (true)."
    );
    unsafe { recite_buffer_free(&raw mut batch) };

    let choice = cstr(&choice_id);
    let mut next_batch = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_session_choose(session, choice.as_ptr(), &raw mut next_batch) },
        ReciteStatus::Ok
    );
    let next = decode_batch(&next_batch);
    assert_eq!(event_kinds(&next), ["line", "end"]);
    assert_eq!(next["events"][0]["text"], "Remaining: 2.");

    unsafe { recite_buffer_free(&raw mut next_batch) };
    recite_session_free(session);
    recite_asset_free(asset);
}

#[test]
fn missing_or_wrong_typed_values_project_as_localisation_errors() {
    let bytes = compile_to_bytes(interpolation_source());
    let mut asset = 0;
    assert_eq!(
        unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) },
        ReciteStatus::Ok
    );

    let name = cstr("name");
    let ada = cstr("Ada");
    let missing_ready = [ReciteInterpolationValue {
        name: name.as_ptr(),
        kind: ReciteInterpolationValueKind::String as u32,
        string_value: ada.as_ptr(),
        integer_value: 0,
        float_value: 0.0,
        boolean_value: 0,
    }];
    let mut session = 0;
    let mut batch = ReciteBuffer::null();
    assert_eq!(
        unsafe {
            recite_session_start_with_values(
                asset,
                std::ptr::null(),
                std::ptr::null(),
                missing_ready.as_ptr(),
                missing_ready.len(),
                &raw mut session,
                &raw mut batch,
            )
        },
        ReciteStatus::Localisation
    );
    assert_eq!(session, 0);

    let ready = cstr("ready");
    let wrong_ready = [ReciteInterpolationValue {
        name: ready.as_ptr(),
        kind: ReciteInterpolationValueKind::String as u32,
        string_value: ada.as_ptr(),
        integer_value: 0,
        float_value: 0.0,
        boolean_value: 0,
    }];
    assert_eq!(
        unsafe {
            recite_session_start_with_values(
                asset,
                std::ptr::null(),
                std::ptr::null(),
                wrong_ready.as_ptr(),
                wrong_ready.len(),
                &raw mut session,
                &raw mut batch,
            )
        },
        ReciteStatus::Localisation
    );
    assert_eq!(session, 0);

    let unknown_kind = [ReciteInterpolationValue {
        name: name.as_ptr(),
        kind: 99,
        string_value: ada.as_ptr(),
        integer_value: 0,
        float_value: 0.0,
        boolean_value: 0,
    }];
    assert_eq!(
        unsafe {
            recite_session_start_with_values(
                asset,
                std::ptr::null(),
                std::ptr::null(),
                unknown_kind.as_ptr(),
                unknown_kind.len(),
                &raw mut session,
                &raw mut batch,
            )
        },
        ReciteStatus::Validation
    );
    assert_eq!(session, 0);

    recite_asset_free(asset);
}

#[test]
fn values_can_be_replaced_between_create_and_begin() {
    let bytes = compile_to_bytes(interpolation_source());
    let mut asset = 0;
    unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) };
    let mut session = 0;
    assert_eq!(
        unsafe {
            recite_session_create(asset, std::ptr::null(), std::ptr::null(), &raw mut session)
        },
        ReciteStatus::Ok
    );

    let name = cstr("name");
    let ready = cstr("ready");
    let remaining = cstr("remaining");
    let ada = cstr("Ada");
    let values = typed_values(&name, &ready, &remaining, &ada);
    assert_eq!(
        unsafe { recite_session_set_interpolation_values(session, values.as_ptr(), values.len()) },
        ReciteStatus::Ok
    );

    let mut batch = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_session_begin(session, &raw mut batch) },
        ReciteStatus::Ok
    );
    assert_eq!(
        decode_batch(&batch)["events"][0]["text"],
        "Hello Ada; ready=true."
    );
    let first = decode_batch(&batch);
    let choice_id = first["events"][2]["choices"][0]["id"]
        .as_str()
        .expect("choice has an ID")
        .to_owned();
    unsafe { recite_buffer_free(&raw mut batch) };

    let remaining = cstr("remaining");
    let wrong_value = cstr("two");
    let wrong_type = [ReciteInterpolationValue {
        name: remaining.as_ptr(),
        kind: ReciteInterpolationValueKind::String as u32,
        string_value: wrong_value.as_ptr(),
        integer_value: 0,
        float_value: 0.0,
        boolean_value: 0,
    }];
    assert_eq!(
        unsafe {
            recite_session_set_interpolation_values(session, wrong_type.as_ptr(), wrong_type.len())
        },
        ReciteStatus::Ok
    );
    let choice = cstr(&choice_id);
    let mut failed_batch = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_session_choose(session, choice.as_ptr(), &raw mut failed_batch) },
        ReciteStatus::Localisation
    );

    let replacement = [ReciteInterpolationValue {
        name: remaining.as_ptr(),
        kind: ReciteInterpolationValueKind::Integer as u32,
        string_value: std::ptr::null(),
        integer_value: 1,
        float_value: 0.0,
        boolean_value: 0,
    }];
    assert_eq!(
        unsafe {
            recite_session_set_interpolation_values(
                session,
                replacement.as_ptr(),
                replacement.len(),
            )
        },
        ReciteStatus::Ok
    );
    let mut next_batch = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_session_choose(session, choice.as_ptr(), &raw mut next_batch) },
        ReciteStatus::Ok
    );
    assert_eq!(
        decode_batch(&next_batch)["events"][0]["text"],
        "Remaining: 1."
    );
    unsafe { recite_buffer_free(&raw mut next_batch) };
    recite_session_free(session);
    recite_asset_free(asset);
}

#[test]
fn restore_with_values_drives_resumption_and_projects_errors() {
    let bytes = compile_to_bytes(interpolation_source());
    let mut asset = 0;
    assert_eq!(
        unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) },
        ReciteStatus::Ok
    );

    let after = cstr("after");
    let mut original = 0;
    assert_eq!(
        unsafe {
            recite_session_create(asset, after.as_ptr(), std::ptr::null(), &raw mut original)
        },
        ReciteStatus::Ok
    );
    let mut snapshot = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_session_snapshot(original, &raw mut snapshot) },
        ReciteStatus::Ok
    );
    let snapshot_bytes =
        unsafe { std::slice::from_raw_parts(snapshot.data, snapshot.len).to_vec() };
    unsafe { recite_buffer_free(&raw mut snapshot) };
    recite_session_free(original);

    let mut restored = 0;
    let mut batch = ReciteBuffer::null();
    assert_eq!(
        unsafe {
            recite_session_restore_with_values(
                asset,
                snapshot_bytes.as_ptr(),
                snapshot_bytes.len(),
                std::ptr::null(),
                0,
                &raw mut restored,
                &raw mut batch,
            )
        },
        ReciteStatus::Localisation
    );
    assert_eq!(restored, 0);

    let remaining = cstr("remaining");
    let wrong_value = cstr("two");
    let wrong_type = [ReciteInterpolationValue {
        name: remaining.as_ptr(),
        kind: ReciteInterpolationValueKind::String as u32,
        string_value: wrong_value.as_ptr(),
        integer_value: 0,
        float_value: 0.0,
        boolean_value: 0,
    }];
    assert_eq!(
        unsafe {
            recite_session_restore_with_values(
                asset,
                snapshot_bytes.as_ptr(),
                snapshot_bytes.len(),
                wrong_type.as_ptr(),
                wrong_type.len(),
                &raw mut restored,
                &raw mut batch,
            )
        },
        ReciteStatus::Localisation
    );
    assert_eq!(restored, 0);

    let valid = [ReciteInterpolationValue {
        name: remaining.as_ptr(),
        kind: ReciteInterpolationValueKind::Integer as u32,
        string_value: std::ptr::null(),
        integer_value: 2,
        float_value: 0.0,
        boolean_value: 0,
    }];
    assert_eq!(
        unsafe {
            recite_session_restore_with_values(
                asset,
                snapshot_bytes.as_ptr(),
                snapshot_bytes.len(),
                valid.as_ptr(),
                valid.len(),
                &raw mut restored,
                &raw mut batch,
            )
        },
        ReciteStatus::Ok
    );
    let restored_output = decode_batch(&batch);
    assert_eq!(event_kinds(&restored_output), ["line", "end"]);
    assert_eq!(restored_output["events"][0]["text"], "Remaining: 2.");
    unsafe { recite_buffer_free(&raw mut batch) };
    recite_session_free(restored);
    recite_asset_free(asset);
}
