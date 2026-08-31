use recite_core::{
    ContextualMetadataDomain, ContextualMetadataProvenance, MetadataContextSelector,
    MetadataDomainDefinition, MissingMetadataContextPolicy, ProducerOrigin, ProjectSchema,
    RegistryDefinition,
};
use tempfile::TempDir;

use recite_compiler::SchemaSummary;

use crate::workspace::WorkspaceConfig;

use super::super::support::{test_workspace, write_file};

pub(super) fn metadata_domain_schema_summary_preserves_available_provenance() {
    let mut schema = ProjectSchema::empty_v1();
    let origin = ProducerOrigin {
        kind: "asset_path".to_owned(),
        id: "data/portraits.toml".to_owned(),
        ..Default::default()
    };
    schema.registries.insert(
        "portrait".to_owned(),
        RegistryDefinition {
            values: ["smile".to_owned()].into_iter().collect(),
            origin: Some(origin.clone()),
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
            provenance: ContextualMetadataProvenance {
                origin: Some(origin.clone()),
                ..Default::default()
            },
        }),
    );

    let summary = SchemaSummary::from_schema(&schema);
    assert_eq!(summary.registries()[0].origin(), Some(&origin));
    let domain = &summary.metadata_domains()[0];
    assert_eq!(domain.provenance().origin(), Some(&origin));
    assert_eq!(
        domain.selector(),
        Some(&MetadataContextSelector::FieldSpeaker)
    );
    assert_eq!(
        domain.missing_context(),
        Some(&MissingMetadataContextPolicy::Diagnostic)
    );
}

pub(super) fn projection_schema_summary_exposes_queries_projectors_and_labels() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let path = temp.path().join("schema.json");
    write_file(temp.path(), "schema.json", valid_projection_schema());
    let workspace = test_workspace(WorkspaceConfig::for_roots(Vec::new()).with_schema_path(path));
    let summary = workspace.schema().summary().expect("schema summary");

    assert_eq!(summary.projection_queries()[0].name(), "actor_skill");
    assert_eq!(
        summary.projection_queries()[0].returns(),
        &recite_core::SchemaTypeRef::Int
    );
    let projector = &summary.presentation_projectors()[0];
    assert_eq!(projector.name(), "choice_skill_prefix");
    assert_eq!(projector.inputs()[0].name, "skill");
    assert_eq!(
        projector.queries().keys().next().map(String::as_str),
        Some("current")
    );
    assert_eq!(
        projector
            .labels()
            .next()
            .map(|(_, label)| label.template_id.as_str()),
        Some("skill_check_prefix")
    );
}

pub(super) fn schema_summary_preserves_source_ownership_and_generated_read_only_state() {
    let toml_temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(
        toml_temp.path(),
        "schema.toml",
        "schema_version = 1\n[producer]\nid = \"standalone\"\n",
    );
    let toml_workspace = test_workspace(
        WorkspaceConfig::for_roots(Vec::new())
            .with_schema_path(toml_temp.path().join("schema.toml")),
    );
    let toml_summary = toml_workspace.schema().summary().expect("TOML summary");
    assert!(toml_summary.ownership().is_standalone());
    assert!(
        toml_summary.capability().actions().iter().any(|action| {
            matches!(action, recite_compiler::SchemaAction::EditStandaloneSource)
        })
    );

    let json_temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(
        json_temp.path(),
        "schema.json",
        "{\"schema_version\":1,\"producer\":{\"kind\":\"adapter\",\"id\":\"generated\"}}\n",
    );
    let json_workspace = test_workspace(
        WorkspaceConfig::for_roots(Vec::new())
            .with_schema_path(json_temp.path().join("schema.json")),
    );
    let json_summary = json_workspace.schema().summary().expect("JSON summary");
    assert!(json_summary.ownership().is_generated());
    assert!(json_summary.capability().is_read_only());
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
      "inputs": [{ "name": "skill", "source": { "kind": "candidate_metadata", "key": "skill" }, "type": "string" }],
      "queries": { "current": { "function": "actor_skill", "args": [{ "input": "skill" }] } },
      "outputs": {
        "prefix": {
          "target": "candidate", "kind": "badge", "slot": "prefix",
          "label": {
            "template_id": "skill_check_prefix", "source_text": "[{skill}]",
            "args": { "skill": { "source": { "input": "skill" }, "type": "string" } }
          }
        }
      }
    }
  }
}"#
}
