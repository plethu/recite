use std::sync::{Arc, Mutex};

use super::super::SnapshotGeneration;
use super::fingerprints::BuildFingerprintSet;
use super::identity::BuildGeneration;
use super::publish::{PreparedPublish, PublishOutcome, PublishRefusal};
use super::request::BuildRequest;

/// Current generation, snapshot, and fingerprint authority at publication time.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct BuildAuthority {
    latest_generation: BuildGeneration,
    snapshot_generation: SnapshotGeneration,
    fingerprints: BuildFingerprintSet,
}
impl BuildAuthority {
    #[must_use]
    pub fn from_request(request: &BuildRequest) -> Self {
        Self {
            latest_generation: request.generation(),
            snapshot_generation: request.snapshot_generation(),
            fingerprints: request.fingerprints().clone(),
        }
    }
    #[must_use]
    pub fn new(
        latest_generation: BuildGeneration,
        snapshot_generation: SnapshotGeneration,
        fingerprints: BuildFingerprintSet,
    ) -> Self {
        Self {
            latest_generation,
            snapshot_generation,
            fingerprints,
        }
    }
    #[must_use]
    pub const fn latest_generation(&self) -> BuildGeneration {
        self.latest_generation
    }
    #[must_use]
    pub const fn snapshot_generation(&self) -> SnapshotGeneration {
        self.snapshot_generation
    }
    #[must_use]
    pub const fn fingerprints(&self) -> &BuildFingerprintSet {
        &self.fingerprints
    }
    pub(crate) fn refusal_for(&self, request: &BuildRequest) -> Option<PublishRefusal> {
        if request.generation() != self.latest_generation {
            Some(PublishRefusal::StaleBuildGeneration)
        } else if request.snapshot_generation() != self.snapshot_generation {
            Some(PublishRefusal::StaleSnapshotGeneration)
        } else if request.fingerprints() != &self.fingerprints {
            Some(PublishRefusal::StaleFingerprints)
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
    pub(crate) fn commit<F>(
        self,
        prepared: PreparedPublish,
        commit: F,
    ) -> Result<PublishOutcome, BuildAuthorityError>
    where
        F: FnOnce(PreparedPublish) -> PublishOutcome,
    {
        let current = self
            .fence
            .authority
            .lock()
            .map_err(|_| BuildAuthorityError::Poisoned)?;
        if let Some(reason) = current.refusal_for(&self.request) {
            return Err(BuildAuthorityError::Refused { reason });
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
