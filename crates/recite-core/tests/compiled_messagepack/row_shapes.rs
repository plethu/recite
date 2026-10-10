use recite_core::compiled::{
    CompiledAssetDecodeError, CompiledInterpolationMode, decode_compiled_dialogue_messagepack,
    encode_compiled_dialogue_messagepack,
};
use serde_value::Value;

use super::support::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const LINE_TABLE: usize = 6;
const CHOICE_TABLE: usize = 7;

#[test]
fn decode_rejects_partial_and_extra_text_row_fields() -> TestResult {
    for (table, accepted, maximum) in [
        (LINE_TABLE, &[5, 7, 9][..], 11),
        (CHOICE_TABLE, &[9, 11][..], 13),
    ] {
        for arity in 0..=maximum {
            if accepted.contains(&arity) {
                continue;
            }
            let mut fields = wire_row(table)?;
            fields.resize(arity, Value::Unit);
            let bytes = asset_with_row(table, Value::Seq(fields))?;
            assert!(
                matches!(
                    decode_compiled_dialogue_messagepack(&bytes),
                    Err(CompiledAssetDecodeError::MalformedAsset(_))
                ),
                "table {table}, arity {arity} must be malformed"
            );
        }
    }
    Ok(())
}

#[test]
fn decode_rejects_scalar_and_map_text_rows() -> TestResult {
    for table in [LINE_TABLE, CHOICE_TABLE] {
        for row in [
            Value::Unit,
            Value::U32(7),
            Value::String("row".to_owned()),
            Value::Map(Default::default()),
            Value::Bytes(vec![0]),
        ] {
            let bytes = asset_with_row(table, row)?;
            assert!(matches!(
                decode_compiled_dialogue_messagepack(&bytes),
                Err(CompiledAssetDecodeError::MalformedAsset(_))
            ));
        }
    }
    Ok(())
}

#[test]
fn decode_rejects_wrong_text_row_field_types_in_prefixes_and_tails() -> TestResult {
    for (table, field, value) in [
        (LINE_TABLE, 0, Value::U32(1)),
        (LINE_TABLE, 1, Value::Unit),
        (LINE_TABLE, 2, Value::String("speaker".to_owned())),
        (LINE_TABLE, 3, Value::Seq(Vec::new())),
        (LINE_TABLE, 4, Value::Bool(false)),
        (LINE_TABLE, 5, Value::Unit),
        (LINE_TABLE, 6, Value::Unit),
        (LINE_TABLE, 7, Value::U32(1)),
        (LINE_TABLE, 8, Value::Map(Default::default())),
        (CHOICE_TABLE, 3, Value::String("ready()".to_owned())),
        (CHOICE_TABLE, 8, Value::Unit),
        (CHOICE_TABLE, 9, Value::Unit),
        (CHOICE_TABLE, 10, Value::Unit),
    ] {
        let mut fields = wire_row(table)?;
        fields.resize(fields.len().max(field + 1), Value::Unit);
        fields[field] = value;
        let bytes = asset_with_row(table, Value::Seq(fields))?;
        assert!(
            matches!(
                decode_compiled_dialogue_messagepack(&bytes),
                Err(CompiledAssetDecodeError::MalformedAsset(_))
            ),
            "table {table}, field {field} must be malformed"
        );
    }
    Ok(())
}

#[test]
fn decode_distinguishes_legacy_rows_from_present_current_tails() -> TestResult {
    for (table, legacy_arity) in [(LINE_TABLE, 5), (CHOICE_TABLE, 9)] {
        let mut fields = wire_row(table)?;
        fields.truncate(legacy_arity);
        fields[1] = Value::String("Literal {unbound}.".to_owned());
        let decoded =
            decode_compiled_dialogue_messagepack(&asset_with_row(table, Value::Seq(fields))?)?;
        let (mode, source_text, authored_source_text, bindings_empty) = if table == LINE_TABLE {
            let row = &decoded.lines[0];
            assert!(row.plural_source_text.is_none());
            assert!(row.authored_plural_source_text.is_none());
            (
                row.interpolation_mode,
                &row.source_text,
                &row.authored_source_text,
                row.interpolation_bindings.is_empty(),
            )
        } else {
            let row = &decoded.choices[0];
            (
                row.interpolation_mode,
                &row.source_text,
                &row.authored_source_text,
                row.interpolation_bindings.is_empty(),
            )
        };
        assert_eq!(mode, CompiledInterpolationMode::Legacy);
        assert_eq!(source_text, "Literal {unbound}.");
        assert_eq!(authored_source_text, source_text);
        assert!(bindings_empty);
        let encoded = encode_compiled_dialogue_messagepack(&decoded)?;
        assert_eq!(encoded_row(&encoded, table)?.len(), legacy_arity);
    }
    Ok(())
}

#[test]
fn decode_accepts_seven_field_lines_and_explicit_nil_plural_tails() -> TestResult {
    let fields = wire_row(LINE_TABLE)?;
    let seven = decode_compiled_dialogue_messagepack(&asset_with_row(
        LINE_TABLE,
        Value::Seq(fields.clone()),
    )?)?;
    let mut plural_fields = fields;
    plural_fields.extend([Value::Unit, Value::Unit]);
    let nine = decode_compiled_dialogue_messagepack(&asset_with_row(
        LINE_TABLE,
        Value::Seq(plural_fields),
    )?)?;
    assert_eq!(seven, nine);
    assert_eq!(
        nine.lines[0].interpolation_mode,
        CompiledInterpolationMode::Current
    );
    assert_eq!(
        nine.choices[0].interpolation_mode,
        CompiledInterpolationMode::Current
    );
    assert_eq!(nine.lines[0].authored_source_text, "Hello.");
    assert_eq!(nine.choices[0].authored_source_text, "Choose.");
    let encoded = encode_compiled_dialogue_messagepack(&nine)?;
    assert_eq!(encoded_row(&encoded, LINE_TABLE)?.len(), 9);
    assert_eq!(encoded_row(&encoded, CHOICE_TABLE)?.len(), 11);
    assert_eq!(decode_compiled_dialogue_messagepack(&encoded)?, nine);
    Ok(())
}

fn valid_rows() -> TestResult<Vec<Value>> {
    let mut asset = valid_wire_asset();
    asset.lines.push(WireLine {
        id: "line",
        source_text: "Hello.",
        speaker: None,
        metadata: WireRange(0, 0),
        source_map: 0,
    });
    asset.line_lookup.push(WireLookupEntry {
        id: "line",
        index: 0,
    });
    asset.choices.push(WireChoice {
        id: "choice",
        source_text: "Choose.",
        metadata: WireRange(0, 0),
        availability_requirement: None,
        availability_requirement_source_text: None,
        availability_reason_override: None,
        target: Tagged::nil(recite_core::compiled::V0_DIVERT_TARGET_TAG_END),
        echo: Tagged::nil(recite_core::compiled::V0_CHOICE_ECHO_TAG_NONE),
        source_map: 0,
    });
    asset.choice_lookup.push(WireLookupEntry {
        id: "choice",
        index: 0,
    });
    match serde_value::to_value(asset)? {
        Value::Seq(rows) => Ok(rows),
        _ => Err("wire fixture must be a sequence".into()),
    }
}

fn take_row(mut rows: Vec<Value>, table: usize) -> TestResult<Vec<Value>> {
    let Value::Seq(mut entries) = rows.remove(table) else {
        return Err("wire table must be a sequence".into());
    };
    let Value::Seq(fields) = entries.remove(0) else {
        return Err("wire row must be a sequence".into());
    };
    Ok(fields)
}

fn wire_row(table: usize) -> TestResult<Vec<Value>> {
    take_row(valid_rows()?, table)
}

fn asset_with_row(table: usize, row: Value) -> TestResult<Vec<u8>> {
    let mut rows = valid_rows()?;
    rows[table] = Value::Seq(vec![row]);
    Ok(rmp_serde::to_vec(&Value::Seq(rows))?)
}

fn encoded_row(bytes: &[u8], table: usize) -> TestResult<Vec<Value>> {
    let Value::Seq(rows) = rmp_serde::from_slice(bytes)? else {
        return Err("encoded asset must be a sequence".into());
    };
    take_row(rows, table)
}
