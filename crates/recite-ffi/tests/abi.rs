#[path = "support/mod.rs"]
mod support;

use support::*;

#[test]
fn asset_load_and_free_round_trip() {
    let bytes = compile_to_bytes(concat!(
        ":: start default\n",
        "> hello@10000000000000000001\n",
        "  Hello.\n",
        "-> END\n",
    ));
    let mut handle: u64 = 0;
    let status = unsafe { recite_asset_load(bytes.as_ptr(), bytes.len(), &raw mut handle) };
    assert_eq!(status, ReciteStatus::Ok);
    assert_ne!(handle, 0);
    recite_asset_free(handle);
}
#[test]
fn status_codes_match_c_abi_design() {
    let cases = [
        (ReciteStatus::Ok, 0),
        (ReciteStatus::Validation, -1),
        (ReciteStatus::AssetLoadOrDecode, -2),
        (ReciteStatus::StaleOrIncompatible, -3),
        (ReciteStatus::SchemaMismatch, -4),
        (ReciteStatus::NoActiveSession, -5),
        (ReciteStatus::SessionAlreadyActive, -6),
        (ReciteStatus::UnknownStartBlock, -7),
        (ReciteStatus::InvalidChoice, -8),
        (ReciteStatus::UnavailableChoice, -9),
        (ReciteStatus::StaleChoice, -10),
        (ReciteStatus::MissingConditionHandler, -11),
        (ReciteStatus::ConditionEvaluation, -12),
        (ReciteStatus::InvalidConditionResult, -13),
        (ReciteStatus::EffectAcknowledgement, -14),
        (ReciteStatus::RejectedRefresh, -15),
        (ReciteStatus::SaveLoadIncompatibility, -16),
        (ReciteStatus::Localisation, -17),
        (ReciteStatus::MissingProjectionHandler, -18),
        (ReciteStatus::ProjectionEvaluation, -19),
        (ReciteStatus::InvalidProjectionResult, -20),
        (ReciteStatus::InvalidHandle, -21),
        (ReciteStatus::DialogueFault, -22),
    ];

    for (status, code) in cases {
        assert_eq!(status as i32, code);
        assert_eq!(ReciteStatus::try_from(code), Ok(status));
    }
    assert_eq!(ReciteStatus::try_from(-23), Err(()));
}
#[test]
fn invalid_bytes_returns_asset_load_error() {
    let garbage = b"not a compiled asset";
    let mut handle: u64 = 0;
    let status = unsafe { recite_asset_load(garbage.as_ptr(), garbage.len(), &raw mut handle) };
    assert_eq!(status, ReciteStatus::AssetLoadOrDecode);
    assert_eq!(handle, 0);
    let msg = unsafe { std::ffi::CStr::from_ptr(recite_last_error_message()) }.to_string_lossy();
    assert!(!msg.is_empty(), "error message should be set");
}
#[test]
fn invalid_handle_returns_error() {
    let mut session: u64 = 0;
    let mut batch = ReciteBuffer::null();
    let status = unsafe {
        recite_session_start(
            9999,
            std::ptr::null(),
            std::ptr::null(),
            &raw mut session,
            &raw mut batch,
        )
    };
    assert_eq!(status, ReciteStatus::InvalidHandle);
}
#[test]
fn session_free_unknown_handle_is_noop() {
    recite_session_free(0xDEADBEEF_DEADBEEF);
}
