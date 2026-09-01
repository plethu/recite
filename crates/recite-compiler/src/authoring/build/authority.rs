use std::sync::{Arc, Mutex};

use super::identity::BuildGeneration;
use super::publish::{BuildPreparedHandle, PublishOutcome, PublishRefusal};
use super::request::BuildRequest;
use super::request_identity::BuildRequestIdentity;

#[cfg(test)]
mod tests;

/// Current generation, snapshot, and fingerprint authority at publication time.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct BuildAuthority {
    latest_generation: BuildGeneration,
    request_identity: BuildRequestIdentity,
}
impl BuildAuthority {
    #[must_use]
    pub fn from_request(request: &BuildRequest) -> Self {
        Self {
            latest_generation: request.generation(),
            request_identity: BuildRequestIdentity::from_request(request),
        }
    }
    #[must_use]
    pub const fn latest_generation(&self) -> BuildGeneration {
        self.latest_generation
    }
    #[must_use]
    pub const fn identity(&self) -> &BuildRequestIdentity {
        &self.request_identity
    }
    pub(crate) fn refusal_for(&self, request: &BuildRequest) -> Option<PublishRefusal> {
        if request.generation() != self.latest_generation {
            Some(PublishRefusal::StaleBuildGeneration)
        } else if request.snapshot_generation() != self.request_identity.snapshot_generation() {
            Some(PublishRefusal::StaleSnapshotGeneration)
        } else if request.fingerprints() != self.request_identity.fingerprints() {
            Some(PublishRefusal::StaleFingerprints)
        } else if !self.request_identity.matches_request(request) {
            Some(PublishRefusal::RequestIdentityMismatch)
        } else {
            None
        }
    }
}

/// Shared authority updated when a newer request begins.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct BuildAuthorityFence {
    authority: Arc<Mutex<BuildAuthority>>,
}
impl BuildAuthorityFence {
    #[must_use]
    pub fn new(authority: BuildAuthority) -> Self {
        Self {
            authority: Arc::new(Mutex::new(authority)),
        }
    }
    pub(crate) fn install_if_newer(
        &self,
        authority: BuildAuthority,
    ) -> Result<(), BuildAuthorityError> {
        let mut current = self
            .authority
            .lock()
            .map_err(|_| BuildAuthorityError::Poisoned)?;
        if authority.latest_generation() > current.latest_generation() {
            *current = authority;
        }
        Ok(())
    }
    pub(crate) fn acquire(
        &self,
        request: &BuildRequest,
    ) -> Result<BuildPublishPermit, BuildAuthorityError> {
        let current = self
            .authority
            .lock()
            .map_err(|_| BuildAuthorityError::Poisoned)?;
        if let Some(reason) = current.refusal_for(request) {
            return Err(BuildAuthorityError::Refused { reason });
        }
        Ok(BuildPublishPermit {
            fence: self.clone(),
            request: request.clone(),
        })
    }
}

/// Permit to publish exactly one prepared request batch.
#[derive(Debug)]
#[non_exhaustive]
pub struct BuildPublishPermit {
    fence: BuildAuthorityFence,
    request: BuildRequest,
}
impl BuildPublishPermit {
    pub(crate) fn commit<H, F>(
        self,
        prepared: H,
        commit: F,
    ) -> Result<PublishOutcome, BuildAuthorityCommitError<H>>
    where
        H: BuildPreparedHandle,
        F: FnOnce(H) -> PublishOutcome,
    {
        let current = match self.fence.authority.lock() {
            Ok(current) => current,
            Err(_) => return Err(BuildAuthorityCommitError::Poisoned { prepared }),
        };
        if let Some(reason) = current.refusal_for(&self.request) {
            return Err(BuildAuthorityCommitError::Refused { reason, prepared });
        }
        Ok(commit(prepared))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum BuildAuthorityError {
    #[error("build authority fence is unavailable")]
    Poisoned,
    #[error("publish authority refused the request: {reason:?}")]
    Refused { reason: PublishRefusal },
}

#[derive(Debug)]
#[non_exhaustive]
pub(crate) enum BuildAuthorityCommitError<H> {
    Poisoned { prepared: H },
    Refused { reason: PublishRefusal, prepared: H },
}
