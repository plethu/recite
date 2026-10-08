use std::collections::BTreeMap;

use recite_core::compiled::{ContentFingerprint, FingerprintAlgorithm, FingerprintDigest};
use recite_core::schema::{
    AvailabilityReasonArgBinding, ConditionDefinition, ConditionReturnType,
    MetadataDomainDefinition, PresentationAffordanceFieldSource, ProducerMetadataValue,
    ProducerOrigin, ProjectSchema, SchemaLiteralValue, SchemaProjectionInputSource, SchemaTypeRef,
    export_schema_manifest_json, load_schema_manifest_str, producer_content_fingerprint,
    validate_project_schema,
};

fn load(source: &str) -> ProjectSchema {
    let report = load_schema_manifest_str("native-test.json", source);
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let Some(schema) = report.schema else {
        panic!("manifest loader returned neither a schema nor diagnostics");
    };
    schema
}

fn full_schema() -> ProjectSchema {
    load(include_str!(
        "../../../fixtures/schema/valid/full_manifest.json"
    ))
}

#[test]
fn loaded_schema_fixtures_pass_native_validation_without_semantic_changes() {
    for source in [
        include_str!("../../../fixtures/schema/valid/full_manifest.json"),
        include_str!("../../../fixtures/schema/valid/generated_manifest.json"),
        include_str!("../../../fixtures/schema/valid/unsorted_manifest.json"),
        include_str!("../../../fixtures/schema/valid/writer_examples.json"),
    ] {
        let schema = load(source);
        validate_project_schema(&schema).expect("loaded fixture is a valid native schema");
        let exported = load(&export_schema_manifest_json(&schema).expect("export"));
        assert_eq!(schema, exported);
    }
}

#[test]
fn native_validation_rejects_unused_invalid_declarations() {
    let mut schema = ProjectSchema::empty_v1();
    schema.conditions.insert(
        "unused".into(),
        ConditionDefinition {
            params: Vec::new(),
            returns: ConditionReturnType::Enum("missing".into()),
            availability_reason: None,
        },
    );
    let diagnostics =
        validate_project_schema(&schema).expect_err("unknown enum is invalid even when unused");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "RECITE_SCHEMA004")
    );
}

fn literal_schema() -> ProjectSchema {
    load(
        r#"{
        "schema_version": 1,
        "conditions": { "ready": { "availability_reason": {
            "reason": "reason", "args": { "value": "value" }
        } } },
        "availability_reasons": { "reason": {
            "template": "{value}", "params": [{ "name": "value", "type": "string" }]
        } },
        "presentation_projectors": { "p": {
            "candidates": { "kind": "runtime_event", "event": "dialogue" },
            "inputs": [{ "name": "value", "source": { "kind": "literal", "value": "value" }, "type": "string" }],
            "outputs": { "o": { "target": "candidate", "kind": "badge", "slot": "prefix",
                "fields": { "value": { "source": { "kind": "literal", "value": "value" }, "type": "string" } }
            } }
        } }
    }"#,
    )
}

#[test]
fn malformed_floats_cannot_be_reinterpreted_as_strings_at_any_literal_site() {
    for index in 0..3 {
        for value in ["NaN", "inf", "+1", "1e400"] {
            let mut schema = literal_schema();
            let literal = SchemaLiteralValue::Float(value.into());
            match index {
                0 => {
                    schema
                        .conditions
                        .get_mut("ready")
                        .expect("condition")
                        .availability_reason
                        .as_mut()
                        .expect("mapping")
                        .args
                        .insert(
                            "value".into(),
                            AvailabilityReasonArgBinding::Literal(literal),
                        );
                }
                1 => {
                    schema
                        .presentation_projectors
                        .get_mut("p")
                        .expect("projector")
                        .inputs[0]
                        .source = SchemaProjectionInputSource::Literal(literal);
                }
                _ => {
                    schema
                        .presentation_projectors
                        .get_mut("p")
                        .expect("projector")
                        .outputs
                        .get_mut("o")
                        .expect("output")
                        .fields
                        .get_mut("value")
                        .expect("field")
                        .source = PresentationAffordanceFieldSource::Literal(literal);
                }
            };
            let diagnostics = validate_project_schema(&schema).expect_err("malformed native float");
            assert!(diagnostics[0].message.contains("finite and representable"));
        }
    }
}

#[test]
fn native_numeric_literal_variants_must_survive_manifest_lowering() {
    for (type_ref, literal) in [
        (SchemaTypeRef::Int, SchemaLiteralValue::Float("1".into())),
        (SchemaTypeRef::Float, SchemaLiteralValue::Int(1)),
    ] {
        let mut schema = literal_schema();
        let field = schema
            .presentation_projectors
            .get_mut("p")
            .expect("projector")
            .outputs
            .get_mut("o")
            .expect("output")
            .fields
            .get_mut("value")
            .expect("field");
        field.type_ref = type_ref;
        field.source = PresentationAffordanceFieldSource::Literal(literal);
        let diagnostics =
            validate_project_schema(&schema).expect_err("native numeric kind must not change");
        assert_eq!(
            diagnostics[0]
                .presentation
                .as_ref()
                .expect("typed native diagnostic")
                .id()
                .as_str(),
            "diagnostic-schema-001-native-invalid"
        );
        assert!(diagnostics[0].message.contains("typed semantic content"));
    }
}

fn origins(schema: &mut ProjectSchema) -> Vec<&mut ProducerOrigin> {
    let mut origins = Vec::new();
    for registry in schema.registries.values_mut() {
        origins.extend(registry.origin.iter_mut());
        origins.extend(registry.value_origins.values_mut());
    }
    for reason in schema.availability_reasons.values_mut() {
        origins.extend(reason.origin.iter_mut());
    }
    for domain in schema.metadata_domains.values_mut() {
        match domain {
            MetadataDomainDefinition::Flat(domain) => {
                origins.extend(domain.provenance.origin.iter_mut());
                origins.extend(domain.provenance.value_origins.values_mut());
            }
            MetadataDomainDefinition::Contextual(domain) => {
                origins.extend(domain.provenance.origin.iter_mut());
                origins.extend(domain.provenance.context_origins.values_mut());
                origins.extend(
                    domain
                        .provenance
                        .value_origins
                        .values_mut()
                        .flat_map(|values| values.values_mut()),
                );
            }
        }
    }
    origins
}

#[test]
fn reserved_origin_extensions_cannot_overwrite_fields_at_any_provenance_site() {
    let count = origins(&mut full_schema()).len();
    assert!(count >= 7, "fixture covers each origin owner");
    for index in 0..count {
        for key in ["kind", "id", "label"] {
            let mut schema = full_schema();
            origins(&mut schema)[index].extensions.insert(
                key.into(),
                ProducerMetadataValue::String("replacement".into()),
            );
            let diagnostics =
                validate_project_schema(&schema).expect_err("reserved key is not an extension");
            assert!(diagnostics[0].message.contains("must be namespaced"));
        }
    }
}

#[test]
fn producer_extension_numbers_are_preserved_without_f64_or_string_coercion() {
    for (number, valid) in [
        ("NaN", false),
        ("01", false),
        ("184467440737095516160", false),
        ("18446744073709551615", true),
        ("1e+100", true),
    ] {
        let mut schema = full_schema();
        origins(&mut schema)[0].extensions.insert(
            "host:detail".into(),
            ProducerMetadataValue::Array(vec![ProducerMetadataValue::Object(BTreeMap::from([(
                "number".into(),
                ProducerMetadataValue::Number(number.into()),
            )]))]),
        );
        if valid {
            let exported = export_schema_manifest_json(&schema)
                .expect("accepted metadata numbers are preserved exactly");
            assert_eq!(load(&exported), schema);
        } else {
            assert_eq!(
                validate_project_schema(&schema).expect_err("invalid number")[0]
                    .code
                    .as_str(),
                "RECITE_SCHEMA001"
            );
        }
    }
}

#[test]
fn opaque_producer_content_fingerprints_retain_text_and_reject_non_utf8() {
    let mut schema = full_schema();
    let fingerprint = producer_content_fingerprint("custom", "opaque:v1").expect("fingerprint");
    schema
        .producer_metadata
        .as_mut()
        .expect("metadata")
        .content_fingerprint = Some(fingerprint);
    assert_eq!(
        load(&export_schema_manifest_json(&schema).expect("export")),
        schema
    );
    schema
        .producer_metadata
        .as_mut()
        .expect("metadata")
        .content_fingerprint = Some(
        ContentFingerprint::new(
            FingerprintAlgorithm::new("custom").expect("algorithm"),
            FingerprintDigest::new(vec![0xff]).expect("digest"),
        )
        .expect("opaque bytes"),
    );
    assert!(
        validate_project_schema(&schema).expect_err("non-UTF8 cannot be manifest text")[0]
            .message
            .contains("UTF-8")
    );
}

#[test]
fn provenance_validation_cannot_be_cached_by_semantic_fingerprint() {
    let mut schema = full_schema();
    let fingerprint = schema.canonical_fingerprint();
    origins(&mut schema)[0]
        .extensions
        .insert("id".into(), ProducerMetadataValue::String("shadow".into()));
    assert_eq!(schema.canonical_fingerprint(), fingerprint);
    assert!(validate_project_schema(&schema).is_err());
}

#[test]
fn native_validation_keeps_empty_schema_valid() {
    validate_project_schema(&ProjectSchema::empty_v1()).expect("empty schema remains valid");
}
