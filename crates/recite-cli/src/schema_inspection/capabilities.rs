use recite_compiler::{
    ProducerActionDescriptor, ProducerActionEvidence, ProducerActionOperation,
    ProducerRetryGuidance, SchemaAction, SchemaCapability,
};

use super::fingerprints::{content_fingerprint_json, schema_fingerprint_json, scopes_json};
use super::model::{
    CapabilityProjection, ProducerActionProjection, ProducerEvidenceProjection,
    ProducerFailureProjection, ProducerLaunchProjection, ProducerOperationProjection,
};
use super::provenance::identity_json;

pub(super) fn capability_json(capability: &SchemaCapability) -> CapabilityProjection {
    let mut actions = Vec::new();
    let mut unavailable_reasons = Vec::new();
    for action in capability.actions() {
        match action {
            SchemaAction::OpenSourceDeclaration => actions.push("open_source_declaration"),
            SchemaAction::EditStandaloneSource => actions.push("edit_standalone_source"),
            SchemaAction::InvokeProducer { .. } => actions.push("invoke_producer"),
            SchemaAction::RetryProducerFailure { .. } => actions.push("retry_producer_failure"),
            SchemaAction::ReadOnlyGenerated => actions.push("read_only_generated"),
            SchemaAction::Unavailable { reason } => {
                actions.push("unavailable");
                unavailable_reasons.push(match reason {
                    recite_compiler::SchemaCapabilityUnavailableReason::UnknownSourceOwner => {
                        "unknown_source_owner"
                    }
                    recite_compiler::SchemaCapabilityUnavailableReason::ProducerCapabilityUnavailable => {
                        "producer_capability_unavailable"
                    }
                    _ => "unknown",
                });
            }
            _ => actions.push("unknown"),
        }
    }
    CapabilityProjection {
        actions: actions.into_iter().map(str::to_owned).collect(),
        unavailable_reasons: unavailable_reasons.into_iter().map(str::to_owned).collect(),
        producer_actions: capability
            .producer_actions()
            .iter()
            .map(producer_action_json)
            .collect(),
    }
}

fn producer_action_json(descriptor: &ProducerActionDescriptor) -> ProducerActionProjection {
    let request = descriptor.request();
    ProducerActionProjection {
        request_id: content_fingerprint_json(request.identity().fingerprint()),
        producer: identity_json(request.producer()),
        operation: producer_operation_json(request.operation()),
        expected: producer_evidence_json(request.expected()),
        launch: ProducerLaunchProjection {
            producer: identity_json(request.launch_snapshot().producer()),
            input_fingerprints: scopes_json(request.launch_snapshot().input_fingerprints()),
        },
    }
}

fn producer_operation_json(operation: &ProducerActionOperation) -> ProducerOperationProjection {
    match operation {
        ProducerActionOperation::Regenerate => ProducerOperationProjection::Regenerate,
        ProducerActionOperation::Retry {
            failure,
            originating_request,
        } => ProducerOperationProjection::Retry {
            failure: ProducerFailureProjection {
                producer: identity_json(failure.producer()),
                code: failure.code().to_owned(),
                detail: failure.detail().map(str::to_owned),
                retry_guidance: retry_guidance(failure.retry_guidance()).to_owned(),
            },
            originating_request_id: content_fingerprint_json(originating_request.fingerprint()),
        },
        _ => ProducerOperationProjection::Unknown,
    }
}

fn retry_guidance(guidance: ProducerRetryGuidance) -> &'static str {
    match guidance {
        ProducerRetryGuidance::RetryNow => "retry_now",
        ProducerRetryGuidance::RetryAfterCorrection => "retry_after_correction",
        ProducerRetryGuidance::DoNotRetry => "do_not_retry",
        _ => "unknown",
    }
}

fn producer_evidence_json(evidence: &ProducerActionEvidence) -> ProducerEvidenceProjection {
    ProducerEvidenceProjection {
        schema_fingerprint: schema_fingerprint_json(evidence.schema_fingerprint()),
        content_fingerprint: content_fingerprint_json(evidence.content_fingerprint()),
        input_fingerprints: scopes_json(evidence.input_fingerprints()),
        output_fingerprint: evidence.output_fingerprint().map(content_fingerprint_json),
    }
}
