use std::time::Duration;

use recite_compiler::{BuildControl, BuildGeneration, BuildTerminalStatus};

use super::super::super::events::WatchState;
use super::super::{BuildStatus, build_once_with_clock};
use super::{alternate_messages, project, source, write};
use crate::i18n::{Messages, UiLocale};
use crate::watch::report_build_result;

fn timed_build<F>(
    state: &mut WatchState,
    messages: &Messages,
    control: &BuildControl,
    duration: Duration,
    post_publish: F,
) -> BuildStatus
where
    F: FnOnce(),
{
    let mut clock_calls = 0;
    let mut clock = || {
        clock_calls += 1;
        match clock_calls {
            1 => Duration::ZERO,
            2 => duration,
            calls => panic!("unexpected clock read {calls}"),
        }
    };
    let mut stderr = Vec::new();
    let status = build_once_with_clock(
        state,
        &mut stderr,
        messages,
        control,
        &mut clock,
        post_publish,
    )
    .expect("build");
    assert_eq!(clock_calls, 2);
    status
}

fn assert_duration(status: &BuildStatus, expected: Duration) {
    assert_eq!(status.telemetry().duration(), Some(expected), "{status:?}");
}

#[test]
fn coordinator_statuses_preserve_injected_duration() {
    let messages = Messages::load(&UiLocale::default()).expect("messages");

    let temp = tempfile::TempDir::new().expect("tempdir");
    write(temp.path(), "recite.project.toml", &project(true));
    write(temp.path(), "dialogue/main.recite", &source("Fresh."));
    let mut state = WatchState::new(temp.path().to_owned());
    let status = timed_build(
        &mut state,
        &messages,
        &BuildControl::new(),
        Duration::from_millis(37),
        || {},
    );
    assert!(matches!(status, BuildStatus::Fresh { asset_count: 1, .. }));
    assert_duration(&status, Duration::from_millis(37));

    let temp = tempfile::TempDir::new().expect("tempdir");
    write(temp.path(), "recite.project.toml", &project(true));
    write(temp.path(), "dialogue/main.recite", &source("Before."));
    let mut state = WatchState::new(temp.path().to_owned());
    let status = timed_build(
        &mut state,
        &messages,
        &BuildControl::new(),
        Duration::from_millis(41),
        || write(temp.path(), "dialogue/main.recite", &source("After.")),
    );
    assert!(matches!(status, BuildStatus::Stale { asset_count: 1, .. }));
    assert_duration(&status, Duration::from_millis(41));

    let temp = tempfile::TempDir::new().expect("tempdir");
    write(temp.path(), "recite.project.toml", &project(true));
    write(temp.path(), "dialogue/main.recite", "not valid recite");
    let mut state = WatchState::new(temp.path().to_owned());
    let status = timed_build(
        &mut state,
        &messages,
        &BuildControl::new(),
        Duration::from_millis(43),
        || {},
    );
    assert!(matches!(status, BuildStatus::Diagnostics { .. }));
    assert_duration(&status, Duration::from_millis(43));
}

#[test]
fn coordinator_cancellation_categories_preserve_injected_duration() {
    let messages = Messages::load(&UiLocale::default()).expect("messages");
    for (duration, cancellation) in [
        (Duration::from_millis(47), BuildTerminalStatus::Cancelled),
        (Duration::from_millis(53), BuildTerminalStatus::Superseded),
    ] {
        let temp = tempfile::TempDir::new().expect("tempdir");
        write(temp.path(), "recite.project.toml", &project(true));
        write(temp.path(), "dialogue/main.recite", &source("Interrupted."));
        let mut state = WatchState::new(temp.path().to_owned());
        let control = BuildControl::new();
        match cancellation {
            BuildTerminalStatus::Cancelled => control.cancel(),
            BuildTerminalStatus::Superseded => control.supersede(BuildGeneration::new(1)),
            _ => unreachable!("test cancellation category"),
        }
        let status = timed_build(&mut state, &messages, &control, duration, || {});
        match (&status, cancellation) {
            (BuildStatus::PublicationFailure { status, .. }, expected) => {
                assert_eq!(*status, expected);
            }
            other => panic!("unexpected status: {other:?}"),
        }
        assert_duration(&status, duration);
    }
}

#[test]
fn recovery_status_consumers_retain_duration_explicitly() {
    let recovery = vec![crate::watch::ProjectBuildRecovery::new(
        std::path::PathBuf::from("stage.marker"),
        crate::watch::ProjectBuildRecoveryReason::PublicationUncommitted,
    )];
    let statuses = [
        (
            BuildStatus::DiagnosticsWithRecovery {
                recovery: recovery.clone(),
                telemetry: recite_compiler::BuildTelemetry::from_duration(Duration::from_millis(
                    59,
                )),
            },
            Duration::from_millis(59),
        ),
        (
            BuildStatus::RecoveryRequired {
                asset_count: 1,
                recovery,
                telemetry: recite_compiler::BuildTelemetry::from_duration(Duration::from_millis(
                    61,
                )),
            },
            Duration::from_millis(61),
        ),
    ];
    for (status, expected) in statuses {
        assert_duration(&status, expected);
    }
}

#[test]
fn duration_report_uses_typed_unit_messages_and_alternate_catalogue_text() {
    let messages = alternate_messages(&[
        (
            "watch-build-duration-microseconds",
            "alt-micro: {$duration} microseconds.",
        ),
        (
            "watch-build-duration-milliseconds",
            "alt-milli: {$duration} milliseconds.",
        ),
    ]);
    for (duration, expected) in [
        (Duration::from_micros(700), "alt-micro: 700 microseconds."),
        (Duration::from_micros(1_500), "alt-milli: 1.5 milliseconds."),
        (
            Duration::from_micros(1_000_001),
            "alt-milli: 1000.001 milliseconds.",
        ),
        (
            Duration::from_micros(1_118),
            "alt-milli: 1.118 milliseconds.",
        ),
    ] {
        let status = BuildStatus::Fresh {
            asset_count: 1,
            telemetry: recite_compiler::BuildTelemetry::from_duration(duration),
        };
        let mut output = Vec::new();
        report_build_result(&mut output, Ok(status), &messages).expect("report");
        assert!(
            String::from_utf8(output)
                .expect("output utf8")
                .contains(expected),
            "expected {expected}"
        );
    }
}
