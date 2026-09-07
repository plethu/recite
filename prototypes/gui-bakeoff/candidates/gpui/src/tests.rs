use super::{Command, FIXTURE, View, Workbench, WorkbenchView, initialize};
use gpui::{EntityInputHandler, TestAppContext};

#[gpui::test]
fn prose_composition_and_source_round_trip(cx: &mut TestAppContext) {
    cx.update(initialize);
    let view = cx.add_window(|window, cx| {
        WorkbenchView::new(
            Workbench::new(FIXTURE).unwrap_or_else(|e| panic!("{e}")),
            window,
            cx,
        )
    });
    view.update(cx, |state, window, cx| {
        state.prose.update(cx, |input, cx| {
            input.set_value("", window, cx);
            input.replace_and_mark_text_in_range(None, "にほん", Some(3..3), window, cx);
            assert!(input.marked_text_range(window, cx).is_some());
            input.replace_text_in_range(None, "日本\nCafé 🐈", window, cx);
            assert_eq!(input.value().as_ref(), "日本\nCafé 🐈");
        });
        state.command(Command::Source, window, cx);
        assert_ne!(state.model.view(), &View::Source);
        assert!(state.status.contains("Apply or discard"));
        state.command(Command::Apply, window, cx);
        state.command(Command::Source, window, cx);
        assert_eq!(state.model.view(), &View::Source, "{}", state.status);
        assert!(state.source.read(cx).value().contains("日本"));
        assert!(state.source.read(cx).value().contains("Café 🐈"));
        assert!(
            state
                .source
                .read(cx)
                .value()
                .contains("7701ceab59d2adfa057a")
        );
        state.command(Command::Script, window, cx);
        assert_eq!(state.prose.read(cx).value().as_ref(), "日本\nCafé 🐈");
    })
    .unwrap_or_else(|e| panic!("{e}"));
}

#[gpui::test]
fn recite_source_uses_the_existing_grammar(cx: &mut TestAppContext) {
    use gpui_component::{
        highlighter::{HighlightTheme, SyntaxHighlighter},
        input::Rope,
    };
    cx.update(initialize);
    let mut highlighter = SyntaxHighlighter::new("recite");
    assert!(highlighter.update(None, &Rope::from(FIXTURE), None));
    assert!(highlighter.tree().is_some());
    let theme = HighlightTheme::default_light();
    let styles = highlighter.styles(&(0..FIXTURE.len()), theme.as_ref());
    assert!(styles.iter().any(|(_, style)| style.color.is_some()));
}

#[gpui::test]
fn writer_controls_preserve_ids_and_preview_tracks_drafts(cx: &mut TestAppContext) {
    cx.update(initialize);
    let view = cx.add_window(|window, cx| {
        WorkbenchView::new(
            Workbench::new(FIXTURE).unwrap_or_else(|e| panic!("{e}")),
            window,
            cx,
        )
    });
    view.update(cx, |state, window, cx| {
        let original_id = state.model.selected().unwrap_or_else(|e| panic!("{e}"))
            .unwrap_or_else(|| panic!("missing passage")).id;
        state.command(Command::Attribute("cheshire_cat".into()), window, cx);
        let passage = state.model.selected().unwrap_or_else(|e| panic!("{e}"))
            .unwrap_or_else(|| panic!("missing passage"));
        assert_eq!(passage.id, original_id);
        assert!(matches!(passage.kind, recite_bakeoff_authoring::PassageKind::Dialogue { speaker: Some(ref s) } if s == "cheshire_cat"));
        state.command(Command::Undo, window, cx);
        assert_eq!(state.model.document().source(), FIXTURE);
        state.command(Command::Redo, window, cx);
        state.command(Command::AddChoice, window, cx);
        let new_id = state.model.selected().unwrap_or_else(|e| panic!("{e}"))
            .unwrap_or_else(|| panic!("missing choice")).id;
        assert_ne!(new_id, original_id);
        state.command(Command::Attribute("garden".into()), window, cx);
        assert!(matches!(state.model.selected().unwrap_or_else(|e| panic!("{e}"))
            .unwrap_or_else(|| panic!("missing choice")).kind,
            recite_bakeoff_authoring::PassageKind::Choice { destination: Some(ref d) } if d == "garden"));
        state.command(Command::Preview, window, cx);
        assert!(state.model.preview_page().is_some(), "{}", state.status);
        assert!(!state.model.preview_stale());
        state.prose.update(cx, |input, cx| input.replace_text_in_range(Some(0..input.value().encode_utf16().count()), "A different route", window, cx));
    }).unwrap_or_else(|e| panic!("{e}"));
    // Input events are delivered after the entity update, as in the application.
    view.update(cx, |state, window, cx| {
        assert!(state.model.has_draft());
        assert!(state.model.preview_stale());
        state.command(Command::Source, window, cx);
        assert_ne!(state.model.view(), &View::Source);
        state.command(Command::Discard, window, cx);
        assert!(!state.model.has_draft());
        assert!(!state.model.preview_stale());
    })
    .unwrap_or_else(|e| panic!("{e}"));
}

#[gpui::test]
fn unfinished_composition_cannot_be_applied(cx: &mut TestAppContext) {
    cx.update(initialize);
    let view = cx.add_window(|window, cx| {
        WorkbenchView::new(
            Workbench::new(FIXTURE).unwrap_or_else(|e| panic!("{e}")),
            window,
            cx,
        )
    });
    view.update(cx, |state, window, cx| {
        state.prose.update(cx, |input, cx| {
            input.replace_and_mark_text_in_range(Some(0..0), "にほん", Some(3..3), window, cx);
        });
        state.command(Command::Apply, window, cx);
        assert_eq!(state.model.document().source(), FIXTURE);
        assert!(!state.model.has_draft());
        assert!(state.status.contains("composition"));
    })
    .unwrap_or_else(|e| panic!("{e}"));
}

#[gpui::test]
fn keyboard_undo_and_clipboard_do_not_cross_passages(cx: &mut TestAppContext) {
    use gpui::{ClipboardItem, Focusable};
    cx.update(initialize);
    let view = cx.add_window(|window, cx| {
        WorkbenchView::new(
            Workbench::new(FIXTURE).unwrap_or_else(|e| panic!("{e}")),
            window,
            cx,
        )
    });
    view.update(cx, |state, window, cx| {
        window.focus(&state.prose.focus_handle(cx), cx);
    })
    .unwrap_or_else(|e| panic!("{e}"));
    cx.run_until_parked();
    cx.update(|cx| cx.write_to_clipboard(ClipboardItem::new_string("Café 🐈\n日本".into())));
    cx.simulate_keystrokes(view.into(), "ctrl-a ctrl-v");
    view.update(cx, |state, _, cx| {
        assert_eq!(state.prose.read(cx).value().as_ref(), "Café 🐈\n日本");
    })
    .unwrap_or_else(|e| panic!("{e}"));
    cx.simulate_keystrokes(view.into(), "ctrl-z");
    view.update(cx, |state, window, cx| {
        assert!(state.prose.read(cx).value().contains("Would you tell me"));
        // Keep an undoable edit when switching; an empty stack would prove nothing.
        state.prose.update(cx, |input, cx| {
            input.replace_text_in_range(
                Some(0..input.value().encode_utf16().count()),
                "An applied revision.",
                window,
                cx,
            )
        });
        state.command(Command::Apply, window, cx);
        let next = state
            .model
            .document()
            .passages()
            .unwrap_or_else(|e| panic!("{e}"))[1]
            .id
            .clone();
        state.command(Command::Select(next), window, cx);
        window.focus(&state.prose.focus_handle(cx), cx);
    })
    .unwrap_or_else(|e| panic!("{e}"));
    cx.run_until_parked();
    cx.simulate_keystrokes(view.into(), "ctrl-z");
    view.update(cx, |state, _, cx| {
        assert_eq!(
            state.prose.read(cx).value().as_ref(),
            "That depends a good deal on where you want to get to."
        );
        assert!(!state.model.has_draft());
    })
    .unwrap_or_else(|e| panic!("{e}"));
}

#[gpui::test]
fn rejected_prose_is_recoverable_and_preview_reaches_both_branches(cx: &mut TestAppContext) {
    cx.update(initialize);
    let view = cx.add_window(|window, cx| {
        WorkbenchView::new(
            Workbench::new(FIXTURE).unwrap_or_else(|e| panic!("{e}")),
            window,
            cx,
        )
    });
    view.update(cx, |state, window, cx| {
        state.prose.update(cx, |input, cx| {
            input.set_value("hello\n:: injected", window, cx)
        });
        state.command(Command::Apply, window, cx);
        assert!(state.model.has_draft());
        assert_eq!(state.model.document().source(), FIXTURE);
        assert!(!state.status.starts_with("Applied."));
        state.command(Command::Discard, window, cx);
        assert!(!state.model.has_draft());
        for (choice, expected) in [(0, "Then it doesn't matter"), (1, "Then I shall")] {
            state.command(Command::Preview, window, cx);
            for _ in 0..8 {
                if state
                    .model
                    .preview_page()
                    .is_some_and(|p| !p.choices.is_empty())
                {
                    break;
                }
                state.command(Command::Advance(None), window, cx);
            }
            assert_eq!(state.model.preview_page().map(|p| p.choices.len()), Some(2));
            state.command(Command::Advance(Some(choice)), window, cx);
            assert!(
                state
                    .model
                    .preview_page()
                    .is_some_and(|p| p.text.contains(expected)),
                "{}",
                state.status
            );
        }
    })
    .unwrap_or_else(|e| panic!("{e}"));
}

#[gpui::test]
fn tab_leaves_prose_without_inserting_a_character(cx: &mut TestAppContext) {
    use gpui::{AppContext, Focusable};
    use gpui_component::Root;
    cx.update(initialize);
    let mut entity = None;
    let window = cx.add_window(|window, cx| {
        let view = cx.new(|cx| {
            WorkbenchView::new(
                Workbench::new(FIXTURE).unwrap_or_else(|e| panic!("{e}")),
                window,
                cx,
            )
        });
        entity = Some(view.clone());
        Root::new(view, window, cx)
    });
    let entity = entity.unwrap_or_else(|| panic!("missing workbench"));
    window
        .update(cx, |_, window, cx| {
            window.focus(&entity.read(cx).prose.focus_handle(cx), cx);
        })
        .unwrap_or_else(|e| panic!("{e}"));
    cx.run_until_parked();
    cx.simulate_keystrokes(window.into(), "tab");
    window
        .update(cx, |_, window, cx| {
            let state = entity.read(cx);
            assert!(!state.prose.focus_handle(cx).is_focused(window));
            assert_eq!(state.prose.read(cx).value().as_ref(), state.model.draft());
        })
        .unwrap_or_else(|e| panic!("{e}"));
    cx.simulate_keystrokes(window.into(), "shift-tab");
    window
        .update(cx, |_, window, cx| {
            assert!(entity.read(cx).prose.focus_handle(cx).is_focused(window));
        })
        .unwrap_or_else(|e| panic!("{e}"));
}
