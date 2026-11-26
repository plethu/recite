//! Untrusted save bytes: decoding, exact restore and resume against a fixed
//! asset never panic, and a restored or resumed session plays without
//! panicking.

#![no_main]

use libfuzzer_sys::fuzz_target;
use recite_runtime::{
    resume::{ResumeOptions, SessionResume, resume_session},
    snapshot::{decode_session_snapshot_messagepack, restore_session},
};

fuzz_target!(|data: &[u8]| {
    let Ok(snapshot) = decode_session_snapshot_messagepack(data) else {
        return;
    };
    let asset = recite_fuzz::fixed_asset();
    if let Ok(mut session) = restore_session(asset, snapshot.clone()) {
        recite_fuzz::play(asset, &mut session, 16);
    }
    for options in [ResumeOptions::new(), ResumeOptions::exact_only()] {
        if let Ok(SessionResume::Resumed { mut session, .. }) =
            resume_session(asset, snapshot.clone(), &options)
        {
            recite_fuzz::play(asset, &mut session, 16);
        }
    }
});
