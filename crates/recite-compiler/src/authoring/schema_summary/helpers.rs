use recite_core::{MetadataDomainDefinition, ProducerFingerprint, ProducerOrigin, ProjectSchema};

use super::identity::{
    SchemaAction, SchemaCapability, SchemaCapabilityUnavailableReason, SchemaDeclarationProvenance,
    SchemaFreshness, SchemaFreshnessUnavailableReason, SchemaOwnership,
};

pub(super) fn ownership(schema: &ProjectSchema) -> SchemaOwnership {
    match schema
        .producer_metadata
        .as_ref()
        .and_then(|metadata| metadata.producer.as_ref())
    {
        Some(producer) if producer.kind() == "standalone" => SchemaOwnership::Standalone {
            producer: producer.clone(),
        },
        Some(producer) => SchemaOwnership::Generated {
            producer: producer.clone(),
        },
        None => SchemaOwnership::Unavailable,
    }
}

pub(super) fn provenance(
    ownership: &SchemaOwnership,
    origin: Option<&ProducerOrigin>,
) -> SchemaDeclarationProvenance {
    SchemaDeclarationProvenance {
        ownership: ownership.clone(),
        origin: origin.cloned(),
    }
}

pub(super) fn capability(
    ownership: &SchemaOwnership,
    has_explicit_origin: bool,
) -> SchemaCapability {
    let actions = match ownership {
        SchemaOwnership::Standalone { .. } => {
            let mut actions = Vec::new();
            if has_explicit_origin {
                actions.push(SchemaAction::OpenSourceDeclaration);
            }
            actions.push(SchemaAction::EditStandaloneSource);
            actions
        }
        SchemaOwnership::Generated { producer } => {
            let mut actions = Vec::new();
            if has_explicit_origin {
                actions.push(SchemaAction::OpenSourceDeclaration);
            }
            actions.extend([
                SchemaAction::InvokeProducer {
                    producer: producer.clone(),
                },
                SchemaAction::RetryProducerFailure {
                    producer: producer.clone(),
                },
                SchemaAction::ReadOnlyGenerated,
            ]);
            actions
        }
        SchemaOwnership::Unavailable => vec![SchemaAction::Unavailable {
            reason: SchemaCapabilityUnavailableReason::UnknownSourceOwner,
        }],
    };
    SchemaCapability { actions }
}

pub(super) fn freshness(has_producer_metadata: bool) -> SchemaFreshness {
    let reason = if has_producer_metadata {
        SchemaFreshnessUnavailableReason::NoComparisonSnapshot
    } else {
        SchemaFreshnessUnavailableReason::NoProducerMetadata
    };
    SchemaFreshness::Unavailable { reason }
}

pub(super) fn sorted_fingerprints(
    fingerprints: &[ProducerFingerprint],
) -> Vec<ProducerFingerprint> {
    let mut fingerprints = fingerprints.to_vec();
    fingerprints.sort();
    fingerprints
}

pub(super) fn domain_origin(definition: &MetadataDomainDefinition) -> Option<&ProducerOrigin> {
    match definition {
        MetadataDomainDefinition::Flat(domain) => domain.provenance.origin.as_ref(),
        MetadataDomainDefinition::Contextual(domain) => domain.provenance.origin.as_ref(),
    }
}
