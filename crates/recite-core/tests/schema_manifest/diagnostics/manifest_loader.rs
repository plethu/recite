use recite_core::load_schema_manifest_str;

use crate::diagnostic_codes;

#[test]
fn manifest_loader_rejects_metadata_only_symbol_for_parameters() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/metadata_only_symbol_for_parameters.json",
        r#"{
  "schema_version": 1,
  "conditions": {
    "can_talk": {
      "params": [{ "name": "who", "type": "symbol" }]
    }
  },
  "effects": {
    "play_bark": {
      "modes": ["immediate"],
      "params": [{ "name": "who", "type": "symbol" }]
    }
  }
}"#,
    );

    assert!(report.schema.is_none());
    assert_eq!(
        diagnostic_codes(&report),
        ["RECITE_SCHEMA004", "RECITE_SCHEMA004"]
    );
}

#[test]
fn malformed_metadata_domains_report_schema_diagnostics() {
    let duplicate_values = load_schema_manifest_str(
        "fixtures/schema/invalid/duplicate_metadata_domain_values.json",
        r#"{
  "schema_version": 1,
  "metadata_domains": {
    "portrait": {
      "kind": "flat",
      "values": ["flat", "flat"]
    }
  }
}"#,
    );
    assert!(duplicate_values.schema.is_none());
    assert_eq!(diagnostic_codes(&duplicate_values), ["RECITE_SCHEMA003"]);

    let invalid_refs = load_schema_manifest_str(
        "fixtures/schema/invalid/invalid_metadata_domain_references.json",
        r#"{
  "schema_version": 1,
  "metadata_domains": {
    "by_speaker": {
      "kind": "contextual",
      "selector": "field:speaker",
      "values_by_context": { "rhea": ["flat"] },
      "missing_context": { "policy": "fallback", "domain": "missing" }
    }
  },
  "metadata": {
    "portrait": {
      "targets": ["line"],
      "type": "symbol",
      "domain": "unknown"
    },
    "caption": {
      "targets": ["line"],
      "type": "string",
      "domain": "by_speaker"
    }
  }
}"#,
    );
    assert!(invalid_refs.schema.is_none());
    assert_eq!(
        diagnostic_codes(&invalid_refs),
        ["RECITE_SCHEMA001", "RECITE_SCHEMA004", "RECITE_SCHEMA004"]
    );

    let missing_required_domain_fields = load_schema_manifest_str(
        "fixtures/schema/invalid/missing_metadata_domain_fields.json",
        r#"{
  "schema_version": 1,
  "metadata_domains": {
    "flat_without_values": { "kind": "flat" },
    "contextual_without_values": {
      "kind": "contextual",
      "selector": "field:speaker"
    }
  }
}"#,
    );
    assert!(missing_required_domain_fields.schema.is_none());
    assert_eq!(
        diagnostic_codes(&missing_required_domain_fields),
        ["RECITE_SCHEMA001", "RECITE_SCHEMA001"]
    );

    let invalid_missing_context_policy = load_schema_manifest_str(
        "fixtures/schema/invalid/invalid_metadata_domain_policy.json",
        r#"{
  "schema_version": 1,
  "metadata_domains": {
    "portrait_all": {
      "kind": "flat",
      "values": ["flat"]
    },
    "portrait_by_speaker": {
      "kind": "contextual",
      "selector": "field:speaker",
      "values_by_context": { "rhea": ["flat"] },
      "missing_context": { "policy": "portrait_all" }
    }
  }
}"#,
    );
    assert!(invalid_missing_context_policy.schema.is_none());
    assert_eq!(
        diagnostic_codes(&invalid_missing_context_policy),
        ["RECITE_SCHEMA001"]
    );
}

#[test]
fn manifest_loader_rejects_strings_rejected_by_the_public_json_schema_contract() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/schema_contract_drift.json",
        r#"{
  "schema_version": 1,
  "types": {
    "state": { "kind": "enum", "values": [""] }
  },
  "registries": {
    "sound": { "values": ["snap"], "origin": "" }
  },
  "speakers": {
    "hazel": { "display_name": "" }
  },
  "conditions": {
    "bad_condition": {
      "params": [{ "name": "", "type": "enum:bad space" }],
      "returns": "enum:"
    }
  },
  "metadata": {
    "bad_metadata": { "targets": ["line"], "type": "registry:" }
  }
}"#,
    );

    assert!(report.schema.is_none());
    assert_eq!(
        diagnostic_codes(&report),
        [
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA004",
            "RECITE_SCHEMA004",
            "RECITE_SCHEMA004"
        ]
    );
}

#[test]
fn manifest_loader_rejects_invalid_definition_and_parameter_names() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/invalid_definition_names.json",
        r#"{
  "schema_version": 1,
  "types": {
    "": { "kind": "enum", "values": ["fresh"] },
    "bad name": { "kind": "enum", "values": ["fresh"] },
    "1bad": { "kind": "enum", "values": ["fresh"] }
  },
  "conditions": {
    "trust_gte": {
      "params": [
        { "name": "bad name", "type": "speaker" }
      ]
    }
  }
}"#,
    );

    assert!(report.schema.is_none());
    assert_eq!(
        diagnostic_codes(&report),
        [
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA001"
        ]
    );
    assert_eq!(report.diagnostics[0].span.start.line(), 4);
    assert_eq!(report.diagnostics[1].span.start.line(), 5);
    assert_eq!(report.diagnostics[2].span.start.line(), 6);
    assert_eq!(report.diagnostics[3].span.start.line(), 11);
}

#[test]
fn manifest_loader_rejects_missing_required_values_fields() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/missing_required_values.json",
        r#"{
  "schema_version": 1,
  "types": {
    "state": { "kind": "enum" }
  }
}"#,
    );

    assert!(report.schema.is_none());
    assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA001"]);
}

#[test]
fn manifest_loader_rejects_explicit_null_optionals() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/null_optionals.json",
        r#"{
  "schema_version": 1,
  "registries": {
    "sound": { "values": ["snap"], "origin": null }
  }
}"#,
    );

    assert!(report.schema.is_none());
    assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA001"]);
}

#[test]
fn manifest_loader_rejects_invalid_projection_declarations() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/presentation_projection.json",
        r#"{
  "schema_version": 1,
  "metadata": {
    "skill": {
      "targets": ["line"],
      "type": "string"
    },
    "tag": {
      "targets": ["choice"],
      "type": "symbol"
    }
  },
  "projection_queries": {
    "actor_skill": {
      "params": [{ "name": "skill", "type": "string" }],
      "returns": "int"
    }
  },
  "presentation_projectors": {
    "choice_skill_prefix": {
      "candidates": { "kind": "metadata_key", "target": "choice", "key": "skill" },
      "inputs": [
        { "name": "skill", "source": { "kind": "candidate_metadata", "key": "skill" }, "type": "int" },
        { "name": "tags", "source": { "kind": "candidate_metadata", "key": "tag", "occurrence": "all" }, "type": "symbol" }
      ],
      "queries": {
        "current": { "function": "missing", "args": [{ "input": "skill" }] }
      },
      "outputs": {
        "prefix": {
          "target": "candidate",
          "kind": "badge",
          "slot": "prefix",
          "label": {
            "template_id": "skill_check_prefix",
            "source_text": "[{skill} {current}]",
            "args": {
              "skill": { "source": { "input": "skill" }, "type": "string" }
            }
          },
          "fields": {
            "current": { "source": { "kind": "query_result", "name": "current" }, "type": "int" }
          }
        }
      }
    }
  }
}"#,
    );

    assert!(report.schema.is_none());
    assert_eq!(
        diagnostic_codes(&report),
        [
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA004",
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA004"
        ]
    );
}

#[test]
fn manifest_loader_rejects_projection_all_with_wrong_array_inner_type() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/projection_all_wrong_array_inner.json",
        r#"{
  "schema_version": 1,
  "metadata": {
    "tag": {
      "targets": ["choice"],
      "type": "symbol",
      "repeatable": true
    }
  },
  "presentation_projectors": {
    "choice_tags": {
      "candidates": { "kind": "metadata_key", "target": "choice", "key": "tag" },
      "inputs": [
        { "name": "tags", "source": { "kind": "candidate_metadata", "key": "tag", "occurrence": "all" }, "type": "array:int" }
      ]
    }
  }
}"#,
    );

    assert!(report.schema.is_none());
    assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA001"]);
}
