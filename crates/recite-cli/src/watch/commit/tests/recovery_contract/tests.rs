use std::io;
use std::path::PathBuf;

use super::{FIRST_STAGE_MARKER, ProjectBuildRecovery, SECOND_STAGE_MARKER};
use crate::i18n::Messages;

#[test]
fn recovery_records_use_alternate_typed_fluent_contract() {
    let mut resource = recite_ui::DEFAULT_RESOURCE.to_owned();
    for (id, replacement) in [
        (
            "watch-build-recovery-required",
            "alt-required count={$count} records={$records}",
        ),
        (
            "watch-build-recovery-record",
            "alt-record marker={$marker} reason={$reason} detail={$detail}",
        ),
        ("watch-build-recovery-reason-stage-cleanup", "alt-stage"),
        (
            "watch-build-recovery-reason-publication-uncommitted",
            "alt-uncommitted",
        ),
        (
            "watch-build-recovery-detail-io",
            "alt-io {$kind} {$raw_os_error} {$message}",
        ),
        (
            "watch-build-recovery-io-permission-denied",
            "alt-permission",
        ),
    ] {
        let mut found = false;
        let mut lines = Vec::new();
        for line in resource.lines() {
            if line.starts_with(&format!("{id} = ")) {
                found = true;
                let mut replacement_lines = replacement.lines();
                if let Some(first) = replacement_lines.next() {
                    lines.push(format!("{id} = {first}"));
                    lines.extend(replacement_lines.map(str::to_owned));
                }
            } else {
                lines.push(line.to_owned());
            }
        }
        resource = lines.join("\n");
        resource.push('\n');
        assert!(found, "missing resource {id}");
    }
    recite_ui::UiContract::default()
        .validate(&resource)
        .expect("alternate resource contract");
    let messages = Messages::from_resources(
        "en-GB".parse().expect("locale"),
        [
            (
                "en-US".parse().expect("locale"),
                recite_ui::DEFAULT_RESOURCE.to_owned(),
            ),
            ("en-GB".parse().expect("locale"), resource),
        ],
    )
    .expect("alternate messages");
    let first = ProjectBuildRecovery::with_io(
        PathBuf::from("stage/first\n.tmp"),
        super::super::super::recovery::ProjectBuildRecoveryReason::StageCleanupFailed,
        &io::Error::new(io::ErrorKind::PermissionDenied, "denied"),
    );
    let second = ProjectBuildRecovery::new(
        PathBuf::from("stage/second.tmp"),
        super::super::super::recovery::ProjectBuildRecoveryReason::PublicationUncommitted,
    );
    assert_eq!(
        super::super::super::build::format_recovery_required(
            &messages,
            2,
            &[second.clone(), first.clone(), first],
        ),
        format!(
            "alt-required count=2 records=; recovery markers: alt-record marker={FIRST_STAGE_MARKER} reason=alt-stage detail=alt-io alt-permission  denied\nalt-record marker={SECOND_STAGE_MARKER} reason=alt-uncommitted detail="
        )
    );
}
