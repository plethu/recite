#[path = "support/preview.rs"]
mod preview_support;

use std::collections::BTreeMap;

use recite_core::{ScalarValue, Value as MetadataValue};
use recite_runtime::{
    localisation::InterpolationValues,
    preview::{
        PreviewError, PreviewInputs, PreviewOptions, PreviewSession, PreviewSnapshot, PreviewStatus,
    },
};
use serde_json::json;
use serde_value::Value;

fn prompt_wire() -> Result<Value, Box<dyn std::error::Error>> {
    let asset = preview_support::asset(concat!(
        ":: start default\n",
        "> prompt@12345678901234567890 speaker=hero label=\"A\" amount=3 ratio=1.5 enabled=true items=[\"x\",2,3.5,false] bind=(count:int=$count)\n",
        "  One item.\n  | {count} items.\n",
        "  ? go@12345678901234567891\n    Go.\n    -> END\n",
    ));
    let mut values = InterpolationValues::new();
    values.insert("count".to_owned(), ScalarValue::Integer(2));
    let mut preview = PreviewSession::new(&asset, None, PreviewOptions::new())?;
    preview.step(PreviewInputs::new().with_interpolation_values(&values));
    let bytes = preview.snapshot()?.encode()?;
    Ok(rmp_serde::from_slice(&bytes)?)
}

fn decode(wire: &Value) -> Result<PreviewSnapshot, PreviewError> {
    let bytes =
        rmp_serde::to_vec_named(wire).map_err(|error| PreviewError::SnapshotEncodeFailed {
            reason: error.to_string(),
        })?;
    PreviewSnapshot::decode(&bytes)
}

fn fixture_field_mut<'a>(
    wire: &'a mut Value,
    path: &str,
) -> Result<&'a mut Value, Box<dyn std::error::Error>> {
    let mut field = wire;
    for part in path.split('/').filter(|part| !part.is_empty()) {
        field = match field {
            Value::Map(fields) => fields.get_mut(&Value::String(part.to_owned())),
            Value::Seq(fields) => fields.get_mut(part.parse::<usize>()?),
            _ => None,
        }
        .ok_or_else(|| format!("missing fixture field {path}"))?;
    }
    Ok(field)
}

fn replace(
    wire: &mut Value,
    path: &str,
    replacement: serde_json::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    *fixture_field_mut(wire, path)? = serde_json::from_value(replacement)?;
    Ok(())
}

#[test]
fn metadata_scalars_arrays_and_plural_projection_round_trip()
-> Result<(), Box<dyn std::error::Error>> {
    let wire = prompt_wire()?;
    let snapshot = decode(&wire)?;
    let decoded = PreviewSnapshot::decode(&snapshot.encode()?)?;
    assert_eq!(snapshot, decoded);
    let PreviewStatus::WaitingForChoice { prompt } = snapshot.state().status() else {
        panic!("fixture must contain a pending prompt");
    };
    let line = prompt.line().expect("line projection");
    assert_eq!(line.metadata.len(), 5);
    assert_eq!(
        line.metadata
            .iter()
            .map(|entry| (entry.key.as_str(), entry.value.clone()))
            .collect::<BTreeMap<_, _>>(),
        BTreeMap::from([
            (
                "label",
                MetadataValue::Scalar(ScalarValue::String("A".to_owned()))
            ),
            ("amount", MetadataValue::Scalar(ScalarValue::Integer(3))),
            ("ratio", MetadataValue::Scalar(ScalarValue::Float(1.5))),
            ("enabled", MetadataValue::Scalar(ScalarValue::Boolean(true))),
            (
                "items",
                MetadataValue::Array(vec![
                    ScalarValue::String("x".to_owned()),
                    ScalarValue::Integer(2),
                    ScalarValue::Float(3.5),
                    ScalarValue::Boolean(false)
                ])
            ),
        ])
    );
    assert_eq!(line.plural.as_ref().expect("plural projection").count, 2);
    assert_eq!(line.text, "2 items.");
    Ok(())
}

#[test]
fn status_maps_are_strict_and_unit_statuses_use_strings() -> Result<(), Box<dyn std::error::Error>>
{
    let original = prompt_wire()?;
    for status in [
        json!({}),
        json!({"Ready": null}),
        json!({"Ended": null}),
        json!({"Unknown": null}),
        json!("WaitingForChoice"),
        json!("Unknown"),
        json!(42),
        json!({"WaitingForChoice": {"prompt": {}}, "WaitingForEffect": {"effect": {}}}),
    ] {
        let mut wire = original.clone();
        replace(&mut wire, "/state/status", status)?;
        assert!(matches!(
            decode(&wire),
            Err(PreviewError::SnapshotDecodeFailed { .. })
        ));
    }
    for status in ["Ready", "Ended"] {
        let mut wire = original.clone();
        replace(&mut wire, "/state/status", json!(status))?;
        assert!(decode(&wire).is_ok(), "valid unit status {status}");
    }
    let mut extra = original.clone();
    let Value::Map(status) = fixture_field_mut(&mut extra, "/state/status")? else {
        panic!("status map");
    };
    status.insert(Value::String("extra".to_owned()), Value::Unit);
    assert!(matches!(
        decode(&extra),
        Err(PreviewError::SnapshotDecodeFailed { .. })
    ));
    Ok(())
}

#[test]
fn line_projection_rejects_invalid_ids_metadata_and_plural_outcome_roles()
-> Result<(), Box<dyn std::error::Error>> {
    let original = prompt_wire()?;
    for (pointer, replacement) in [
        (
            "/state/status/WaitingForChoice/prompt/line_projection/id",
            json!(""),
        ),
        (
            "/state/status/WaitingForChoice/prompt/line_projection/speaker",
            json!(""),
        ),
        (
            "/state/status/WaitingForChoice/prompt/line_projection/metadata/0/value",
            json!({"Scalar": {"Identifier": "actor"}}),
        ),
        (
            "/state/status/WaitingForChoice/prompt/line_projection/metadata/0/value",
            json!({"Array": [{"Identifier": "actor"}]}),
        ),
        (
            "/state/status/WaitingForChoice/prompt/line_projection/metadata/0/source_span",
            json!({"file":"source.recite","start":{"line":0,"column":1},"end":null}),
        ),
        (
            "/state/status/WaitingForChoice/prompt/line_projection/plural/resolution/outcome",
            json!("MissingEntry"),
        ),
        (
            "/state/status/WaitingForChoice/prompt/line_projection/plural/resolution/attempts",
            json!([{"locale":"en","context":"context","key":"x","selected_arm":null,"outcome":"Translated"}]),
        ),
    ] {
        let mut wire = original.clone();
        replace(&mut wire, pointer, replacement)?;
        assert!(
            matches!(
                decode(&wire),
                Err(PreviewError::SnapshotDecodeFailed { .. })
            ),
            "{pointer}"
        );
    }
    let mut valid_attempts = original;
    replace(
        &mut valid_attempts,
        "/state/status/WaitingForChoice/prompt/line_projection/plural/resolution/attempts",
        json!([
        {"locale":"en","context":"context","key":"a","selected_arm":null,"outcome":"MissingPluralForms"},
        {"locale":"en","context":"context","key":"b","selected_arm":null,"outcome":"MissingEntry"},
        {"locale":"en","context":"context","key":"c","selected_arm":1,"outcome":"MissingTranslation"},
        {"locale":"en","context":"context","key":"d","selected_arm":1,"outcome":"Matched"}
        ]),
    )?;
    let decoded = decode(&valid_attempts)?;
    assert_eq!(PreviewSnapshot::decode(&decoded.encode()?)?, decoded);
    Ok(())
}
