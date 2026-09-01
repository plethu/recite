#![cfg(test)]

use std::collections::BTreeSet;

use recite_compiler::{
    SchemaAction, SchemaCapabilityUnavailableReason, SchemaFreshness,
    SchemaFreshnessUnavailableReason, SchemaOwnership, SchemaSummary,
};
use recite_core::{
    ConditionReturnType, EffectMode, MetadataContextSelector, MissingMetadataContextPolicy,
    SchemaTypeDefinition, SchemaTypeRef, load_schema_manifest_str, load_schema_source_str,
};

const STANDALONE: &str = include_str!("../../../fixtures/schema/valid/standalone.toml");
const GENERATED: &str = include_str!("../../../fixtures/schema/valid/full_manifest.json");

#[test]
fn standalone_fixture_projects_exact_typed_declarations_in_stable_order() {
    let report = load_schema_source_str("standalone.toml", STANDALONE);
    assert!(
        report.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        report.diagnostics
    );
    let source = report.source.expect("standalone fixture lowers");
    let summary = SchemaSummary::from_source(&source);

    assert_eq!(summary.schema_version(), 1);
    assert!(
        matches!(summary.ownership(), SchemaOwnership::Standalone { producer } if producer.id() == "standalone-example")
    );
    assert!(!summary.source().generated_output_is_read_only());
    assert_eq!(
        summary
            .types()
            .iter()
            .map(|item| item.name())
            .collect::<Vec<_>>(),
        ["actor_kind", "mood"]
    );
    assert!(
        matches!(summary.types()[1].definition(), SchemaTypeDefinition::Enum(definition) if definition.values == BTreeSet::from(["calm".to_owned(), "tense".to_owned()]))
    );
    assert_eq!(
        summary
            .speakers()
            .iter()
            .map(|item| item.name())
            .collect::<Vec<_>>(),
        ["hazel", "rhea"]
    );

    let condition = &summary.conditions()[0];
    assert_eq!(condition.name(), "can_open");
    assert_eq!(
        condition.params()[0].type_ref,
        SchemaTypeRef::Registry("item".to_owned())
    );
    assert!(matches!(condition.returns(), ConditionReturnType::Bool));
    let mapping = condition.availability_reason().expect("mapped reason");
    assert_eq!(mapping.reason.as_str(), "missing_key");

    let reason = &summary.availability_reasons()[0];
    assert_eq!(reason.id().as_str(), "missing_key");
    assert_eq!(reason.template(), "You need {item}.");
    assert_eq!(
        reason.params()[0].type_ref,
        SchemaTypeRef::Registry("item".to_owned())
    );
    let reason_origin = reason.origin().expect("reason provenance");
    assert_eq!(reason_origin.id, "schema/reasons.rs");
    assert!(
        reason
            .capability()
            .supports(&SchemaAction::OpenSourceDeclaration)
    );

    let effect = &summary.effects()[0];
    assert_eq!(effect.name(), "open");
    assert_eq!(
        effect.params()[0].type_ref,
        SchemaTypeRef::Registry("item".to_owned())
    );
    assert_eq!(
        effect.modes(),
        &BTreeSet::from([EffectMode::Immediate, EffectMode::Blocking])
    );

    assert_eq!(
        summary
            .metadata()
            .iter()
            .map(|item| item.name())
            .collect::<Vec<_>>(),
        ["skill", "threshold", "tone"]
    );
    assert_eq!(summary.metadata()[2].type_ref(), &SchemaTypeRef::Symbol);
    let contextual = summary
        .metadata_domains()
        .iter()
        .find(|domain| domain.name() == "tone_by_speaker")
        .expect("contextual domain")
        .contextual()
        .expect("contextual definition");
    assert_eq!(contextual.selector, MetadataContextSelector::FieldSpeaker);
    assert_eq!(
        contextual.values_by_context["rhea"],
        BTreeSet::from(["calm".to_owned()])
    );
    assert!(
        matches!(&contextual.missing_context, MissingMetadataContextPolicy::Fallback { domain } if domain == "tone")
    );

    assert_eq!(summary.projection_queries()[0].name(), "actor_skill");
    let projector = &summary.presentation_projectors()[0];
    assert_eq!(projector.name(), "choice_badge");
    assert_eq!(projector.inputs()[0].type_ref, SchemaTypeRef::String);
    assert!(projector.queries().contains_key("current"));
    assert!(projector.outputs()["badge"].label.is_some());

    assert!(summary.fingerprints().source_owned().is_some());
    assert_eq!(
        summary.fingerprints().producer_inputs()[0].id,
        "standalone-example"
    );
    assert!(matches!(
        summary.freshness(),
        SchemaFreshness::Unavailable {
            reason: SchemaFreshnessUnavailableReason::NoComparisonSnapshot
        }
    ));
}

#[test]
fn generated_fixture_is_typed_producer_evidence_and_read_only() {
    let report = load_schema_manifest_str("full_manifest.json", GENERATED);
    assert!(
        report.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        report.diagnostics
    );
    let schema = report.schema.expect("generated fixture lowers");
    let summary = SchemaSummary::from_schema(&schema);

    assert!(
        matches!(summary.ownership(), SchemaOwnership::Generated { producer } if producer.kind() == "adapter" && producer.id() == "example")
    );
    assert!(summary.source().generated_output_is_read_only());
    assert!(
        summary
            .capability()
            .supports(&SchemaAction::ReadOnlyGenerated)
    );
    assert!(
        !summary
            .capability()
            .actions()
            .iter()
            .any(|action| matches!(action, SchemaAction::InvokeProducer { .. }))
    );
    let producer_metadata = summary.producer_metadata().expect("producer metadata");
    assert_eq!(producer_metadata.schema_export_version(), Some(1));
    assert_eq!(
        producer_metadata.producer_fingerprints()[0].id,
        "content/items"
    );
    let registry_origin = summary.registries()[0]
        .origin()
        .expect("registry provenance");
    assert_eq!(registry_origin.id, "content/items/brass_key.item");
    let producer_content = summary
        .fingerprints()
        .producer_content()
        .expect("content fingerprint");
    assert_eq!(producer_content.algorithm().as_str(), "blake3");

    let exported = load_schema_source_str("standalone.toml", STANDALONE)
        .source
        .expect("source fixture")
        .export_json();
    let roundtrip = load_schema_manifest_str("standalone.json", &exported)
        .schema
        .expect("exported source reloads");
    let roundtrip_summary = SchemaSummary::from_schema(&roundtrip);
    assert!(matches!(
        roundtrip_summary.ownership(),
        SchemaOwnership::Generated { .. }
    ));
    assert!(roundtrip_summary.source().generated_output_is_read_only());
    assert!(
        !roundtrip_summary
            .capability()
            .actions()
            .iter()
            .any(|action| matches!(action, SchemaAction::EditStandaloneSource))
    );
}

#[test]
fn missing_producer_is_explicitly_unavailable_and_malformed_schema_stays_core_owned() {
    let summary = SchemaSummary::from_schema(&recite_core::ProjectSchema::empty_v1());
    assert!(matches!(summary.ownership(), SchemaOwnership::Unavailable));
    assert!(matches!(
        summary.capability().actions(),
        [SchemaAction::Unavailable {
            reason: SchemaCapabilityUnavailableReason::UnknownSourceOwner
        }]
    ));
    assert!(matches!(
        summary.freshness(),
        SchemaFreshness::Unavailable {
            reason: SchemaFreshnessUnavailableReason::NoProducerMetadata
        }
    ));

    let report = load_schema_manifest_str(
        "malformed_shape.json",
        include_str!("../../../fixtures/schema/invalid/malformed_shape.json"),
    );
    assert!(report.schema.is_none());
    assert!(!report.diagnostics.is_empty());
    let diagnostic_codes = report
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.to_string())
        .collect::<BTreeSet<_>>();
    assert!(
        diagnostic_codes
            .iter()
            .any(|code| code.starts_with("RECITE_SCHEMA"))
    );
}
