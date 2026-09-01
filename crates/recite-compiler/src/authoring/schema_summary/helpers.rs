use recite_core::{MetadataDomainDefinition, ProducerFingerprint, ProducerOrigin, ProjectSchema};

use super::evidence::{ProducerCapabilityStatus, SchemaSummaryEvidence};
use super::identity::{
    SchemaAction, SchemaCapability, SchemaCapabilityUnavailableReason, SchemaDeclarationProvenance,
    SchemaFreshness, SchemaFreshnessUnavailableReason, SchemaOwnership,
};
use super::producer::{
    ProducerActionDescriptor, ProducerActionEvidence, ProducerActionRequest, ProducerLaunchSnapshot,
};

pub(super) fn ownership(schema: &ProjectSchema, source_owned: bool) -> SchemaOwnership {
    match schema
        .producer_metadata
        .as_ref()
        .and_then(|metadata| metadata.producer.as_ref())
    {
        Some(producer) if source_owned => SchemaOwnership::Standalone {
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
    source_owned: bool,
    evidence: Option<&SchemaSummaryEvidence>,
    producer_expected: Option<&ProducerActionEvidence>,
    producer_launch: Option<&ProducerLaunchSnapshot>,
) -> SchemaCapability {
    let (actions, producer_actions) = match ownership {
        SchemaOwnership::Standalone { .. } if source_owned => {
            let mut actions = Vec::new();
            if has_explicit_origin {
                actions.push(SchemaAction::OpenSourceDeclaration);
            }
            actions.push(SchemaAction::EditStandaloneSource);
            (actions, Vec::new())
        }
        SchemaOwnership::Generated { producer } => {
            let mut actions = Vec::new();
            let mut producer_actions = Vec::new();
            if has_explicit_origin {
                actions.push(SchemaAction::OpenSourceDeclaration);
            }
            let status = evidence.and_then(SchemaSummaryEvidence::capability);
            match status {
                Some(ProducerCapabilityStatus::Supported) => {
                    actions.push(SchemaAction::InvokeProducer {
                        producer: producer.clone(),
                    });
                    if let (Some(expected), Some(launch)) = (producer_expected, producer_launch)
                        && let Ok(request) =
                            ProducerActionRequest::regenerate(expected.clone(), launch.clone())
                    {
                        producer_actions.push(ProducerActionDescriptor::new(request));
                    }
                    if let (Some(failed_result), Some(launch)) = (
                        evidence.and_then(SchemaSummaryEvidence::failed_result),
                        producer_launch,
                    ) && let Ok(request) =
                        failed_result.retry_request_with_launch(launch.clone())
                    {
                        actions.push(SchemaAction::RetryProducerFailure {
                            producer: producer.clone(),
                        });
                        producer_actions.push(ProducerActionDescriptor::new(request));
                    }
                }
                Some(ProducerCapabilityStatus::Unavailable) => {
                    actions.push(SchemaAction::Unavailable {
                        reason: SchemaCapabilityUnavailableReason::ProducerCapabilityUnavailable,
                    });
                }
                Some(ProducerCapabilityStatus::ReadOnly) | None => {
                    actions.push(SchemaAction::ReadOnlyGenerated);
                }
            }
            (actions, producer_actions)
        }
        SchemaOwnership::Standalone { .. } | SchemaOwnership::Unavailable => (
            vec![SchemaAction::Unavailable {
                reason: SchemaCapabilityUnavailableReason::UnknownSourceOwner,
            }],
            Vec::new(),
        ),
    };
    SchemaCapability {
        actions,
        producer_actions,
    }
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
