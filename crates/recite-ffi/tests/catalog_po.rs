#[path = "support/mod.rs"]
mod support;

use std::ffi::CString;

use recite_ffi::{
    ReciteBuffer, ReciteStatus, recite_asset_free, recite_asset_load, recite_buffer_free,
    recite_catalog_add_po, recite_catalog_create, recite_catalog_free, recite_session_begin,
    recite_session_choose, recite_session_create, recite_session_free,
    recite_session_restore_with_catalog, recite_session_set_catalog, recite_session_snapshot,
};
use support::{compile_to_bytes, decode_batch};

fn add_po(catalog: u64, po: &str) -> ReciteStatus {
    let locale = CString::new("fr").ok();
    match locale {
        Some(locale) => unsafe {
            recite_catalog_add_po(catalog, locale.as_ptr(), po.as_ptr(), po.len())
        },
        None => ReciteStatus::Validation,
    }
}

#[test]
fn catalogue_revision_is_retained_after_add_and_free() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        "> prompt@11111111111111111111\n",
        "  Pick.\n",
        "  ? go@22222222222222222222\n",
        "    Go.\n",
        "    -> after\n",
        ":: after\n",
        "> line@33333333333333333333\n",
        "  After.\n",
        "-> END\n",
    ));
    let mut asset = 0;
    assert_eq!(
        unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) },
        ReciteStatus::Ok
    );
    let mut catalog = 0;
    assert_eq!(
        unsafe { recite_catalog_create(&raw mut catalog) },
        ReciteStatus::Ok
    );
    assert_eq!(
        add_po(
            catalog,
            "msgctxt \"11111111111111111111\"\nmsgid \"Pick.\"\nmsgstr \"Choisir.\"\n"
        ),
        ReciteStatus::Ok
    );
    let locale = CString::new("fr").ok();
    let locale = match locale {
        Some(locale) => locale,
        None => panic!("valid locale"),
    };
    let mut first = 0;
    assert_eq!(
        unsafe { recite_session_create(asset, std::ptr::null(), locale.as_ptr(), &raw mut first) },
        ReciteStatus::Ok
    );
    assert_eq!(recite_session_set_catalog(first, catalog), ReciteStatus::Ok);
    let mut initial = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_session_begin(first, &raw mut initial) },
        ReciteStatus::Ok
    );
    assert_eq!(
        decode_batch(&initial)["events"][0]["line"]["text"],
        "Choisir."
    );
    unsafe { recite_buffer_free(&raw mut initial) };

    assert_eq!(
        add_po(
            catalog,
            "msgctxt \"33333333333333333333\"\nmsgid \"After.\"\nmsgstr \"Après.\"\n"
        ),
        ReciteStatus::Ok
    );
    // A conflicting file cannot partially install an earlier new entry.
    assert_eq!(
        add_po(
            catalog,
            concat!(
                "msgctxt \"44444444444444444444\"\nmsgid \"New.\"\nmsgstr \"Nouveau.\"\n\n",
                "msgctxt \"33333333333333333333\"\nmsgid \"After.\"\nmsgstr \"Autre.\"\n",
            )
        ),
        ReciteStatus::Localisation
    );

    let mut second = 0;
    assert_eq!(
        unsafe { recite_session_create(asset, std::ptr::null(), locale.as_ptr(), &raw mut second) },
        ReciteStatus::Ok
    );
    assert_eq!(
        recite_session_set_catalog(second, catalog),
        ReciteStatus::Ok
    );
    recite_catalog_free(catalog);
    let mut initial_second = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_session_begin(second, &raw mut initial_second) },
        ReciteStatus::Ok
    );
    unsafe { recite_buffer_free(&raw mut initial_second) };

    let choice = CString::new("22222222222222222222").ok();
    let choice = match choice {
        Some(choice) => choice,
        None => panic!("valid choice"),
    };
    let mut first_output = ReciteBuffer::null();
    let mut second_output = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_session_choose(first, choice.as_ptr(), &raw mut first_output) },
        ReciteStatus::Ok
    );
    assert_eq!(
        unsafe { recite_session_choose(second, choice.as_ptr(), &raw mut second_output) },
        ReciteStatus::Ok
    );
    assert_eq!(decode_batch(&first_output)["events"][0]["text"], "After.");
    assert_eq!(decode_batch(&second_output)["events"][0]["text"], "Après.");
    unsafe { recite_buffer_free(&raw mut first_output) };
    unsafe { recite_buffer_free(&raw mut second_output) };
    recite_session_free(first);
    recite_session_free(second);
    recite_asset_free(asset);
}

#[test]
fn restore_with_catalog_localises_first_drain() {
    let bytes =
        compile_to_bytes(":: start default\n> line@11111111111111111111\n  Hello.\n-> END\n");
    let mut asset = 0;
    assert_eq!(
        unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) },
        ReciteStatus::Ok
    );
    let mut catalog = 0;
    assert_eq!(
        unsafe { recite_catalog_create(&raw mut catalog) },
        ReciteStatus::Ok
    );
    assert_eq!(
        add_po(
            catalog,
            "msgctxt \"11111111111111111111\"\nmsgid \"Hello.\"\nmsgstr \"Bonjour.\"\n"
        ),
        ReciteStatus::Ok
    );
    let locale = CString::new("fr").ok();
    let locale = match locale {
        Some(locale) => locale,
        None => panic!("valid locale"),
    };
    let mut prepared = 0;
    assert_eq!(
        unsafe {
            recite_session_create(asset, std::ptr::null(), locale.as_ptr(), &raw mut prepared)
        },
        ReciteStatus::Ok
    );
    let mut snapshot = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_session_snapshot(prepared, &raw mut snapshot) },
        ReciteStatus::Ok
    );
    recite_session_free(prepared);
    let mut restored = 0;
    let mut batch = ReciteBuffer::null();
    assert_eq!(
        unsafe {
            recite_session_restore_with_catalog(
                asset,
                snapshot.data,
                snapshot.len,
                std::ptr::null(),
                0,
                catalog,
                std::ptr::null(),
                &raw mut restored,
                &raw mut batch,
            )
        },
        ReciteStatus::Ok
    );
    assert_eq!(decode_batch(&batch)["events"][0]["text"], "Bonjour.");
    unsafe { recite_buffer_free(&raw mut snapshot) };
    unsafe { recite_buffer_free(&raw mut batch) };
    recite_session_free(restored);
    recite_catalog_free(catalog);
    recite_asset_free(asset);
}

#[test]
fn catalogue_bad_inputs_do_not_replace_attached_provider() {
    assert_eq!(
        unsafe { recite_catalog_create(std::ptr::null_mut()) },
        ReciteStatus::Validation
    );
    let mut catalog = 0;
    assert_eq!(
        unsafe { recite_catalog_create(&raw mut catalog) },
        ReciteStatus::Ok
    );
    let locale = CString::new("fr").ok();
    let locale = match locale {
        Some(locale) => locale,
        None => panic!("valid locale"),
    };
    let invalid_po = [0xffu8];
    assert_eq!(
        unsafe {
            recite_catalog_add_po(
                catalog,
                locale.as_ptr(),
                invalid_po.as_ptr(),
                invalid_po.len(),
            )
        },
        ReciteStatus::Localisation
    );
    assert_eq!(
        unsafe { recite_catalog_add_po(u64::MAX, locale.as_ptr(), b"".as_ptr(), 0) },
        ReciteStatus::InvalidHandle
    );
    assert_eq!(
        unsafe { recite_catalog_add_po(catalog, std::ptr::null(), b"".as_ptr(), 0) },
        ReciteStatus::Validation
    );
    assert_eq!(
        add_po(
            catalog,
            "msgctxt \"11111111111111111111\"\nmsgid \"Hello.\"\nmsgstr \"Bonjour.\"\n"
        ),
        ReciteStatus::Ok
    );

    let bytes =
        compile_to_bytes(":: start default\n> line@11111111111111111111\n  Hello.\n-> END\n");
    let mut asset = 0;
    assert_eq!(
        unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut asset) },
        ReciteStatus::Ok
    );
    let mut session = 0;
    assert_eq!(
        unsafe {
            recite_session_create(asset, std::ptr::null(), locale.as_ptr(), &raw mut session)
        },
        ReciteStatus::Ok
    );
    assert_eq!(
        recite_session_set_catalog(session, catalog),
        ReciteStatus::Ok
    );
    assert_eq!(
        recite_session_set_catalog(session, u64::MAX),
        ReciteStatus::InvalidHandle
    );
    assert_eq!(
        recite_session_set_catalog(u64::MAX, catalog),
        ReciteStatus::InvalidHandle
    );
    let mut batch = ReciteBuffer::null();
    assert_eq!(
        unsafe { recite_session_begin(session, &raw mut batch) },
        ReciteStatus::Ok
    );
    assert_eq!(decode_batch(&batch)["events"][0]["text"], "Bonjour.");
    unsafe { recite_buffer_free(&raw mut batch) };
    recite_session_free(session);
    recite_catalog_free(catalog);
    recite_asset_free(asset);
}
