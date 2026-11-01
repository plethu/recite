use super::*;
use crate::{
    attention::EditingTarget,
    keyed::keyed,
    tests::support::{self, mounted::Mounted},
};
use recite_writer_model::{View, Workbench};
use std::sync::Arc;
use tokio::sync::mpsc;
use xilem::{
    WidgetView,
    masonry::{
        core::{
            TextEvent,
            keyboard::{Code, Key, KeyState, KeyboardEvent, Modifiers},
        },
        widgets::Passthrough,
    },
};

fn source() -> String {
    let mut source = ":: start default\n".to_owned();
    for n in 1..=40 {
        source.push_str(&format!("> @line_{n}\n  Line {n}.\n"));
    }
    source.push_str("-> END\n");
    source
}
fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed((), crate::view::writer(writer))
}

#[test]
fn overlapping_shift_keeps_the_native_editor_undo_and_unapplied_source() {
    let directory = support::project(&source()).unwrap();
    let mut writer = support::open(&directory).unwrap();
    writer.native_field_focus(Some(EditingTarget::passage(&writer, format!("line_{}", 9))));
    let original = writer.session.model().document().source_snapshot();
    let mut mounted = Mounted::new(writer, surface);
    mounted.replace_text("Manuscript", "Pending ninth passage.");
    let field = mounted.find("Manuscript");
    for direction in [Direction::Next, Direction::Previous, Direction::Next] {
        mounted.state.shift_reading(direction);
        mounted.settle();
        assert_eq!(mounted.find("Manuscript"), field);
        assert_eq!(
            mounted.state.session.model().draft(),
            "Pending ninth passage."
        );
        assert!(Arc::ptr_eq(
            &original,
            &mounted.state.session.model().document().source_snapshot()
        ));
        assert!(!mounted.state.navigation.busy());
    }
    mounted.type_text("Manuscript", "x");
    mounted
        .harness
        .process_text_event(TextEvent::Keyboard(KeyboardEvent {
            state: KeyState::Down,
            key: Key::Character("z".into()),
            code: Code::KeyZ,
            modifiers: if cfg!(target_os = "macos") {
                Modifiers::META
            } else {
                Modifiers::CONTROL
            },
            ..KeyboardEvent::default()
        }));
    mounted.settle();
    assert_eq!(
        mounted.state.session.model().draft(),
        "Pending ninth passage."
    );
    assert_eq!(mounted.state.reading.window.range(41), 8..24);
}

#[test]
fn eviction_waits_for_checked_admission_and_rejected_or_newer_drafts_keep_the_old_window() {
    for outcome in ["valid", "invalid", "newer", "cancel"] {
        let directory = support::project(&source()).unwrap();
        let mut writer = support::open(&directory).unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let (sender, mut jobs) = mpsc::unbounded_channel();
        writer.navigation.sender = Some(sender);
        writer.navigation.runtime = Some(runtime.handle().clone());
        let draft = if outcome == "invalid" {
            "-> syntax"
        } else {
            "Keep first passage."
        };
        writer.change(|model| {
            model.set_draft(draft.into());
            Ok(())
        });
        writer.shift_reading(Direction::Next);
        assert_eq!(writer.reading.window.range(41), 0..16);
        assert!(writer.navigation.busy());
        let completed = jobs.try_recv().unwrap().execute();
        if outcome == "newer" {
            writer.change(|model| {
                model.set_draft("Newer first passage.".into());
                Ok(())
            });
        } else if outcome == "cancel" {
            writer.cancel_navigation();
        }
        let request = completed.request.clone();
        if let Some(retired) = writer.accept_navigation(completed) {
            retired.retire().unwrap();
        }
        writer.navigation_retired(&request);
        support::finish_navigation(&mut writer, &mut jobs).unwrap();
        if outcome == "valid" {
            assert_eq!(writer.reading.window.range(41), 8..24);
            assert_eq!(
                writer.session.model().view(),
                &View::Passage(format!("line_{}", 9))
            );
            assert!(writer.session.model().document().source().contains(draft));
        } else {
            assert_eq!(writer.reading.window.range(41), 0..16);
            assert_eq!(
                writer.session.model().view(),
                &View::Passage(format!("line_{}", 1))
            );
            assert_eq!(
                writer.session.model().draft(),
                if outcome == "newer" {
                    "Newer first passage."
                } else {
                    draft
                }
            );
            if outcome == "cancel" {
                assert!(writer.reading_refusal().is_none());
                assert!(writer.status.contains("cancelled"));
            }
        }
    }
}

#[test]
fn preedit_started_during_preparation_prevents_a_prepared_window_from_retiring_its_field() {
    let directory = support::project(&source()).unwrap();
    let mut writer = support::open(&directory).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut jobs) = mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    writer.shift_reading(Direction::Next);
    let completed = jobs.try_recv().unwrap().execute();
    let preedit = writer.attention.begin_preedit();
    let request = completed.request.clone();
    if let Some(retired) = writer.accept_navigation(completed) {
        retired.retire().unwrap();
    }
    writer.navigation_retired(&request);
    assert_eq!(writer.reading.window.range(41), 0..16);
    assert_eq!(
        writer.session.model().view(),
        &View::Passage(format!("line_{}", 1))
    );
    assert!(writer.reading_refusal().is_none());
    assert!(writer.status.contains("composition"));
    assert!(jobs.try_recv().is_err());
    drop(preedit);
    writer.shift_reading(Direction::Next);
    support::finish_navigation(&mut writer, &mut jobs).unwrap();
    assert_eq!(writer.reading.window.range(41), 8..24);
}

#[test]
fn a_refusal_is_local_to_its_exact_draft_and_old_reason_actions_cannot_expand_a_replacement() {
    let directory = support::project(&source()).unwrap();
    let mut writer = support::open(&directory).unwrap();
    writer.change(|model| {
        model.set_draft("-> syntax".into());
        Ok(())
    });
    support::click(&mut writer, &["Next beat entries"]).unwrap();
    let names = support::labels(&mut writer).unwrap();
    assert!(names.iter().any(|name| name == "Can't continue yet"));
    assert!(names.iter().any(|name| name == "Continue paused"));
    assert!(!names.iter().any(|name| name == "no problems"));
    let previous = writer.reading_refusal().unwrap().request();
    writer.explain_reading_refusal(&previous);
    assert!(writer.reading_refusal().unwrap().reason().is_some());
    writer.change(|model| {
        model.set_draft("Edited text.".into());
        Ok(())
    });
    assert!(writer.reading_refusal().is_none());
    writer.change(|model| {
        model.set_draft("-> another".into());
        Ok(())
    });
    support::click(&mut writer, &["Next beat entries"]).unwrap();
    writer.explain_reading_refusal(&previous);
    assert!(!writer.reading_refusal().unwrap().expanded());
}

#[test]
fn replacing_a_reading_projection_with_fewer_rows_reveals_a_useful_remaining_range() {
    let directory = support::project(&source()).unwrap();
    let mut writer = support::open(&directory).unwrap();
    writer.change(|model| model.select(View::Block("start".into())));
    writer.reading.window.reveal(39);
    writer.change(|model| {
        let mut shorter = Workbench::open(
            model.document().key().as_str(),
            ":: start default\n> @only\n  A shorter beat.\n-> END\n",
        )?;
        shorter.select(View::Block("start".into()))?;
        *model = shorter;
        Ok(())
    });
    assert_eq!(writer.reading.window.range(2), 0..2);
    assert_eq!(writer.reading.rows.len(), 2);
}
