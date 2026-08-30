use recite_core::{Diagnostic, DiagnosticSeverity, DocumentKey};

use super::super::SnapshotGeneration;
use super::freshness::{AffectedInput, RestartGuidance};
use super::identity::{
    BuildFingerprintSet, BuildGeneration, BuildInput, BuildInputAuthority, BuildInputKind,
    default_fingerprints,
};

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
        if let Some(input) = effective
            .iter()
            .find(|input| input.kind == BuildInputKind::Schema && input.schema_model().is_none())
        {
            return Err(BuildRequestError::SchemaModelRequired {
                key: input.key.clone(),
            });
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
            .map(|input| {
                AffectedInput::new(super::identity::BuildInputFingerprint::from_input(input))
            })
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
    #[error("schema input {key} requires a parsed canonical schema model")]
    SchemaModelRequired { key: DocumentKey },
}

/// Validation and freshness output supplied by an injected check seam.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct BuildCheck {
    diagnostics: Vec<Diagnostic>,
    freshness: super::freshness::FreshnessAssessment,
}
impl BuildCheck {
    #[must_use]
    pub fn new(
        diagnostics: Vec<Diagnostic>,
        freshness: super::freshness::FreshnessAssessment,
    ) -> Self {
        Self {
            diagnostics,
            freshness,
        }
    }
    #[must_use]
    pub fn passed(freshness: super::freshness::FreshnessAssessment) -> Self {
        Self::new(Vec::new(), freshness)
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
}
