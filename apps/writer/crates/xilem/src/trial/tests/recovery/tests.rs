use super::*;
use crate::{attention::EditingTarget, keyed::keyed, tests::support::mounted::Mounted};
use std::fs;
use xilem::{WidgetView, masonry::widgets::Passthrough};

fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed((), crate::view::writer(writer))
}

#[test]
fn mounted_preview_recovery_applies_the_current_field_without_saving_or_replacing_it() {
    let directory = project(SOURCE).unwrap();
    let mut writer = open(&directory).unwrap();
    install(&mut writer).unwrap();
    writer.return_from_trial();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut navigation) = async_mpsc::unbounded_channel();
    let navigation_sender = sender;
    let (sender, mut requests) = async_mpsc::unbounded_channel();
    let mut mounted = Mounted::new(writer, surface);
    mounted.state.navigation.sender = Some(navigation_sender);
    mounted.state.navigation.runtime = Some(runtime.handle().clone());
    mounted.state.trial.sender = Some(sender);
    let field = mounted.find("Manuscript");
    mounted.replace_text("Manuscript", "Current unfinished text.");
    mounted.activate("Try scene");
    assert!(mounted.contains_text("Return to unfinished edit"));
    assert!(!mounted.contains_text("Review current inputs"));
    assert!(!mounted.contains_text("no problems"));
    assert_eq!(
        mounted.state.session.model().preview_page().unwrap().text,
        "Hello."
    );
    mounted.activate("Return to unfinished edit");
    assert_eq!(mounted.find("Manuscript"), field);
    assert!(mounted.state.attention.edit_request().is_some());
    mounted.activate("Try scene");
    mounted.activate("Apply edit and preview");
    assert!(mounted.state.navigation.applying_preview());
    assert!(requests.try_recv().is_err());
    support::finish_navigation(&mut mounted.state, &mut navigation).unwrap();
    assert!(matches!(
        finish(&mut mounted.state, requests.try_recv().unwrap()).unwrap(),
        Some(Retired::Preview(_))
    ));
    assert_eq!(
        mounted.state.session.model().preview_page().unwrap().text,
        "Current unfinished text."
    );
    assert!(mounted.state.session.dirty());
    assert_eq!(
        fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
        SOURCE
    );
    mounted.settle();
    mounted.activate("Return to writing");
    assert_eq!(mounted.find("Manuscript"), field);
    assert!(
        mounted
            .state
            .session
            .model()
            .document()
            .source()
            .contains("Current unfinished text.")
    );
}

#[test]
fn apply_then_preview_never_starts_after_newer_typing_cancellation_return_or_replacement() {
    for change in ["newer", "cancel", "return", "replace", "composition"] {
        let directory = project(SOURCE).unwrap();
        let mut writer = open(&directory).unwrap();
        install(&mut writer).unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let (sender, mut navigation) = async_mpsc::unbounded_channel();
        writer.navigation.sender = Some(sender);
        writer.navigation.runtime = Some(runtime.handle().clone());
        let (sender, mut requests) = async_mpsc::unbounded_channel();
        writer.trial.sender = Some(sender);
        writer.change(|model| {
            model.set_draft("Captured edit.".into());
            Ok(())
        });
        writer.start_trial();
        writer.apply_trial_edit();
        let completed = navigation.try_recv().unwrap().execute();
        let preedit = match change {
            "newer" => {
                writer.change(|model| {
                    model.set_draft("Newer typing.".into());
                    Ok(())
                });
                None
            }
            "cancel" => {
                writer.cancel_trial_preparation();
                None
            }
            "return" => {
                writer.return_from_trial();
                None
            }
            "replace" => {
                writer.window = xilem::WindowId::next();
                None
            }
            _ => Some(writer.attention.begin_preedit()),
        };
        writer.accept_navigation(completed);
        assert!(!writer.navigation.busy(), "{change}");
        assert!(
            requests.try_recv().is_err(),
            "must not preview after {change}"
        );
        assert_eq!(
            writer.session.model().preview_page().unwrap().text,
            "Hello."
        );
        assert_eq!(
            fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
            SOURCE
        );
        if change == "newer" {
            assert_eq!(writer.session.model().draft(), "Newer typing.");
            assert!(
                writer
                    .session
                    .model()
                    .document()
                    .source()
                    .contains("Captured edit.")
            );
            assert!(!writer.trial.visible);
        }
        if change == "return" {
            assert!(!writer.trial.visible);
        }
        drop(preedit);
    }
}

#[test]
fn invalid_apply_returns_to_the_exact_field_with_local_recovery_and_keeps_the_old_run() {
    let directory = project(SOURCE).unwrap();
    let mut writer = open(&directory).unwrap();
    install(&mut writer).unwrap();
    let target = EditingTarget::capture(&writer).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut navigation) = async_mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    let (sender, mut requests) = async_mpsc::unbounded_channel();
    writer.trial.sender = Some(sender);
    writer.change(|model| {
        model.set_draft("-> syntax".into());
        Ok(())
    });
    writer.start_trial();
    writer.apply_trial_edit();
    support::finish_navigation(&mut writer, &mut navigation).unwrap();
    assert!(requests.try_recv().is_err());
    assert!(!writer.trial.visible);
    assert_eq!(writer.attention.edit_request().unwrap().target(), &target);
    assert_eq!(writer.session.model().draft(), "-> syntax");
    assert_eq!(
        writer.reading_refusal().unwrap().title(),
        "Can't preview this edit yet"
    );
    assert_eq!(
        writer.session.model().preview_page().unwrap().text,
        "Hello."
    );
}

#[test]
fn preview_opening_cannot_hide_an_active_manuscript_composition() {
    let directory = project(SOURCE).unwrap();
    let mut writer = open(&directory).unwrap();
    let preedit = writer.attention.begin_preedit();
    writer.start_trial();
    assert!(!writer.trial.visible);
    writer.configure_trial();
    assert!(!writer.trial.visible);
    assert!(writer.status.contains("Finish text composition"));
    drop(preedit);
}

#[test]
fn finishing_an_edit_independently_retires_its_preview_recovery_message() {
    for apply in [true, false] {
        let directory = project(SOURCE).unwrap();
        let mut writer = open(&directory).unwrap();
        writer.change(|model| {
            model.set_draft("Pending.".into());
            Ok(())
        });
        writer.start_trial();
        assert!(writer.trial_feedback().is_some());
        writer.return_to_trial_edit();
        writer.change(|model| {
            if apply {
                model.apply()
            } else {
                model.discard();
                Ok(())
            }
        });
        writer.configure_trial();
        assert!(writer.unfinished_trial_edit().is_none());
        assert!(writer.trial_feedback().is_none());
        assert!(writer.trial_status().is_none());
    }
}

#[test]
fn recovered_source_never_offers_manuscript_apply_or_enqueues_preview() {
    let directory = project(SOURCE).unwrap();
    let mut writer = open(&directory).unwrap();
    writer.change(|model| {
        model.select(recite_writer_model::View::Source)?;
        model.stage_repaired_source(":: recovered\n? unfinished".to_owned().into())
    });
    let recovery = writer.session.model().recovery();
    let (sender, mut requests) = async_mpsc::unbounded_channel();
    writer.trial.sender = Some(sender);
    writer.start_trial();
    assert!(writer.source_repair_pending());
    assert!(writer.unfinished_trial_edit().is_none());
    writer.apply_trial_edit();
    assert!(requests.try_recv().is_err());
    assert!(!writer.navigation.busy());
    assert_eq!(writer.session.model().recovery(), recovery);
}

#[test]
fn current_request_precedes_expanded_next_run_settings_without_replacing_either() {
    let directory =
        project(":: start default\n:if trusts(player)\n  > @ffee6p8\n    Hello.\n-> END\n")
            .unwrap();
    let mut writer = open(&directory).unwrap();
    install(&mut writer).unwrap();
    writer.configure_trial();
    let mut mounted = Mounted::new(writer, surface);
    let answer = mounted.find("Answer true");
    let settings = mounted.find("Preview wording");
    let bounds = |id| {
        let widget = mounted.harness.get_widget_with_id(id);
        widget
            .ctx()
            .window_transform()
            .transform_rect_bbox(widget.ctx().border_box())
    };
    assert!(bounds(answer).y0 < bounds(settings).y0);
    mounted.activate("Hide preview inputs");
    mounted.activate("Configure preview");
    assert_eq!(mounted.find("Preview wording"), settings);
    assert_eq!(mounted.find("Answer true"), answer);
}

#[test]
fn returning_to_an_unfinished_edit_preserves_split_and_reveals_script_from_map() {
    for workspace in [Workspace::Map, Workspace::Split] {
        let directory = project(SOURCE).unwrap();
        let mut writer = open(&directory).unwrap();
        writer.map.workspace = workspace;
        writer.change(|model| {
            model.set_draft("Pending.".into());
            Ok(())
        });
        writer.start_trial();
        writer.return_to_trial_edit();
        assert_eq!(
            writer.map.workspace,
            if workspace == Workspace::Map {
                Workspace::Script
            } else {
                Workspace::Split
            }
        );
        assert!(matches!(
            writer.attention.pane_request().unwrap().motion(),
            PaneMotion::To(Pane::Script)
        ));
        assert!(writer.attention.edit_request().is_some());
    }
}
