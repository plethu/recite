use super::*;
use crate::editing::{Pane, Writer};
use crate::tests::support;
use freya_testing::prelude::*;
use recite_writer_model::View;

#[test]
fn rejected_navigation_retires_its_last_captured_document_owner() {
    fn app() -> impl IntoElement {
        let mut writer = support::writer();
        use_hook(move || {
            let owner = writer
                .buffers
                .model
                .peek()
                .as_ref()
                .unwrap()
                .document()
                .source_snapshot();
            let weak = std::sync::Arc::downgrade(&owner);
            drop(owner);
            let (requests, waiting) = async_channel::bounded(1);
            writer.work.requests.set(Some(requests));
            navigate(
                writer,
                crate::navigation::request::Request {
                    target: crate::navigation::request::Target::Location(
                        crate::navigation::snapshot(writer),
                    ),
                    cancelled: None,
                },
                EventHandler::new(|result: Result<(), String>| assert!(result.is_err())),
            )
            .unwrap();
            let pending = waiting.try_recv().unwrap();
            writer
                .buffers
                .model
                .set(Err(recite_writer_model::EditError::Stale.into()));
            assert!(
                weak.upgrade().is_some(),
                "the queued origin owns the replaced document"
            );
            futures_lite::future::block_on(finish(writer, pending));
            assert!(
                weak.upgrade().is_none(),
                "rejection must retire captured analysis too"
            );
        });
        label().text("Rejected navigation retired")
    }
    let _ = TestingRunner::new(app, (900., 650.).into(), |_| {}, 1.);
}

#[test]
fn a_newer_dialog_or_typing_before_navigation_starts_cannot_apply_the_old_draft() {
    fn app() -> impl IntoElement {
        let mut writer = support::writer();
        use_hook(move || {
            for changed in ["settings", "typing"] {
                writer.buffers.install(support::model(), false);
                writer.settings_open.set(false);
                writer.buffers.prose.set("Submitted.".into());
                let (requests, waiting) = async_channel::bounded(1);
                writer.work.requests.set(Some(requests));
                let outcome = std::rc::Rc::new(std::cell::RefCell::new(None));
                let completed = outcome.clone();
                navigate(
                    writer,
                    crate::navigation::request::Request {
                        target: crate::navigation::request::Target::Location(
                            crate::navigation::location::Location {
                                source: true,
                                passage: None,
                                beat: None,
                                ..crate::navigation::snapshot(writer)
                            },
                        ),
                        cancelled: None,
                    },
                    EventHandler::new(move |result| {
                        *completed.borrow_mut() = Some(result);
                    }),
                )
                .unwrap();
                let pending = waiting.try_recv().unwrap();
                if changed == "settings" {
                    writer.settings_open.set(true);
                } else {
                    writer.buffers.prose.set("Newer typing.".into());
                }
                futures_lite::future::block_on(finish(writer, pending));
                assert!(outcome.borrow().as_ref().unwrap().is_err());
                let state = writer.buffers.model.peek();
                let model = state.as_ref().unwrap();
                assert_eq!(model.document().source(), support::SOURCE);
                assert!(!model.document().can_undo());
                assert_eq!(
                    model.draft(),
                    if changed == "typing" {
                        "Newer typing."
                    } else {
                        "Submitted."
                    }
                );
                assert!(model.has_draft());
            }
        });
        label().text("Navigation origin checked")
    }
    let _ = TestingRunner::new(app, (900., 650.).into(), |_| {}, 1.);
}

#[test]
fn newer_dialog_view_or_typing_cancels_only_the_deferred_action() {
    fn app() -> impl IntoElement {
        let writer = support::writer();
        use_hook(move || {
            for changed in [
                "none",
                "settings",
                "commands",
                "pane",
                "view",
                "localising",
                "typing",
            ] {
                exercise_continuation(writer, changed).unwrap();
            }
        });
        label().text("Continuations checked")
    }
    let _ = TestingRunner::new(app, (900., 650.).into(), |_| {}, 1.);
}
fn exercise_continuation(
    mut writer: Writer,
    changed: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    writer.buffers.install(support::model(), false);
    writer.settings_open.set(false);
    writer.command_search.mode.set(None);
    writer.pane.set(Pane::Script);
    writer.layout.view.set(recite_config::WriterView::Script);
    writer.localisation.write().active = false;
    writer.buffers.prose.set("Submitted.".into());
    let (requests, waiting) = async_channel::bounded(1);
    writer.work.requests.set(Some(requests));
    let called = std::rc::Rc::new(std::cell::Cell::new(false));
    let action = called.clone();
    writer.after_prose(move |writer| {
        writer.work.require_idle()?;
        action.set(true);
        writer.try_navigate(recite_writer_model::Workbench::add_line)
    });
    let pending = waiting.try_recv()?;
    assert!(writer.work.busy());
    match changed {
        "settings" => writer.settings_open.set(true),
        "commands" => writer
            .command_search
            .mode
            .set(Some(crate::commands::SearchMode::Commands)),
        "pane" => writer.pane.set(Pane::Map),
        "view" => writer.layout.view.set(recite_config::WriterView::Map),
        "localising" => writer.localisation.write().active = true,
        "typing" => writer.buffers.prose.set("Newer typing.".into()),
        _ => {}
    }
    futures_lite::future::block_on(finish(writer, pending));
    assert!(!writer.work.busy());
    assert_eq!(called.get(), changed == "none", "{changed}");
    let state = writer.buffers.model.peek();
    let model = state.as_ref().unwrap();
    assert!(model.document().source().contains("Submitted."));
    if changed == "typing" {
        assert_eq!(model.draft(), "Newer typing.");
        assert!(model.has_draft());
    }
    Ok(())
}

#[test]
fn latest_focused_field_wins_and_returning_to_current_field_keeps_newer_typing() {
    fn app() -> impl IntoElement {
        let mut writer = support::writer();
        let a = use_a11y();
        let b = use_a11y();
        let c = use_a11y();
        use_hook(move || {
            for return_to_current in [false, true] {
                writer.buffers.install(support::model(), false);
                writer.buffers.prose.set("Submitted.".into());
                let (requests, waiting) = async_channel::bounded(1);
                writer.work.requests.set(Some(requests));
                let mut platform = Platform::get().current_window();
                platform.focused_accessibility_id.set(b);
                crate::prose::focus::select(writer, support::B.into(), b);
                assert!(writer.work.busy());
                platform.focused_accessibility_id.set(c);
                crate::prose::focus::select(writer, support::C.into(), c);
                assert_eq!(waiting.len(), 1);
                if return_to_current {
                    platform.focused_accessibility_id.set(a);
                    crate::prose::focus::select(writer, support::A.into(), a);
                    writer.buffers.prose.set("Newer typing.".into());
                }
                let pending = waiting.try_recv().unwrap();
                futures_lite::future::block_on(finish(writer, pending));
                let state = writer.buffers.model.peek();
                let model = state.as_ref().unwrap();
                assert_eq!(
                    model.view(),
                    &View::Passage(
                        if return_to_current {
                            support::A
                        } else {
                            support::C
                        }
                        .into()
                    )
                );
                assert_eq!(
                    model.draft(),
                    if return_to_current {
                        "Newer typing."
                    } else {
                        "Before C."
                    }
                );
                assert_eq!(model.has_draft(), return_to_current);
            }
        });
        label().text("Focus checked")
    }
    let _ = TestingRunner::new(app, (900., 650.).into(), |_| {}, 1.);
}
