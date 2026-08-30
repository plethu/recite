use std::fs;
use std::path::PathBuf;

use recite_compiler::{
    BuildCandidate, BuildControl, BuildPreparedHandle, BuildPublisher, BuildRequest, BuildTarget,
    PreparedPublishIdentity, PublishAbortReason, PublishFailure, PublishFailureReason,
    PublishOutcome, RecoveryNeeded,
};

use super::request::ProjectBuildRequest;
use super::staging::{self, StagedOutput};
use super::targets::{TargetMap, TargetMapError};

/// Filesystem publisher for one exact prepared project request.
///
/// Preparation writes only adjacent Recite-owned stage markers. Published
/// targets change only during `commit`, under the compiler authority fence.
#[derive(Debug)]
pub struct ProjectBuildPublisher {
    root: PathBuf,
    request: BuildRequest,
    targets: TargetMap,
}

/// A staged candidate batch consumed by commit or abort.
#[derive(Debug)]
pub struct ProjectPreparedBuild {
    identity: PreparedPublishIdentity,
    staged: Vec<StagedTarget>,
}

#[derive(Debug)]
struct StagedTarget {
    target: BuildTarget,
    file: StagedOutput,
}

impl ProjectBuildPublisher {
    /// Validate the project output boundary before accepting candidates.
    pub fn new(request: &ProjectBuildRequest) -> Result<Self, ProjectBuildPublisherError> {
        let targets = TargetMap::from_request(request)?;
        Ok(Self {
            root: targets.root.clone(),
            request: request.build_request().clone(),
            targets,
        })
    }

    #[must_use]
    pub fn project_root(&self) -> &std::path::Path {
        &self.root
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
                cleanup(&staged);
                return Err(PublishFailure::Preparation {
                    target: candidate.target().clone(),
                    reason: PublishFailureReason::Rejected,
                });
            }
            let Some(output) = self.targets.targets.get(candidate.target()) else {
                cleanup(&staged);
                return Err(PublishFailure::Preparation {
                    target: candidate.target().clone(),
                    reason: PublishFailureReason::Rejected,
                });
            };
            if let Err(error) = ensure_output_boundary(&self.root, output) {
                cleanup(&staged);
                return Err(PublishFailure::Preparation {
                    target: candidate.target().clone(),
                    reason: error,
                });
            }
            let file =
                match staging::stage(output, candidate.bytes(), request.generation().as_u64()) {
                    Ok(file) => file,
                    Err(_) => {
                        cleanup(&staged);
                        return Err(PublishFailure::Preparation {
                            target: candidate.target().clone(),
                            reason: PublishFailureReason::Storage,
                        });
                    }
                };
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
            cleanup(&prepared.staged);
        }
    }

    fn commit(&mut self, prepared: Self::Prepared) -> PublishOutcome {
        let all = prepared
            .staged
            .iter()
            .map(|item| item.target.clone())
            .collect::<Vec<_>>();
        let mut committed = Vec::new();
        for (index, staged) in prepared.staged.iter().enumerate() {
            if ensure_output_boundary(&self.root, &staged.file.output).is_err()
                || staging::replace(&staged.file).is_err()
            {
                let remaining = all.iter().skip(index + 1).cloned().collect();
                return PublishOutcome::Partial {
                    committed: committed.clone(),
                    failed: staged.target.clone(),
                    remaining,
                    recovery: RecoveryNeeded::for_targets(
                        committed
                            .iter()
                            .cloned()
                            .chain([staged.target.clone()])
                            .collect(),
                    ),
                };
            }
            committed.push(staged.target.clone());
        }
        PublishOutcome::Published { targets: committed }
    }
}

fn cleanup(staged: &[StagedTarget]) {
    for staged in staged {
        staging::remove(&staged.file);
    }
}

fn has_duplicates(targets: &[BuildTarget]) -> bool {
    targets.windows(2).any(|pair| pair[0] == pair[1])
}

fn ensure_output_boundary(
    root: &std::path::Path,
    output: &std::path::Path,
) -> Result<(), PublishFailureReason> {
    if fs::symlink_metadata(output)
        .ok()
        .is_some_and(|metadata| metadata.file_type().is_symlink())
    {
        let canonical = fs::canonicalize(output).map_err(|_| PublishFailureReason::Rejected)?;
        return if canonical.starts_with(root) {
            Ok(())
        } else {
            Err(PublishFailureReason::Rejected)
        };
    }
    let mut existing = output.to_owned();
    while !existing.exists() {
        if !existing.pop() {
            return Err(PublishFailureReason::Rejected);
        }
    }
    let canonical = fs::canonicalize(existing).map_err(|_| PublishFailureReason::Storage)?;
    if !canonical.starts_with(root) {
        return Err(PublishFailureReason::Rejected);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ProjectBuildPublisherError {
    #[error(transparent)]
    Targets(#[from] TargetMapError),
}
