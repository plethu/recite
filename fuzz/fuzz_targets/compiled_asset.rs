//! Untrusted `.recitec` bytes: decoding never panics, a decoded asset
//! re-encodes to bytes that decode to the same asset, and starting, playing
//! and analysing it return errors rather than panicking.

#![no_main]

use libfuzzer_sys::fuzz_target;
use recite_core::compiled::{
    decode_compiled_dialogue_messagepack, encode_compiled_dialogue_messagepack,
};
use recite_runtime::{resume::analyze_save_impact, start_scene};

fuzz_target!(|data: &[u8]| {
    let Ok(asset) = decode_compiled_dialogue_messagepack(data) else {
        return;
    };
    let encoded = encode_compiled_dialogue_messagepack(&asset)
        .unwrap_or_else(|error| panic!("a decoded asset re-encodes: {error}"));
    let decoded = decode_compiled_dialogue_messagepack(&encoded)
        .unwrap_or_else(|error| panic!("a re-encoded asset decodes: {error}"));
    assert_eq!(decoded, asset, "re-encoding changed the asset");
    if let Ok(mut session) = start_scene(&asset, None) {
        recite_fuzz::play(&asset, &mut session, 32);
    }
    let _ = analyze_save_impact(&asset, &asset);
    let _ = analyze_save_impact(&asset, recite_fuzz::fixed_asset());
});
