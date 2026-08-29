use recite_core::{
    AvailabilityReasonArgBinding, ConditionReturnType, EffectMode, MetadataContextSelector,
    MetadataDomainDefinition, MetadataOccurrence, MetadataTarget, MissingMetadataContextPolicy,
    ProducerFingerprint, ProducerOrigin, ProjectionInputRef, ProjectionOutputTarget,
    SchemaLiteralValue, SchemaProjectionInputSource, SchemaProjectionSelector,
    SchemaTypeDefinition, SchemaTypeRef, load_schema_manifest_str,
};

use crate::diagnostic_codes;

#[test]
fn valid_generated_manifest_loads_into_canonical_schema() {
    let report = load_schema_manifest_str(
        "fixtures/schema/valid/generated_manifest.json",
        include_str!("../../../../fixtures/schema/valid/generated_manifest.json"),
    );

    assert_eq!(diagnostic_codes(&report), Vec::<&str>::new());
    let schema = report.schema.expect("valid schema manifest");

    assert_eq!(schema.schema_version, 1);
    assert_eq!(
        schema.types.keys().map(String::as_str).collect::<Vec<_>>(),
        ["thread_stage_kind"]
    );
    let SchemaTypeDefinition::Enum(thread_stage_kind) = &schema.types["thread_stage_kind"];
    assert_eq!(
        thread_stage_kind
            .values
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["angry", "completed", "fine", "fresh", "tired"]
    );

    assert_eq!(
        schema.conditions["trust_gte"].returns,
        ConditionReturnType::Bool
    );
    let mapping = schema.conditions["trust_gte"]
        .availability_reason
        .as_ref()
        .expect("trust_gte has reason mapping");
    assert_eq!(mapping.reason.as_str(), "trust_too_low");
    assert_eq!(
        mapping.args["subject"],
        AvailabilityReasonArgBinding::ConditionParam("actor_a".to_owned())
    );
    assert_eq!(
        schema.availability_reasons["trust_too_low"].template,
        "{subject} does not trust {target} enough ({threshold})."
    );
    assert_eq!(
        schema.availability_reasons["trust_too_low"].origin,
        Some(ProducerOrigin {
            kind: "script_member".to_owned(),
            id: "schema/availability.rs".to_owned(),
            label: None,
            ..Default::default()
        })
    );
    assert_eq!(
        schema.conditions["thread_stage"].returns,
        ConditionReturnType::Enum("thread_stage_kind".to_owned())
    );
    assert_eq!(
        schema.effects["advance_thread"]
            .modes
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        [EffectMode::Deferred, EffectMode::Blocking]
    );
    assert_eq!(
        schema.metadata["sfx"]
            .targets
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        [MetadataTarget::Choice, MetadataTarget::Line]
    );
    assert!(!schema.markup["shake"].allows_nesting);
}

#[test]
fn full_generated_manifest_loads_producer_metadata_and_projection_features() {
    let report = load_schema_manifest_str(
        "fixtures/schema/valid/full_manifest.json",
        include_str!("../../../../fixtures/schema/valid/full_manifest.json"),
    );

    assert_eq!(diagnostic_codes(&report), Vec::<&str>::new());
    let schema = report.schema.expect("full schema manifest");
    assert_eq!(
        schema.producer_metadata,
        Some(recite_core::ProducerMetadata {
            producer: Some(recite_core::ProducerIdentity {
                kind: "adapter".to_owned(),
                id: "example".to_owned(),
            }),
            content_fingerprint: Some(
                recite_core::producer_content_fingerprint(
                    "blake3",
                    "0000000000000000000000000000000000000000000000000000000000000000",
                )
                .expect("valid content fingerprint"),
            ),
            schema_export_version: Some(1),
            inclusion_policy: Some("dialogue-export-v1".to_owned()),
            producer_fingerprints: vec![ProducerFingerprint {
                id: "content/items".to_owned(),
                kind: "directory".to_owned(),
                algorithm: "blake3".to_owned(),
                value: "6f1d".to_owned(),
            }],
        })
    );
    assert_eq!(
        schema.registries["item"]
            .origin
            .as_ref()
            .expect("registry provenance")
            .id,
        "content/items/brass_key.item"
    );
    assert_eq!(
        schema.registries["item"]
            .origin
            .as_ref()
            .expect("registry provenance")
            .extensions["engine:resource_kind"],
        recite_core::ProducerMetadataValue::String("item".to_owned())
    );
    let MetadataDomainDefinition::Flat(domain) = &schema.metadata_domains["tone"] else {
        panic!("tone is a flat domain");
    };
    assert_eq!(domain.provenance.value_origins["calm"].kind, "data_row");
    let MetadataDomainDefinition::Contextual(domain) = &schema.metadata_domains["tone_by_speaker"]
    else {
        panic!("tone_by_speaker is contextual");
    };
    assert_eq!(domain.provenance.context_origins["rhea"].kind, "data_row");
    assert_eq!(
        domain.provenance.value_origins["rhea"]["calm"].kind,
        "data_cell"
    );
    assert_eq!(domain.provenance.producer_fingerprints.len(), 1);
    assert_eq!(
        schema.presentation_projectors["choice_skill_prefix"].inputs[2].type_ref,
        SchemaTypeRef::Array(Box::new(SchemaTypeRef::Symbol))
    );
}

#[test]
fn missing_and_empty_availability_reason_sections_lower_to_empty_maps() {
    let missing = load_schema_manifest_str(
        "fixtures/schema/valid/no_availability_reasons.json",
        r#"{
  "schema_version": 1
}"#,
    );
    assert_eq!(diagnostic_codes(&missing), Vec::<&str>::new());
    assert!(
        missing
            .schema
            .expect("missing availability reason section is valid")
            .availability_reasons
            .is_empty()
    );

    let empty = load_schema_manifest_str(
        "fixtures/schema/valid/empty_availability_reasons.json",
        r#"{
  "schema_version": 1,
  "availability_reasons": {}
}"#,
    );
    assert_eq!(diagnostic_codes(&empty), Vec::<&str>::new());
    assert!(
        empty
            .schema
            .expect("empty availability reason section is valid")
            .availability_reasons
            .is_empty()
    );
}

#[test]
fn availability_reason_literals_load_into_canonical_schema() {
    let report = load_schema_manifest_str(
        "fixtures/schema/valid/availability_reason_literals.json",
        r#"{
  "schema_version": 1,
  "types": {
    "mood": { "kind": "enum", "values": ["sad"] }
  },
  "registries": {
    "actor": { "values": ["hazel"] }
  },
  "speakers": {
    "rhea": {}
  },
  "conditions": {
    "can_answer": {
      "availability_reason": {
        "reason": "answer_blocked",
        "args": {
          "actor": "hazel",
          "speaker": "rhea",
          "mood": "sad",
          "count": 3,
          "weight": 1.5,
          "enabled": true
        }
      }
    }
  },
  "availability_reasons": {
    "answer_blocked": {
      "template": "{actor} {speaker} {mood} {count} {weight} {enabled}",
      "params": [
        { "name": "actor", "type": "registry:actor" },
        { "name": "speaker", "type": "speaker" },
        { "name": "mood", "type": "enum:mood" },
        { "name": "count", "type": "int" },
        { "name": "weight", "type": "float" },
        { "name": "enabled", "type": "bool" }
      ]
    }
  }
}"#,
    );

    assert_eq!(diagnostic_codes(&report), Vec::<&str>::new());
    let schema = report.schema.expect("valid reason literals");
    let args = &schema.conditions["can_answer"]
        .availability_reason
        .as_ref()
        .expect("mapping")
        .args;
    assert_eq!(
        args["actor"],
        AvailabilityReasonArgBinding::Literal(SchemaLiteralValue::String("hazel".to_owned()))
    );
    assert_eq!(
        args["count"],
        AvailabilityReasonArgBinding::Literal(SchemaLiteralValue::Int(3))
    );
    assert_eq!(
        args["weight"],
        AvailabilityReasonArgBinding::Literal(SchemaLiteralValue::Float("1.5".to_owned()))
    );
    assert_eq!(
        args["enabled"],
        AvailabilityReasonArgBinding::Literal(SchemaLiteralValue::Bool(true))
    );
}

#[test]
fn manifest_type_refs_support_the_issue_52_surface_exactly() {
    let report = load_schema_manifest_str(
        "fixtures/schema/valid/generated_manifest.json",
        include_str!("../../../../fixtures/schema/valid/generated_manifest.json"),
    );
    let schema = report.schema.expect("valid schema manifest");

    assert_eq!(
        schema.effects["advance_thread"].params[0].type_ref,
        SchemaTypeRef::Registry("thread".to_owned())
    );
    assert_eq!(
        schema.effects["advance_thread"].params[1].type_ref,
        SchemaTypeRef::Enum("thread_stage_kind".to_owned())
    );
    assert_eq!(
        schema.conditions["trust_gte"].params[0].type_ref,
        SchemaTypeRef::Speaker
    );
}

#[test]
fn metadata_type_refs_support_symbol() {
    let report = load_schema_manifest_str(
        "fixtures/schema/valid/symbol_metadata_type.json",
        r#"{
  "schema_version": 1,
  "metadata": {
    "route": {
      "targets": ["line"],
      "type": "symbol"
    }
  }
}"#,
    );
    assert_eq!(diagnostic_codes(&report), Vec::<&str>::new());
    let schema = report.schema.expect("valid symbol metadata schema");
    assert_eq!(schema.metadata["route"].type_ref, SchemaTypeRef::Symbol);
}

#[test]
fn metadata_domains_load_into_canonical_schema() {
    let report = load_schema_manifest_str(
        "fixtures/schema/valid/metadata_domains.json",
        r#"{
  "schema_version": 1,
  "metadata_domains": {
    "portrait_all": {
      "kind": "flat",
      "values": ["flat", "concerned", "wry"]
    },
    "portrait_by_speaker": {
      "kind": "contextual",
      "selector": "field:speaker",
      "values_by_context": {
        "rhea": ["flat", "concerned"],
        "hazel": ["flat", "wry"]
      },
      "missing_context": { "policy": "fallback", "domain": "portrait_all" }
    }
  },
  "metadata": {
    "portrait": {
      "targets": ["line"],
      "type": "symbol",
      "domain": "portrait_by_speaker"
    }
  }
}"#,
    );

    assert_eq!(diagnostic_codes(&report), Vec::<&str>::new());
    let schema = report.schema.expect("valid metadata domain schema");
    assert_eq!(
        schema.metadata["portrait"].domain.as_deref(),
        Some("portrait_by_speaker")
    );
    let MetadataDomainDefinition::Contextual(domain) =
        &schema.metadata_domains["portrait_by_speaker"]
    else {
        panic!("portrait_by_speaker should be contextual");
    };
    assert_eq!(domain.selector, MetadataContextSelector::FieldSpeaker);
    assert_eq!(
        domain.missing_context,
        MissingMetadataContextPolicy::Fallback {
            domain: "portrait_all".to_owned()
        }
    );
}

#[test]
fn presentation_projection_declarations_load_into_canonical_schema() {
    let report = load_schema_manifest_str(
        "fixtures/schema/valid/presentation_projection.json",
        r#"{
  "schema_version": 1,
  "metadata": {
    "skill": {
      "targets": ["choice"],
      "type": "string"
    },
    "threshold": {
      "targets": ["choice"],
      "type": "int"
    },
    "tag": {
      "targets": ["choice"],
      "type": "symbol",
      "repeatable": true
    }
  },
  "projection_queries": {
    "actor_skill": {
      "params": [{ "name": "skill", "type": "string" }],
      "returns": "int",
      "max_calls_per_event": 1
    }
  },
  "presentation_projectors": {
    "choice_skill_prefix": {
      "candidates": { "kind": "metadata_set", "target": "choice", "required_keys": ["skill", "threshold"] },
      "inputs": [
        { "name": "skill", "source": { "kind": "candidate_metadata", "key": "skill" }, "type": "string", "required": true },
        { "name": "threshold", "source": { "kind": "candidate_metadata", "key": "threshold" }, "type": "int", "required": true },
        { "name": "tags", "source": { "kind": "candidate_metadata", "key": "tag", "occurrence": "all" }, "type": "array:symbol" }
      ],
      "queries": {
        "current": { "function": "actor_skill", "args": [{ "input": "skill" }] }
      },
      "outputs": {
        "prefix": {
          "target": "candidate",
          "kind": "badge",
          "slot": "prefix",
          "label": {
            "template_id": "skill_check_prefix",
            "source_text": "[{skill} {current}/{threshold}]",
            "args": {
              "skill": { "source": { "input": "skill" }, "type": "string" },
              "current": { "source": { "query_result": "current" }, "type": "int" },
              "threshold": { "source": { "input": "threshold" }, "type": "int" }
            }
          },
          "fields": {
            "current": { "source": { "kind": "query_result", "name": "current" }, "type": "int" },
            "threshold": { "source": { "kind": "input", "name": "threshold" }, "type": "int" }
          }
        }
      }
    }
  }
}"#,
    );

    assert_eq!(diagnostic_codes(&report), Vec::<&str>::new());
    let schema = report.schema.expect("valid projection schema");
    assert_eq!(
        schema.projection_queries["actor_skill"].returns,
        SchemaTypeRef::Int
    );
    let projector = &schema.presentation_projectors["choice_skill_prefix"];
    assert_eq!(
        projector.candidates,
        SchemaProjectionSelector::MetadataSet {
            target: MetadataTarget::Choice,
            required_keys: vec!["skill".to_owned(), "threshold".to_owned()]
        }
    );
    assert_eq!(
        projector.inputs[2].source,
        SchemaProjectionInputSource::CandidateMetadata {
            key: "tag".to_owned(),
            occurrence: MetadataOccurrence::All
        }
    );
    assert_eq!(
        projector.inputs[2].type_ref,
        SchemaTypeRef::Array(Box::new(SchemaTypeRef::Symbol))
    );
    assert_eq!(
        projector.queries["current"].args,
        [ProjectionInputRef::Input {
            name: "skill".to_owned()
        }]
    );
    let output = &projector.outputs["prefix"];
    assert_eq!(output.target, ProjectionOutputTarget::Candidate);
    assert_eq!(
        output.label.as_ref().expect("label").template_id.as_str(),
        "skill_check_prefix"
    );
}
