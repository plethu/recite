use super::*;
use crate::{WindowKey, test_support};

const SOURCE: &str = ":: start default\n> @ffee6p8\n  Hello.\n-> END\n";

fn writer() -> (tempfile::TempDir, Writer) {
    let directory = test_support::project(SOURCE).unwrap();
    let writer = test_support::open(&directory).unwrap();
    (directory, writer)
}

#[test]
fn notices_for_another_window_change_nothing() {
    let (_directory, mut writer) = writer();
    writer.accept_external_notice(Output {
        window: WindowKey::next(),
        event: Event::Warning("Stale window".into()),
    });
    assert_eq!(writer.notices.warning, None);
}

#[test]
fn watcher_warnings_are_shown_and_a_stopped_watcher_releases_its_connection() {
    let (_directory, mut writer) = writer();
    let window = writer.window;
    writer.accept_external_notice(Output {
        window,
        event: Event::Warning("Watching is limited.".into()),
    });
    assert_eq!(
        writer.notices.warning.as_deref(),
        Some("Watching is limited.")
    );
    writer.accept_external_notice(Output {
        window,
        event: Event::Stopped("Automatic checks stopped.".into()),
    });
    assert_eq!(
        writer.notices.warning.as_deref(),
        Some("Automatic checks stopped.")
    );
    assert!(writer.notices.connection.is_none() && writer.notices.flight.is_none());
}
