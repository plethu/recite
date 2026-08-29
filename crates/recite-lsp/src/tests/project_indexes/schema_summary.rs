use recite_core::{
    ContextualMetadataDomain, FlatMetadataDomain, MetadataContextSelector,
    MetadataDomainDefinition, MissingMetadataContextPolicy, ProjectSchema, RegistryDefinition,
};
use tempfile::TempDir;

use crate::summary::{
    MetadataContextSelectorSummary, MetadataDomainKindSummary, MissingMetadataContextPolicySummary,
    ProvenanceSummary, SchemaSummary,
};
use crate::workspace::{LspWorkspace, WorkspaceConfig};

use super::super::support::write_file;

pub(super) fn metadata_domain_schema_summary_preserves_available_provenance() {
    let mut schema = ProjectSchema::empty_v1();
    schema.registries.insert(
        "portrait".to_owned(),
        RegistryDefinition {
            values: ["smile".to_owned()].into_iter().collect(),
            origin: Some(recite_core::ProducerOrigin {
                kind: "asset_path".to_owned(),
                id: "data/portraits.toml".to_owned(),
                label: None,
                ..Default::default()
            }),
            value_origins: [(
                "smile".to_owned(),
                recite_core::ProducerOrigin {
                    kind: "asset_row".to_owned(),
                    id: "data/portraits.toml#smile".to_owned(),
                    label: None,
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        },
    );
    schema.metadata_domains.insert(
        "portrait_by_speaker".to_owned(),
        MetadataDomainDefinition::Contextual(ContextualMetadataDomain {
            selector: MetadataContextSelector::FieldSpeaker,
            values_by_context: [(
                "hazel".to_owned(),
                ["smile".to_owned()].into_iter().collect(),
            )]
            .into_iter()
            .collect(),
            missing_context: MissingMetadataContextPolicy::Diagnostic,
            provenance: recite_core::ContextualMetadataProvenance {
                origin: Some(recite_core::ProducerOrigin {
                    kind: "asset_path".to_owned(),
                    id: "data/portraits.toml#portrait_by_speaker".to_owned(),
                    label: None,
                    ..Default::default()
                }),
                ..Default::default()
            },
        }),
    );
    schema.metadata_domains.insert(
        "stage_by_tone".to_owned(),
        MetadataDomainDefinition::Contextual(ContextualMetadataDomain {
            selector: MetadataContextSelector::MetadataKey("tone".to_owned()),
            values_by_context: [(
                "warm".to_owned(),
                ["market".to_owned()].into_iter().collect(),
            )]
            .into_iter()
            .collect(),
            missing_context: MissingMetadataContextPolicy::Fallback {
                domain: "tone".to_owned(),
            },
            provenance: Default::default(),
        }),
    );
    schema.metadata_domains.insert(
        "tone".to_owned(),
        MetadataDomainDefinition::Flat(FlatMetadataDomain {
            values: ["warm".to_owned()].into_iter().collect(),
            provenance: recite_core::FlatMetadataProvenance {
                value_origins: [(
                    "warm".to_owned(),
                    recite_core::ProducerOrigin {
                        kind: "asset_path".to_owned(),
                        id: "data/tone.toml#warm".to_owned(),
                        label: None,
                        ..Default::default()
                    },
                )]
                .into_iter()
                .collect(),
                ..Default::default()
            },
        }),
    );

    let summary = SchemaSummary::from_schema(&schema);

    assert_eq!(
        summary.registries[0].provenance,
        ProvenanceSummary::Present {
            origin: recite_core::ProducerOrigin {
                kind: "asset_path".to_owned(),
                id: "data/portraits.toml".to_owned(),
                label: None,
                ..Default::default()
            }
        }
    );
    assert!(matches!(
        summary.registries[0].value_provenance.get("smile"),
        Some(ProvenanceSummary::Present { .. })
    ));
    let speaker_domain = summary
        .metadata_domains
        .iter()
        .find(|domain| domain.name == "portrait_by_speaker")
        .expect("portrait_by_speaker metadata domain");
    assert_eq!(
        speaker_domain.provenance,
        ProvenanceSummary::Present {
            origin: recite_core::ProducerOrigin {
                kind: "asset_path".to_owned(),
                id: "data/portraits.toml#portrait_by_speaker".to_owned(),
                label: None,
                ..Default::default()
            }
        }
    );
    match &speaker_domain.kind {
        MetadataDomainKindSummary::Contextual {
            selector,
            values_by_context,
            missing_context,
        } => {
            assert_eq!(selector, &MetadataContextSelectorSummary::FieldSpeaker);
            assert_eq!(values_by_context[0].context, "hazel");
            assert_eq!(values_by_context[0].values, ["smile"]);
            assert_eq!(
                missing_context,
                &MissingMetadataContextPolicySummary::Diagnostic
            );
        }
        other => panic!("unexpected metadata domain summary: {other:?}"),
    }
    assert!(speaker_domain.contextual_value_provenance.is_empty());

    let tone_domain = summary
        .metadata_domains
        .iter()
        .find(|domain| domain.name == "stage_by_tone")
        .expect("stage_by_tone metadata domain");
    let tone_values = summary
        .metadata_domains
        .iter()
        .find(|domain| domain.name == "tone")
        .expect("tone metadata domain");
    assert!(matches!(
        tone_values.flat_value_provenance.get("warm"),
        Some(ProvenanceSummary::Present { .. })
    ));
    match &tone_domain.kind {
        MetadataDomainKindSummary::Contextual {
            selector,
            values_by_context,
            missing_context,
        } => {
            assert_eq!(
                selector,
                &MetadataContextSelectorSummary::MetadataKey {
                    key: "tone".to_owned()
                }
            );
            assert_eq!(values_by_context[0].context, "warm");
            assert_eq!(values_by_context[0].values, ["market"]);
            assert_eq!(
                missing_context,
                &MissingMetadataContextPolicySummary::Fallback {
                    domain: "tone".to_owned()
                }
            );
        }
        other => panic!("unexpected metadata domain summary: {other:?}"),
    }
}

pub(super) fn projection_schema_summary_exposes_queries_projectors_and_labels() {
    let (_temp, schema_path) = write_schema_temp(valid_projection_schema());
    let workspace =
        LspWorkspace::new(WorkspaceConfig::for_roots(Vec::new()).with_schema_path(schema_path));
    let summary = workspace.schema().summary().expect("schema summary");

    assert_eq!(summary.projection_queries[0].name, "actor_skill");
    assert_eq!(summary.projection_queries[0].returns, "int");
    assert_eq!(
        summary.presentation_projectors[0].name,
        "choice_skill_prefix"
    );
    assert_eq!(summary.presentation_projectors[0].inputs[0].name, "skill");
    assert_eq!(
        summary.presentation_projectors[0].queries[0].name,
        "current"
    );
    assert_eq!(summary.presentation_projectors[0].outputs[0].name, "prefix");
    assert_eq!(
        summary.presentation_projectors[0].outputs[0].label_template,
        Some("skill_check_prefix".to_owned())
    );
}

fn write_schema_temp(source: &str) -> (TempDir, std::path::PathBuf) {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let path = temp.path().join("schema.json");
    write_file(temp.path(), "schema.json", source);
    (temp, path)
}

pub(super) fn valid_projection_schema() -> &'static str {
    r#"{
  "schema_version": 1,
  "metadata": {
    "skill": { "targets": ["choice"], "type": "string" },
    "threshold": { "targets": ["choice"], "type": "int" }
  },
  "projection_queries": {
    "actor_skill": {
      "params": [{ "name": "skill", "type": "string" }],
      "returns": "int"
    }
  },
  "presentation_projectors": {
    "choice_skill_prefix": {
      "candidates": { "kind": "metadata_set", "target": "choice", "required_keys": ["skill", "threshold"] },
      "inputs": [
        { "name": "skill", "source": { "kind": "candidate_metadata", "key": "skill" }, "type": "string" },
        { "name": "threshold", "source": { "kind": "candidate_metadata", "key": "threshold" }, "type": "int" }
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
          }
        }
      }
    }
  }
}"#
}
