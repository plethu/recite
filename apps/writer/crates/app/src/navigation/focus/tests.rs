use super::*;
use crate::test_support::{open, project};
use tokio::sync::mpsc;

const SOURCE: &str = ":: start default\n> @ffee6p8\n  Alpha.\n> @rthh7ab\n  Bravo.\n> @bqyvpqq\n  Charlie.\n-> END\n";
fn id(digit: char) -> String {
    match digit {
        '1' => "ffee6p8",
        '2' => "rthh7ab",
        '3' => "bqyvpqq",
        _ => unreachable!("fixture has three passages"),
    }
    .into()
}

fn target(writer: &Writer, digit: char) -> EditingTarget {
    EditingTarget::passage(writer, id(digit))
}

#[test]
fn clean_focus_is_synchronous_and_needs_no_worker_or_source_projection() {
    let directory = project(SOURCE).unwrap();
    let mut writer = open(&directory).unwrap();
    let source = writer.session.model().document().source_snapshot();
    writer.native_field_focus(Some(target(&writer, '2')));
    assert_eq!(writer.session.model().view(), &target(&writer, '2').view());
    assert_eq!(writer.session.model().draft(), "Bravo.");
    assert!(!writer.navigation.busy());
    assert!(writer.editing_passage(&id('2')));
    writer.native_field_focus(None);
    assert!(!writer.editing_passage(&id('2')));
    writer.native_field_focus(Some(target(&writer, '2')));
    assert!(
        writer.editing_passage(&id('2')),
        "same-target focus is acknowledged"
    );
    assert!(Arc::ptr_eq(
        &source,
        &writer.session.model().document().source_snapshot()
    ));
}

#[test]
fn apply_admits_only_the_latest_field_after_its_matching_retirement() {
    for latest in [Some('3'), None] {
        let directory = project(SOURCE).unwrap();
        let mut writer = open(&directory).unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let (sender, mut jobs) = mpsc::unbounded_channel();
        writer.navigation.sender = Some(sender);
        writer.navigation.runtime = Some(runtime.handle().clone());
        writer.change(|model| {
            model.set_draft("Edited alpha.".into());
            Ok(())
        });
        writer.native_field_focus(Some(target(&writer, '2')));
        let completed = jobs.try_recv().unwrap().execute();
        let request = completed.request.clone();
        writer.native_field_focus(latest.map(|digit| target(&writer, digit)));
        drop(writer.accept_navigation(completed));
        assert!(writer.navigation.busy(), "retirement is part of admission");
        assert_eq!(writer.session.model().view(), &target(&writer, '1').view());
        writer.navigation_retired(&request);
        assert!(!writer.navigation.busy());
        assert!(
            jobs.try_recv().is_err(),
            "same-document focus must not start another navigation worker"
        );
        assert_eq!(
            writer.session.model().view(),
            &target(&writer, latest.unwrap_or('1')).view()
        );
        assert!(
            writer
                .session
                .model()
                .document()
                .source()
                .contains("Edited alpha.")
        );
        assert!(writer.field_restoration().is_none());
    }
}

#[test]
fn rejection_cancellation_and_newer_typing_keep_the_outgoing_draft_without_an_apply_loop() {
    for failure in ["invalid", "cancel", "newer"] {
        let directory = project(SOURCE).unwrap();
        let mut writer = open(&directory).unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let (sender, mut jobs) = mpsc::unbounded_channel();
        writer.navigation.sender = Some(sender);
        writer.navigation.runtime = Some(runtime.handle().clone());
        let draft = if failure == "invalid" {
            "-> syntax"
        } else {
            "Captured alpha."
        };
        writer.change(|model| {
            model.set_draft(draft.into());
            Ok(())
        });
        writer.native_field_focus(Some(target(&writer, '2')));
        let completed = jobs.try_recv().unwrap().execute();
        let request = completed.request.clone();
        if failure == "cancel" {
            writer.cancel_navigation();
        }
        if failure == "newer" {
            writer.change(|model| {
                model.set_draft("Newer alpha.".into());
                Ok(())
            });
        }
        drop(writer.accept_navigation(completed));
        assert!(writer.navigation.busy());
        writer.navigation_retired(&request);
        assert!(!writer.navigation.busy());
        assert!(jobs.try_recv().is_err());
        assert_eq!(writer.session.model().view(), &target(&writer, '1').view());
        assert_eq!(
            writer.session.model().draft(),
            if failure == "newer" {
                "Newer alpha."
            } else {
                draft
            }
        );
        assert_eq!(
            writer.field_restoration(),
            Some(FieldRestoration::Passage(target(&writer, '1')))
        );
        writer.native_field_focus(Some(target(&writer, '1')));
        assert!(writer.field_restoration().is_none());
    }
}
