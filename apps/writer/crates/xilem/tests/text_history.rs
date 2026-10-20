//! Regressions for the retained Masonry text-history patch.
use masonry::core::keyboard::{Code, Key, KeyState};
use masonry::core::{Ime, KeyboardEvent, Modifiers, TextEvent};
use masonry::parley::fontique;
use masonry::testing::TestHarness;
use masonry::widgets::TextArea;

fn area(text: &str) -> TestHarness<TextArea<true>> {
    let root = masonry::core::NewWidget::new(
        TextArea::new_editable(text).with_insert_newline(masonry::widgets::InsertNewline::OnEnter),
    );
    let mut harness =
        TestHarness::create_with_size(masonry::theme::default_property_set(), root, (360, 180));
    harness.edit_root_widget(|mut widget| {
        let (fonts, _) = widget.ctx.text_contexts();
        let registered = fonts.collection.register_fonts(
            masonry::peniko::Blob::new(std::sync::Arc::new(
                include_bytes!("../../freya/fonts/literata/Literata-Variable.ttf").to_vec(),
            )),
            None,
        );
        fonts.collection.set_generic_families(
            fontique::GenericFamily::SansSerif,
            registered.into_iter().map(|(id, _)| id).take(1),
        );
        TextArea::insert_style(&mut widget, fontique::GenericFamily::SansSerif);
    });
    harness.focus_on(Some(harness.root_id()));
    harness.edit_root_widget(|mut widget| {
        TextArea::select_byte_range(&mut widget, text.len(), text.len())
    });
    harness.redraw();
    harness
}

fn shortcut(harness: &mut TestHarness<TextArea<true>>, name: &str, shift: bool) {
    let mut modifiers = Modifiers::CONTROL;
    if shift {
        modifiers |= Modifiers::SHIFT;
    }
    harness.process_text_event(TextEvent::Keyboard(KeyboardEvent {
        state: KeyState::Down,
        key: Key::Character(name.into()),
        code: if name == "z" { Code::KeyZ } else { Code::KeyY },
        modifiers,
        ..KeyboardEvent::default()
    }));
}

fn text(harness: &TestHarness<TextArea<true>>) -> String {
    harness.root_widget().text().to_string()
}

#[test]
fn unbound_command_chords_do_not_insert_text_but_option_and_alt_graph_do() {
    let mut harness = area("Start");
    for modifiers in [
        Modifiers::CONTROL,
        Modifiers::META,
        Modifiers::CONTROL | Modifiers::ALT,
    ] {
        for name in ["s", " "] {
            harness.process_text_event(TextEvent::Keyboard(KeyboardEvent {
                state: KeyState::Down,
                key: Key::Character(name.into()),
                modifiers,
                ..KeyboardEvent::default()
            }));
        }
    }
    assert_eq!(text(&harness), "Start");
    for modifiers in [Modifiers::ALT, Modifiers::CONTROL | Modifiers::ALT_GRAPH] {
        harness.process_text_event(TextEvent::Keyboard(KeyboardEvent {
            state: KeyState::Down,
            key: Key::Character("€".into()),
            modifiers,
            ..KeyboardEvent::default()
        }));
    }
    assert_eq!(text(&harness), "Start€€");
    harness.process_text_event(TextEvent::Keyboard(KeyboardEvent {
        state: KeyState::Down,
        key: Key::Character("z".into()),
        modifiers: Modifiers::CONTROL | Modifiers::ALT_GRAPH,
        ..KeyboardEvent::default()
    }));
    assert_eq!(text(&harness), "Start€€z");
    shortcut(&mut harness, "z", false);
    assert_eq!(text(&harness), "Start€€");
    shortcut(&mut harness, "z", false);
    assert_eq!(text(&harness), "Start€");
    shortcut(&mut harness, "z", false);
    assert_eq!(text(&harness), "Start");
}

#[test]
fn typing_undo_redo_and_new_branch_preserve_edits() {
    let mut harness = area("Start");
    harness.keyboard_type_chars("xy");
    shortcut(&mut harness, "z", false);
    assert_eq!(text(&harness), "Startx");
    shortcut(&mut harness, "z", true);
    assert_eq!(text(&harness), "Startxy");
    shortcut(&mut harness, "z", false);
    harness.keyboard_type_chars("Z");
    shortcut(&mut harness, "y", false);
    assert_eq!(text(&harness), "StartxZ");
}

#[test]
fn composition_is_one_undo_transaction_and_restores_replaced_selection() {
    let mut harness = area("Café last");
    harness.edit_root_widget(|mut widget| TextArea::select_byte_range(&mut widget, 6, 10));
    harness.process_text_event(TextEvent::Ime(Ime::Preedit("にほん".into(), Some((9, 9)))));
    harness.process_text_event(TextEvent::Ime(Ime::Preedit("日本".into(), Some((6, 6)))));
    harness.process_text_event(TextEvent::Ime(Ime::Commit("日本".into())));
    assert_eq!(text(&harness), "Café 日本");
    shortcut(&mut harness, "z", false);
    assert_eq!(text(&harness), "Café last");
    shortcut(&mut harness, "c", false);
    assert_eq!(harness.clipboard_contents(), "last");
    shortcut(&mut harness, "y", false);
    assert_eq!(text(&harness), "Café 日本");
}

#[test]
fn paste_and_cut_are_independent_unicode_transactions() {
    let mut harness = area("Start");
    let pasted = "StartCafé 日本 🐈\nمرحبا";
    harness.process_text_event(TextEvent::ClipboardPaste("Café 日本 🐈\nمرحبا".into()));
    assert_eq!(text(&harness), pasted);
    shortcut(&mut harness, "a", false);
    shortcut(&mut harness, "x", false);
    assert_eq!(text(&harness), "");
    shortcut(&mut harness, "z", false);
    assert_eq!(text(&harness), pasted);
    shortcut(&mut harness, "z", false);
    assert_eq!(text(&harness), "Start");
    shortcut(&mut harness, "y", false);
    assert_eq!(text(&harness), pasted);
}

#[test]
fn replacing_passage_text_clears_widget_history() {
    let mut harness = area("Start");
    harness.keyboard_type_chars("x");
    harness.edit_root_widget(|mut widget| TextArea::reset_text(&mut widget, "Other passage"));
    shortcut(&mut harness, "z", false);
    assert_eq!(text(&harness), "Other passage");
}

#[test]
fn cancelled_preedit_does_not_add_an_undo_step() {
    let mut harness = area("Start");
    harness.keyboard_type_chars("x");
    harness.process_text_event(TextEvent::Ime(Ime::Preedit("未確定".into(), Some((9, 9)))));
    harness.process_text_event(TextEvent::Ime(Ime::Preedit(String::new(), None)));
    assert_eq!(text(&harness), "Startx");
    shortcut(&mut harness, "z", false);
    assert_eq!(text(&harness), "Start");
}

#[test]
fn cancellation_restores_selected_text_and_commit_after_empty_preedit_undoes_once() {
    let mut harness = area("Café last");
    harness.edit_root_widget(|mut widget| TextArea::select_byte_range(&mut widget, 6, 10));
    harness.process_text_event(TextEvent::Ime(Ime::Preedit("日本".into(), Some((6, 6)))));
    harness.process_text_event(TextEvent::Ime(Ime::Preedit(String::new(), None)));
    assert_eq!(text(&harness), "Café last");
    // Winit commonly clears preedit immediately before delivering the commit.
    harness.process_text_event(TextEvent::Ime(Ime::Commit("日本".into())));
    assert_eq!(text(&harness), "Café 日本");
    shortcut(&mut harness, "z", false);
    assert_eq!(text(&harness), "Café last");
}

#[test]
fn retained_history_is_bounded_and_exhaustion_does_not_type_shortcuts() {
    let mut harness = area("Start");
    for _ in 0..300 {
        harness.process_text_event(TextEvent::Ime(Ime::Commit("x".into())));
    }
    for _ in 0..105 {
        shortcut(&mut harness, "z", false);
    }
    assert_eq!(text(&harness), format!("Start{}", "x".repeat(200)));
    for _ in 0..105 {
        shortcut(&mut harness, "y", false);
    }
    assert_eq!(text(&harness), format!("Start{}", "x".repeat(300)));
}
