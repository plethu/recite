use recite_core::{Diagnostic, DiagnosticSeverity, DocumentKey};

use super::super::SnapshotGeneration;
use super::fingerprints::{BuildFingerprintSet, BuildInputFingerprint, default_fingerprints};
use super::freshness::{AffectedInput, RestartGuidance};
use super::identity::{
    BuildGeneration, BuildInput, BuildInputAuthority, BuildInputKind, BuildInputPayload,
};
use super::request_identity::BuildRequestIdentity;

/// A complete canonical build request.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct BuildRequest {
    generation: BuildGeneration,
    snapshot_generation: SnapshotGeneration,
    policy: super::identity::BuildInputPolicy,
    inputs: Vec<BuildInput>,
    fingerprints: BuildFingerprintSet,
    restart: RestartGuidance,
}

impl BuildRequest {
    pub fn new(
        generation: BuildGeneration,
        snapshot_generation: SnapshotGeneration,
        inputs: impl IntoIterator<Item = BuildInput>,
    ) -> Result<Self, BuildRequestError> {
        Self::new_with_policy(
            generation,
            snapshot_generation,
            inputs,
            super::identity::BuildInputPolicy::SavedOnly,
        )
    }
    pub fn new_with_policy(
        generation: BuildGeneration,
        snapshot_generation: SnapshotGeneration,
        inputs: impl IntoIterator<Item = BuildInput>,
        policy: super::identity::BuildInputPolicy,
    ) -> Result<Self, BuildRequestError> {
        let mut inputs = inputs.into_iter().collect::<Vec<_>>();
        for input in &inputs {
            if (input.kind() == &BuildInputKind::Schema)
                != matches!(input.payload(), BuildInputPayload::Schema(_))
            {
                return Err(BuildRequestError::SchemaPayloadMismatch {
                    key: input.key().clone(),
                });
            }
        }
        if policy == super::identity::BuildInputPolicy::SavedOnly
            && let Some(input) = inputs
                .iter()
                .find(|input| input.authority == BuildInputAuthority::Overlay)
        {
            return Err(BuildRequestError::OverlayNotAllowed {
                key: input.key.clone(),
            });
        }
        inputs.sort_by(|left, right| {
            left.key
                .cmp(&right.key)
                .then(left.kind.cmp(&right.kind))
                .then(left.authority.cmp(&right.authority))
        });
        let mut effective = Vec::with_capacity(inputs.len());
        for input in inputs {
            if let Some(previous) = effective.iter_mut().find(|previous: &&mut BuildInput| {
                previous.key == input.key && previous.kind == input.kind
            }) {
                if input.authority == BuildInputAuthority::Overlay
                    && previous.authority == BuildInputAuthority::Saved
                {
                    *previous = input;
                } else {
                    return Err(BuildRequestError::DuplicateInput {
                        key: input.key,
                        kind: input.kind,
                    });
                }
            } else {
                effective.push(input);
            }
        }
        let schema_count = effective
            .iter()
            .filter(|input| input.kind() == &BuildInputKind::Schema)
            .count();
        if schema_count > 1 {
            return Err(BuildRequestError::MultipleSchemaInputs);
        }
        Ok(Self {
            generation,
            snapshot_generation,
            policy,
            fingerprints: default_fingerprints(&effective),
            inputs: effective,
            restart: RestartGuidance::NotApplicable,
        })
    }
    #[must_use]
    pub const fn with_restart_guidance(mut self, restart: RestartGuidance) -> Self {
        self.restart = restart;
        self
    }
    #[must_use]
    pub const fn generation(&self) -> BuildGeneration {
        self.generation
    }
    #[must_use]
    pub const fn snapshot_generation(&self) -> SnapshotGeneration {
        self.snapshot_generation
    }
    #[must_use]
    pub const fn input_policy(&self) -> super::identity::BuildInputPolicy {
        self.policy
    }
    #[must_use]
    pub fn inputs(&self) -> &[BuildInput] {
        &self.inputs
    }
    #[must_use]
    pub fn affected_inputs(&self) -> Vec<AffectedInput> {
        self.inputs
            .iter()
            .map(|input| AffectedInput::new(BuildInputFingerprint::from_input(input)))
            .collect()
    }
    #[must_use]
    pub const fn fingerprints(&self) -> &BuildFingerprintSet {
        &self.fingerprints
    }
    #[must_use]
    pub const fn restart_guidance(&self) -> RestartGuidance {
        self.restart
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum BuildRequestError {
    #[error("unsaved overlay {key} requires explicit build input policy")]
    OverlayNotAllowed { key: DocumentKey },
    #[error("build input {key} of kind {kind:?} was supplied more than once")]
    DuplicateInput {
        key: DocumentKey,
        kind: BuildInputKind,
    },
    #[error("schema input {key} must carry exactly one canonical schema payload")]
    SchemaPayloadMismatch { key: DocumentKey },
    #[error("a build request cannot contain multiple schema inputs")]
    MultipleSchemaInputs,
}

/// Validation and freshness output supplied by an injected check seam.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct BuildCheck {
    request_identity: BuildRequestIdentity,
    diagnostics: Vec<Diagnostic>,
    freshness: super::freshness::FreshnessAssessment,
}
impl BuildCheck {
    #[must_use]
    pub fn new(
        request: &BuildRequest,
        diagnostics: Vec<Diagnostic>,
        freshness: super::freshness::FreshnessAssessment,
    ) -> Self {
        Self {
            request_identity: BuildRequestIdentity::from_request(request),
            diagnostics,
            freshness,
        }
    }
    #[must_use]
    pub fn passed(request: &BuildRequest) -> Self {
        Self::new(
            request,
            Vec::new(),
            super::freshness::FreshnessAssessment::fresh(request.fingerprints().clone()),
        )
    }
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
    #[must_use]
    pub const fn freshness(&self) -> &super::freshness::FreshnessAssessment {
        &self.freshness
    }
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
    }
    pub(crate) fn validate_for(&self, request: &BuildRequest) -> Result<(), BuildCheckError> {
        if !self.request_identity.matches_request(request) {
            return Err(BuildCheckError::RequestMismatch);
        }
        if self.freshness.expected() != request.fingerprints() {
            return Err(BuildCheckError::FreshnessMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum BuildCheckError {
    #[error("build check identity does not match its request")]
    RequestMismatch,
    #[error("build check freshness does not match its request fingerprints")]
    FreshnessMismatch,
}
