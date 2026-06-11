//! Golden v0 wire-byte snapshot for the compiled MessagePack asset.
//!
//! The tag-surface round-trip test proves the compiler encoder and the core
//! decoder agree with each other. It cannot prove they agree with the
//! documented v0 layout in `docs/recite-production-spec.md` §12.2: a mirrored
//! change to both sides still round-trips cleanly. This snapshot pins the
//! encoded bytes themselves, so any wire-layout change — intentional or
//! accidental, on either side — surfaces in review as a snapshot diff.
//!
//! Per spec §12.2, the v0 shape stays correctable until the first tagged
//! release; updating this snapshot is the explicit, reviewable record of such
//! a correction. After the first tagged release, a diff here means a
//! `format_version` or `compiler_compatibility_version` bump is required
//! instead of a snapshot update.

use std::fmt::Write as _;

use recite_core::decode_compiled_dialogue_messagepack;

use super::fixture_support::assert_text_snapshot;
use super::tag_surface::compile_schema_tag_surface_asset;

#[test]
fn schema_tag_surface_messagepack_matches_the_golden_v0_wire_bytes() {
    let asset = compile_schema_tag_surface_asset();

    let decoded = decode_compiled_dialogue_messagepack(&asset.messagepack)
        .expect("golden tag-surface asset decodes");
    assert_eq!(decoded, asset.dialogue);
    assert!(
        !decoded.availability_reasons.is_empty(),
        "golden asset should pin availability reason rows"
    );
    assert!(
        !decoded.condition_availability_reasons.is_empty(),
        "golden asset should pin condition reason mapping rows"
    );

    assert_text_snapshot(
        &hex_dump(&asset.messagepack),
        "compiled_asset_v0_tag_surface_messagepack_hex".to_owned(),
    );
}

fn hex_dump(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2 + bytes.len() / 16 + 1);
    for chunk in bytes.chunks(32) {
        for byte in chunk {
            let _ = write!(output, "{byte:02x}");
        }
        output.push('\n');
    }
    output
}
