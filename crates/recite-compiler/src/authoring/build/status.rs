use super::super::SnapshotGeneration;
use super::failure::BuildResultFailure;
use super::freshness::{AffectedInput, FreshnessAssessment, RestartGuidance};
use super::identity::BuildGeneration;
use super::lifecycle::{BuildPhase, BuildState};
use super::publish::{BuildCandidate, PublishOutcome};
use super::request_identity::BuildRequestIdentity;
use super::result::{BuildResult, BuildTerminalStatus};
use recite_core::Diagnostic;

/// An owned, transport-neutral view of one build lifecycle state.
///
/// This is a read projection of [`BuildState`] and [`BuildResult`]. It carries
/// the identity, inputs, diagnostics, outputs, freshness, restart guidance,
/// and publication outcome already established by the build lifecycle. It
/// performs no discovery, I/O, rendering, or protocol translation. The
/// projection owns its values so a host can retain or transport it without
/// retaining the mutable lifecycle that produced it.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct BuildStatusProjection {
    phase: BuildPhase,
    terminal_status: Option<BuildTerminalStatus>,
    request_identity: Option<BuildRequestIdentity>,
    affected_inputs: Vec<AffectedInput>,
    diagnostics: Vec<Diagnostic>,
    candidates: Vec<BuildCandidate>,
    freshness: Option<FreshnessAssessment>,
    publish: Option<PublishOutcome>,
    restart_guidance: Option<RestartGuidance>,
    failure: Option<BuildResultFailure>,
}

impl BuildStatusProjection {
    /// Project the supplied lifecycle state without changing its ordering or
    /// interpreting any host-specific transport concerns.
    #[must_use]
    pub fn from_state(state: &BuildState) -> Self {
        match state {
            BuildState::Idle => Self::idle(),
            BuildState::Checking { request } => {
                Self::active(BuildPhase::Checking, request, state.candidates())
            }
            BuildState::Building { request, .. } => {
                Self::active(BuildPhase::Building, request, state.candidates())
            }
            BuildState::Ready { request, .. } => {
                Self::active(BuildPhase::Ready, request, state.candidates())
            }
            BuildState::Publishing { request, .. } => {
                Self::active(BuildPhase::Publishing, request, state.candidates())
            }
            BuildState::Succeeded { result } => Self::terminal(BuildPhase::Succeeded, result),
            BuildState::Failed { result } => Self::terminal(BuildPhase::Failed, result),
            BuildState::Stale { result } => Self::terminal(BuildPhase::Stale, result),
            BuildState::Cancelled { result } => Self::terminal(BuildPhase::Cancelled, result),
            BuildState::Superseded { result } => Self::terminal(BuildPhase::Superseded, result),
        }
    }

    /// Project a completed result, deriving its terminal lifecycle phase from
    /// the result's typed status.
    #[must_use]
    pub fn from_result(result: &BuildResult) -> Self {
        Self::terminal(phase_for(result.status()), result)
    }

    #[must_use]
    fn idle() -> Self {
        Self {
            phase: BuildPhase::Idle,
            terminal_status: None,
            request_identity: None,
            affected_inputs: Vec::new(),
            diagnostics: Vec::new(),
            candidates: Vec::new(),
            freshness: None,
            publish: None,
            restart_guidance: None,
            failure: None,
        }
    }

    #[must_use]
    fn active(
        phase: BuildPhase,
        request: &super::request::BuildRequest,
        candidates: &[BuildCandidate],
    ) -> Self {
        Self {
            phase,
            terminal_status: None,
            request_identity: Some(BuildRequestIdentity::from_request(request)),
            affected_inputs: request.affected_inputs(),
            diagnostics: Vec::new(),
            candidates: candidates.to_vec(),
            freshness: None,
            publish: None,
            restart_guidance: Some(request.restart_guidance()),
            failure: None,
        }
    }

    #[must_use]
    fn terminal(phase: BuildPhase, result: &BuildResult) -> Self {
        Self {
            phase,
            terminal_status: Some(result.status()),
            request_identity: Some(result.request_identity().clone()),
            affected_inputs: result.affected_inputs().to_vec(),
            diagnostics: result.diagnostics().to_vec(),
            candidates: result.candidates().to_vec(),
            freshness: Some(result.freshness().clone()),
            publish: Some(result.publish().clone()),
            restart_guidance: Some(result.restart_guidance()),
            failure: result.failure().cloned(),
        }
    }

    /// The lifecycle phase represented by this projection.
    #[must_use]
    pub const fn phase(&self) -> BuildPhase {
        self.phase
    }

    /// The terminal status, when this is a completed state.
    #[must_use]
    pub const fn terminal_status(&self) -> Option<BuildTerminalStatus> {
        self.terminal_status
    }

    /// The exact request identity, including input fingerprints and authority.
    #[must_use]
    pub const fn request_identity(&self) -> Option<&BuildRequestIdentity> {
        self.request_identity.as_ref()
    }

    /// Inputs identified as affected by the request, in lifecycle order.
    #[must_use]
    pub fn affected_inputs(&self) -> &[AffectedInput] {
        &self.affected_inputs
    }

    /// Structured diagnostics produced by the completed build.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Candidate output assets, preserving the lifecycle's deterministic order.
    #[must_use]
    pub fn candidates(&self) -> &[BuildCandidate] {
        &self.candidates
    }

    /// Freshness assessment, when the lifecycle has completed its check.
    #[must_use]
    pub const fn freshness(&self) -> Option<&FreshnessAssessment> {
        self.freshness.as_ref()
    }

    /// Typed publication outcome, including partial or indeterminate recovery.
    #[must_use]
    pub const fn publish(&self) -> Option<&PublishOutcome> {
        self.publish.as_ref()
    }

    /// Host restart guidance, when a request exists.
    #[must_use]
    pub const fn restart_guidance(&self) -> Option<RestartGuidance> {
        self.restart_guidance
    }

    /// Typed terminal failure, when one was recorded.
    #[must_use]
    pub const fn failure(&self) -> Option<&BuildResultFailure> {
        self.failure.as_ref()
    }

    /// Build generation carried by the request or terminal result.
    #[must_use]
    pub const fn generation(&self) -> Option<BuildGeneration> {
        match self.request_identity.as_ref() {
            Some(identity) => Some(identity.generation()),
            None => None,
        }
    }

    /// Authoring snapshot generation carried by the request or terminal result.
    #[must_use]
    pub const fn snapshot_generation(&self) -> Option<SnapshotGeneration> {
        match self.request_identity.as_ref() {
            Some(identity) => Some(identity.snapshot_generation()),
            None => None,
        }
    }
}

impl From<&BuildState> for BuildStatusProjection {
    fn from(state: &BuildState) -> Self {
        Self::from_state(state)
    }
}

impl From<&BuildResult> for BuildStatusProjection {
    fn from(result: &BuildResult) -> Self {
        Self::from_result(result)
    }
}

const fn phase_for(status: BuildTerminalStatus) -> BuildPhase {
    match status {
        BuildTerminalStatus::Succeeded => BuildPhase::Succeeded,
        BuildTerminalStatus::Failed => BuildPhase::Failed,
        BuildTerminalStatus::Stale => BuildPhase::Stale,
        BuildTerminalStatus::Cancelled => BuildPhase::Cancelled,
        BuildTerminalStatus::Superseded => BuildPhase::Superseded,
    }
}
