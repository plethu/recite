//! Golden v0 MessagePack bytes and compact JSON inspection projections.
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
//! a correction. After the first tagged release, a MessagePack snapshot diff means a
//! `format_version` or `compiler_compatibility_version` bump is required
//! instead of a snapshot update.
//! JSON snapshots separately pin inspection field order, tags and formatting.

use std::fmt::Write as _;

use recite_compiler::compile::{CompileInput, CompiledAssetOutput, compile_inputs};
use recite_core::compiled::decode_compiled_dialogue_messagepack;

use super::fixture_support::assert_text_snapshot;
use super::tag_surface::{
    compile_literal_reason_tag_surface_asset, compile_schema_tag_surface_asset,
    compile_value_tag_surface_asset,
};
use super::{compile_fixture, options};

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

#[test]
fn value_tag_surface_messagepack_matches_the_golden_v0_wire_bytes() {
    let asset = compile_value_tag_surface_asset();

    let decoded = decode_compiled_dialogue_messagepack(&asset.messagepack)
        .expect("value golden asset decodes");
    assert_eq!(decoded, asset.dialogue);

    assert_text_snapshot(
        &hex_dump(&asset.messagepack),
        "compiled_asset_v0_value_tag_surface_messagepack_hex".to_owned(),
    );
}

#[test]
fn literal_reason_tag_surface_messagepack_matches_the_golden_v0_wire_bytes() {
    let asset = compile_literal_reason_tag_surface_asset();

    let decoded = decode_compiled_dialogue_messagepack(&asset.messagepack)
        .expect("literal-reason golden asset decodes");
    assert_eq!(decoded, asset.dialogue);
    assert_eq!(
        asset.dialogue.condition_availability_reasons[0]
            .args
            .iter()
            .map(|binding| &binding.value)
            .collect::<Vec<_>>(),
        [
            &recite_core::compiled::CompiledAvailabilityReasonArgValue::Literal(
                recite_core::ScalarValue::Boolean(true)
            ),
            &recite_core::compiled::CompiledAvailabilityReasonArgValue::Literal(
                recite_core::ScalarValue::Float(1.25)
            ),
            &recite_core::compiled::CompiledAvailabilityReasonArgValue::Literal(
                recite_core::ScalarValue::Integer(7)
            ),
            &recite_core::compiled::CompiledAvailabilityReasonArgValue::Literal(
                recite_core::ScalarValue::String("literal".to_owned())
            ),
        ]
    );

    assert_text_snapshot(
        &hex_dump(&asset.messagepack),
        "compiled_asset_v0_literal_reason_tag_surface_messagepack_hex".to_owned(),
    );
}

#[test]
fn inspection_json_pins_every_v0_tag_payload() {
    for (name, asset) in [
        ("schema_tags", compile_schema_tag_surface_asset()),
        ("value_tags", compile_value_tag_surface_asset()),
        (
            "literal_reason_tags",
            compile_literal_reason_tag_surface_asset(),
        ),
    ] {
        assert_text_snapshot(
            &asset.inspection_json,
            format!("compiled_asset_v0_{name}_json"),
        );
    }
}

#[test]
fn inspection_json_pins_plural_interpolation_and_authored_unicode() {
    let asset = compile_fixture("fixtures/recite/valid/language_pressure.recite");
    assert_text_snapshot(
        &asset.inspection_json,
        "compiled_asset_v0_language_pressure_json".to_owned(),
    );
}

#[test]
fn inspection_json_pins_escaping_and_numeric_edges() {
    let asset = compile_escaping_and_numeric_edges();
    let decoded = decode_compiled_dialogue_messagepack(&asset.messagepack)
        .expect("numeric and escaping asset decodes");
    assert_eq!(decoded, asset.dialogue);
    assert_text_snapshot(
        &asset.inspection_json,
        "compiled_asset_v0_escaping_numeric_json".to_owned(),
    );
}

fn compile_escaping_and_numeric_edges() -> CompiledAssetOutput {
    let report = compile_inputs(
        [CompileInput::new(
            "dialogue/échapper.recite",
            concat!(
                ":: escaped default label=\"héllo\\n\\\"quote\\\"\\\\slash\\tend\"\n",
                "> edge@11111111111111111111 fraction=-1.25 negative_zero=-0.0 huge=1e30 tiny=1e-20 min=-9223372036854775808 max=9223372036854775807 empty=[]\n",
                "  😀\"À bientôt\" \\path.\n",
                "-> END\n",
            ),
        )],
        options(),
    )
    .expect("numeric and escaping source compiles");
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    report
        .asset
        .expect("numeric and escaping source emits an asset")
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
