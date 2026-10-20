use super::*;
use crate::tests::support::{finish_navigation, finish_work, open, project};
use std::sync::atomic;
use std::sync::mpsc;
use tokio::sync::mpsc as async_mpsc;
use tokio::sync::mpsc::UnboundedReceiver;

#[path = "failure/tests.rs"]
mod failure;
#[path = "history/tests.rs"]
mod history;
#[path = "reload/tests.rs"]
mod reload;
#[path = "repair/tests.rs"]
mod repair;
#[path = "visual/tests.rs"]
mod visual;

const SOURCE: &str = ":: first default\n> one@11111111111111111111\n  First.\n-> second\n:: second\n> two@22222222222222222222\n  Second.\n-> END\n";
struct Fixture {
    directory: tempfile::TempDir,
    writer: Writer,
    jobs: UnboundedReceiver<worker::Job>,
    other: PathBuf,
    _runtime: tokio::runtime::Runtime,
}
fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let directory = project(SOURCE)?;
    let other = directory.path().join("other.recite");
    std::fs::write(
        &other,
        ":: other\n> third@aaaaaaaaaaaaaaaaaaaa\n  Other.\n-> END\n",
    )?;
    let mut writer = open(&directory)?;
    let (sender, jobs) = async_mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    let runtime = tokio::runtime::Runtime::new()?;
    writer.navigation.runtime = Some(runtime.handle().clone());
    Ok(Fixture {
        directory,
        writer,
        jobs,
        other,
        _runtime: runtime,
    })
}

#[test]
fn newer_typing_and_cancellation_reject_prepared_switches_and_release_destination_leases() {
    for cancel in [false, true] {
        let mut fixture = fixture().unwrap();
        let writer = &mut fixture.writer;
        writer.change(|model| {
            model.set_draft("Keep original draft.".into());
            Ok(())
        });
        writer.switch(&fixture.other);
        let completed = fixture.jobs.try_recv().unwrap().execute();
        assert!(writer.navigation.busy());
        if cancel {
            writer.cancel_navigation();
        } else {
            writer.change(|model| {
                model.set_draft("Keep newer typing.".into());
                Ok(())
            });
        }
        drop(writer.accept_navigation(completed));
        assert!(!writer.navigation.busy());
        assert_eq!(
            writer.session.model().document().key().as_str(),
            "scene.recite"
        );
        assert_eq!(
            writer.session.model().draft(),
            if cancel {
                "Keep original draft."
            } else {
                "Keep newer typing."
            }
        );
        assert_eq!(writer.session.model().document().source(), SOURCE);
        drop(recite_writer_session::recovery::RecoveryStore::open(&fixture.other).unwrap());
        // Failure/cancellation must restart outgoing recovery and allow a fresh exact checkpoint.
        writer.switch(&fixture.other);
        finish_navigation(writer, &mut fixture.jobs).unwrap();
        assert_eq!(
            writer.session.model().document().key().as_str(),
            "other.recite"
        );
        writer.switch(&fixture.directory.path().join("scene.recite"));
        finish_navigation(writer, &mut fixture.jobs).unwrap();
        assert_eq!(
            writer.session.model().draft(),
            if cancel {
                "Keep original draft."
            } else {
                "Keep newer typing."
            }
        );
    }
}

#[test]
fn route_apply_accepts_captured_text_but_newer_typing_stops_the_route_and_survives_reopen() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    let original_view = writer.session.model().view().clone();
    writer.change(|model| {
        model.set_draft("Captured text.".into());
        Ok(())
    });
    writer.navigate(Target::Location(Location {
        document: "scene.recite".into(),
        beat: Some("second".into()),
        ..Location::default()
    }));
    let completed = fixture.jobs.try_recv().unwrap().execute();
    writer.change(|model| {
        model.set_draft("Newer typing.".into());
        Ok(())
    });
    drop(writer.accept_navigation(completed));
    assert!(!writer.navigation.busy());
    assert!(fixture.jobs.try_recv().is_err());
    assert_eq!(writer.session.model().view(), &original_view);
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains("Captured text.")
    );
    assert_eq!(writer.session.model().draft(), "Newer typing.");
    assert!(writer.status.contains("newer typing remains open"));
    writer.session.project_mut().unwrap().checkpoint().unwrap();
    drop(fixture.writer);
    let writer = open(&fixture.directory).unwrap();
    assert_eq!(writer.session.model().draft(), "Newer typing.");
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains("Captured text.")
    );
}

#[test]
fn failed_destination_harvests_parking_and_apply_and_history_use_checked_worker_publication() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    writer.navigate(Target::Location(Location {
        document: "other.recite".into(),
        beat: Some("missing".into()),
        ..Location::default()
    }));
    finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert_eq!(writer.session.model().document().source(), SOURCE);
    assert!(!writer.navigation.busy());
    drop(recite_writer_session::recovery::RecoveryStore::open(&fixture.other).unwrap());
    writer.change(|model| {
        model.set_draft("Applied asynchronously.".into());
        Ok(())
    });
    writer.navigate(Target::Edit(EditOperation::Apply));
    finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert!(!writer.session.model().has_draft());
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains("Applied asynchronously.")
    );
    writer.navigate(Target::Edit(EditOperation::Undo));
    finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert_eq!(writer.session.model().document().source(), SOURCE);
}

#[test]
fn forwarded_routes_reply_after_publication_and_honor_the_forwarders_cancellation() {
    for cancel in [false, true] {
        let mut fixture = fixture().unwrap();
        let (reply, replied) = mpsc::channel();
        let cancelled = Arc::new(atomic::AtomicBool::new(false));
        let incoming = recite_writer_session::activation::IncomingRoute {
            project: None,
            route: None,
            document: None,
            editor: None,
            cancelled: cancelled.clone(),
            reply,
        };
        fixture
            .writer
            .navigate_activation(
                Location {
                    document: "other.recite".into(),
                    beat: Some("other".into()),
                    ..Location::default()
                },
                incoming,
            )
            .unwrap();
        assert!(replied.try_recv().is_err());
        if cancel {
            cancelled.store(true, atomic::Ordering::Release);
        }
        finish_navigation(&mut fixture.writer, &mut fixture.jobs).unwrap();
        assert_eq!(replied.try_recv().unwrap().is_ok(), !cancel);
        assert_eq!(
            fixture.writer.session.model().document().key().as_str(),
            if cancel {
                "scene.recite"
            } else {
                "other.recite"
            }
        );
    }
}

#[test]
fn save_after_apply_waits_for_retirement_keeps_newer_typing_and_never_runs_after_failure_or_cancel()
{
    for outcome in ["accepted", "newer", "failed", "cancelled"] {
        let mut fixture = fixture().unwrap();
        let writer = &mut fixture.writer;
        let (sender, mut saves) = async_mpsc::unbounded_channel();
        writer.work.sender = Some(sender);
        writer.change(|model| {
            model.set_draft(
                if outcome == "failed" {
                    "-> syntax"
                } else {
                    "Captured text."
                }
                .into(),
            );
            Ok(())
        });
        writer.navigate(Target::Edit(EditOperation::Apply));
        let completed = fixture.jobs.try_recv().unwrap().execute();
        let request = completed.request.clone();
        if outcome == "newer" {
            writer.change(|model| {
                model.set_draft("Newer saved text.".into());
                Ok(())
            });
        }
        writer.save();
        assert!(writer.status.contains("Save requested"));
        assert!(!writer.work.busy());
        if outcome == "cancelled" {
            writer.cancel_navigation();
        }
        let retired = writer.accept_navigation(completed);
        assert!(
            !writer.work.busy(),
            "Save must wait until displaced analysis retires"
        );
        drop(retired);
        writer.navigation_retired(&request);
        if matches!(outcome, "failed" | "cancelled") {
            assert!(!writer.work.busy());
            assert!(saves.try_recv().is_err());
            assert_eq!(
                std::fs::read_to_string(fixture.directory.path().join("scene.recite")).unwrap(),
                SOURCE
            );
            if outcome == "failed" {
                assert!(writer.status.contains("Requested save was not run"));
            }
        } else {
            finish_work(writer, &mut saves).unwrap();
            let source =
                std::fs::read_to_string(fixture.directory.path().join("scene.recite")).unwrap();
            assert!(source.contains(if outcome == "newer" {
                "Newer saved text."
            } else {
                "Captured text."
            }));
            assert!(!writer.session.dirty());
        }
    }
}

#[test]
fn deferred_save_all_is_not_downgraded_by_a_later_current_file_save() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    let scene = fixture.directory.path().join("scene.recite");
    let project = writer.session.project_mut().unwrap();
    project.switch(&fixture.other).unwrap();
    project
        .current_mut()
        .edit(|model| {
            model.set_draft("Inactive draft.".into());
            Ok(())
        })
        .unwrap();
    project.switch(&scene).unwrap();
    let (sender, mut saves) = async_mpsc::unbounded_channel();
    writer.work.sender = Some(sender);
    writer.change(|model| {
        model.set_draft("Active draft.".into());
        Ok(())
    });
    writer.navigate(Target::Edit(EditOperation::Apply));
    writer.save_all();
    writer.save();
    finish_navigation(writer, &mut fixture.jobs).unwrap();
    finish_work(writer, &mut saves).unwrap();
    assert!(
        std::fs::read_to_string(&fixture.other)
            .unwrap()
            .contains("Inactive draft.")
    );
    assert!(
        std::fs::read_to_string(scene)
            .unwrap()
            .contains("Active draft.")
    );
    assert!(!writer.session.dirty());
}
