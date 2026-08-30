use std::path::PathBuf;

use recite_compiler::{
    BuildCandidate, BuildControl, BuildPreparedHandle, BuildPublisher, BuildRequest, BuildTarget,
    PreparedPublishIdentity, PublishAbortReason, PublishFailure, PublishFailureReason,
    PublishOutcome,
};

use super::recovery::{ProjectBuildPublisherError, ProjectBuildRecovery};
use super::request::ProjectBuildRequest;
use super::staging::{self, StagedOutput};
use super::targets::{TargetMap, reject_symlink_components};

/// Filesystem publisher for one exact prepared project request.
///
/// Preparation writes only adjacent Recite-owned stage markers. Published
/// targets change only during `commit`, under the compiler authority fence.
/// Preparation may create missing output directories. Rename replacement is
/// process-visible per-file atomicity where the platform supports it; this
/// publisher does not claim crash durability or global batch atomicity.
#[derive(Debug)]
pub struct ProjectBuildPublisher {
    root: PathBuf,
    request: BuildRequest,
    targets: TargetMap,
    recovery: Vec<ProjectBuildRecovery>,
}

/// A staged candidate batch consumed by commit or abort.
#[derive(Debug)]
pub struct ProjectPreparedBuild {
    pub(super) identity: PreparedPublishIdentity,
    pub(super) staged: Vec<StagedTarget>,
}

#[derive(Debug)]
pub(super) struct StagedTarget {
    pub(super) target: BuildTarget,
    pub(super) file: StagedOutput,
}

impl ProjectBuildPublisher {
    /// Validate the project output boundary before accepting candidates.
    pub fn new(request: &ProjectBuildRequest) -> Result<Self, ProjectBuildPublisherError> {
        let targets = TargetMap::from_request(request)?;
        Ok(Self {
            root: targets.root.clone(),
            request: request.build_request().clone(),
            targets,
            recovery: Vec::new(),
        })
    }

    #[must_use]
    pub fn project_root(&self) -> &std::path::Path {
        &self.root
    }

    #[must_use]
    pub fn recovery(&self) -> &[ProjectBuildRecovery] {
        &self.recovery
    }
}

impl BuildPreparedHandle for ProjectPreparedBuild {
    fn identity(&self) -> PreparedPublishIdentity {
        self.identity.clone()
    }
}

impl BuildPublisher for ProjectBuildPublisher {
    type Prepared = ProjectPreparedBuild;

    fn prepare(
        &mut self,
        request: &BuildRequest,
        candidates: &[BuildCandidate],
        control: &BuildControl,
    ) -> Result<Self::Prepared, PublishFailure> {
        if request != &self.request {
            let Some(target) = candidates
                .first()
                .map(|candidate| candidate.target().clone())
                .or_else(|| Some(self.targets.first_target.clone()))
            else {
                return Err(PublishFailure::Preparation {
                    target: self.targets.first_target.clone(),
                    reason: PublishFailureReason::Rejected,
                });
            };
            return Err(PublishFailure::Preparation {
                target,
                reason: PublishFailureReason::Rejected,
            });
        }
        let mut expected = self.targets.targets.keys().cloned().collect::<Vec<_>>();
        expected.sort();
        let mut actual = candidates
            .iter()
            .map(|candidate| candidate.target().clone())
            .collect::<Vec<_>>();
        actual.sort();
        if expected != actual || has_duplicates(&actual) {
            let Some(target) = candidates
                .first()
                .map(|candidate| candidate.target().clone())
                .or_else(|| expected.first().cloned())
            else {
                return Err(PublishFailure::Preparation {
                    target: self.targets.first_target.clone(),
                    reason: PublishFailureReason::Rejected,
                });
            };
            return Err(PublishFailure::Preparation {
                target,
                reason: PublishFailureReason::Rejected,
            });
        }

        let mut ordered = candidates.to_vec();
        ordered.sort_by(|left, right| left.target().cmp(right.target()));
        let mut staged = Vec::with_capacity(ordered.len());
        for candidate in &ordered {
            if control.cancellation().is_some() {
                cleanup(&staged, &mut self.recovery);
                return Err(PublishFailure::Preparation {
                    target: candidate.target().clone(),
                    reason: PublishFailureReason::Rejected,
                });
            }
            let Some(output) = self.targets.targets.get(candidate.target()) else {
                cleanup(&staged, &mut self.recovery);
                return Err(PublishFailure::Preparation {
                    target: candidate.target().clone(),
                    reason: PublishFailureReason::Rejected,
                });
            };
            if let Err(error) = ensure_output_boundary(&self.root, output) {
                cleanup(&staged, &mut self.recovery);
                return Err(PublishFailure::Preparation {
                    target: candidate.target().clone(),
                    reason: error,
                });
            }
            let file = match staging::stage(output, candidate.bytes(), request) {
                Ok(file) => file,
                Err(_) => {
                    cleanup(&staged, &mut self.recovery);
                    return Err(PublishFailure::Preparation {
                        target: candidate.target().clone(),
                        reason: PublishFailureReason::Storage,
                    });
                }
            };
            if let Err(error) = ensure_output_boundary(&self.root, output) {
                if let Err(cleanup_error) = staging::remove(&file) {
                    self.recovery.push(ProjectBuildRecovery::new(
                        file.temp.clone(),
                        format!("stage cleanup failed: {cleanup_error}"),
                    ));
                }
                cleanup(&staged, &mut self.recovery);
                return Err(PublishFailure::Preparation {
                    target: candidate.target().clone(),
                    reason: error,
                });
            }
            staged.push(StagedTarget {
                target: candidate.target().clone(),
                file,
            });
        }

        Ok(ProjectPreparedBuild {
            identity: PreparedPublishIdentity::for_request(request, ordered),
            staged,
        })
    }

    fn abort(&mut self, prepared: Option<Self::Prepared>, _reason: PublishAbortReason) {
        if let Some(prepared) = prepared {
            cleanup(&prepared.staged, &mut self.recovery);
        }
    }

    fn commit(&mut self, prepared: Self::Prepared) -> PublishOutcome {
        super::commit::commit_prepared(&self.root, prepared, &mut self.recovery)
    }
}

fn cleanup(staged: &[StagedTarget], recovery: &mut Vec<ProjectBuildRecovery>) {
    for staged in staged {
        if let Err(error) = staging::remove(&staged.file) {
            recovery.push(ProjectBuildRecovery::new(
                staged.file.temp.clone(),
                format!("stage cleanup failed: {error}"),
            ));
        }
    }
}

fn has_duplicates(targets: &[BuildTarget]) -> bool {
    targets.windows(2).any(|pair| pair[0] == pair[1])
}

fn ensure_output_boundary(
    root: &std::path::Path,
    output: &std::path::Path,
) -> Result<(), PublishFailureReason> {
    reject_symlink_components(root, output).map_err(|error| match error {
        super::targets::TargetPathError::Inspection(_) => PublishFailureReason::Storage,
        _ => PublishFailureReason::Rejected,
    })
}
