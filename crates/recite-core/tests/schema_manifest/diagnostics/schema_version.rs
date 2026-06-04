use recite_core::load_schema_manifest_str;

use crate::diagnostic_codes;

#[test]
fn malformed_manifest_shape_reports_schema_diagnostic() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/malformed_shape.json",
        include_str!("../../../../../fixtures/schema/invalid/malformed_shape.json"),
    );

    assert!(report.schema.is_none());
    assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA001"]);
    assert_eq!(report.diagnostics[0].span.start.line(), 2);
}

#[test]
fn unsupported_manifest_versions_report_schema_diagnostic() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/unsupported_version.json",
        include_str!("../../../../../fixtures/schema/invalid/unsupported_version.json"),
    );

    assert!(report.schema.is_none());
    assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA002"]);
}

#[test]
fn unsupported_manifest_version_span_uses_the_top_level_field() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/unsupported_version.json",
        r#"{
  "types": {
    "schema_version": { "kind": "enum", "values": ["nested"] }
  },
  "schema_version": 2
}"#,
    );

    assert!(report.schema.is_none());
    assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA002"]);
    assert_eq!(report.diagnostics[0].span.start.line(), 5);
}

#[test]
fn schema_version_accepts_json_numbers_equal_to_one() {
    for source in [
        r#"{
  "schema_version": 1.0
}"#,
        r#"{
  "schema_version": 1e0
}"#,
        r#"{
  "schema_version": 10e-1
}"#,
    ] {
        let report =
            load_schema_manifest_str("fixtures/schema/valid/schema_version_one.json", source);

        assert_eq!(diagnostic_codes(&report), Vec::<&str>::new());
        assert!(report.schema.is_some());
    }
}

#[test]
fn schema_version_token_lookup_uses_the_top_level_field() {
    let report = load_schema_manifest_str(
        "fixtures/schema/valid/schema_version_one.json",
        r#"{
  "types": {
    "schema_version": { "kind": "enum", "values": ["nested"] }
  },
  "schema_version": 1
}"#,
    );

    assert_eq!(diagnostic_codes(&report), Vec::<&str>::new());
    assert!(report.schema.is_some());
}

#[test]
fn schema_version_token_lookup_uses_json_key_semantics() {
    let report = load_schema_manifest_str(
        "fixtures/schema/valid/schema_version_one.json",
        "{\"schema\\u005fversion\": 1}",
    );

    assert_eq!(diagnostic_codes(&report), Vec::<&str>::new());
    assert!(report.schema.is_some());
}

#[test]
fn schema_version_rejects_numbers_that_only_round_to_one() {
    for source in [
        r#"{
  "schema_version": 1.0000000000000001
}"#,
        r#"{
  "schema_version": 1.00000000000000000000000000001
}"#,
    ] {
        let report = load_schema_manifest_str(
            "fixtures/schema/invalid/schema_version_near_one.json",
            source,
        );

        assert!(report.schema.is_none());
        assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA002"]);
    }
}

#[test]
fn schema_version_rejects_large_negative_exponents_without_allocating_expected_value() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/schema_version_large_exponent.json",
        r#"{
  "schema_version": 1e-100000
}"#,
    );

    assert!(report.schema.is_none());
    assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA002"]);
}
