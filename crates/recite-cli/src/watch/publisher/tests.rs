use std::fs;

use recite_compiler::BuildTarget;
use tempfile::TempDir;

use super::super::recovery::ProjectBuildRecoveryReason;
use super::{StagedTarget, cleanup};
use crate::watch::recovery::ProjectBuildRecovery;

#[test]
fn preparation_cleanup_failure_keeps_typed_marker_record() {
    let temp = TempDir::new().expect("tempdir");
    let marker = temp.path().join(".recite-stage-marker.tmp");
    fs::create_dir(&marker).expect("marker directory");
    let staged = StagedTarget {
        target: BuildTarget::new("compiled/dialogue.recitec").expect("target"),
        file: super::super::staging::StagedOutput {
            temp: marker.clone(),
            output: temp.path().join("compiled/dialogue.recitec"),
        },
    };
    let mut recovery = Vec::<ProjectBuildRecovery>::new();

    cleanup(&[staged], &mut recovery);

    assert_eq!(recovery.len(), 1);
    assert_eq!(recovery[0].marker(), marker);
    assert_eq!(
        recovery[0].reason(),
        ProjectBuildRecoveryReason::StageCleanupFailed
    );
}
