use std::path::Path;

use recite_compiler::{BuildTarget, PublishOutcome, RecoveryNeeded};

use super::publisher::{ProjectPreparedBuild, StagedTarget};
use super::recovery::ProjectBuildRecovery;
use super::staging;
use super::targets::reject_symlink_components;

pub(super) fn commit_prepared(
    root: &Path,
    prepared: ProjectPreparedBuild,
    recovery_log: &mut Vec<ProjectBuildRecovery>,
) -> PublishOutcome {
    let all = prepared
        .staged
        .iter()
        .map(|item| item.target.clone())
        .collect::<Vec<_>>();
    let mut committed = Vec::new();
    for (index, staged) in prepared.staged.iter().enumerate() {
        if reject_symlink_components(root, &staged.file.output).is_err() {
            record_uncommitted(&prepared.staged[index..], recovery_log);
            return partial(&committed, staged, &all, index);
        }
        match staging::replace(&staged.file) {
            staging::ReplaceOutcome::Failed => {
                record_uncommitted(&prepared.staged[index..], recovery_log);
                return partial(&committed, staged, &all, index);
            }
            staging::ReplaceOutcome::CommittedWithCleanup(error) => {
                recovery_log.push(ProjectBuildRecovery::new(
                    staged.file.temp.clone(),
                    format!("published but stage cleanup failed: {error}"),
                ));
                committed.push(staged.target.clone());
            }
            staging::ReplaceOutcome::Committed => committed.push(staged.target.clone()),
        }
    }
    PublishOutcome::Published { targets: committed }
}

fn partial(
    committed: &[BuildTarget],
    failed: &StagedTarget,
    all: &[BuildTarget],
    index: usize,
) -> PublishOutcome {
    PublishOutcome::Partial {
        committed: committed.to_vec(),
        failed: failed.target.clone(),
        remaining: all.iter().skip(index + 1).cloned().collect(),
        recovery: RecoveryNeeded::for_targets(
            committed
                .iter()
                .cloned()
                .chain([failed.target.clone()])
                .collect(),
        ),
    }
}

fn record_uncommitted(staged: &[StagedTarget], recovery_log: &mut Vec<ProjectBuildRecovery>) {
    for remaining in staged {
        recovery_log.push(ProjectBuildRecovery::new(
            remaining.file.temp.clone(),
            "publication did not commit this target; inspect before cleanup".to_owned(),
        ));
    }
}
