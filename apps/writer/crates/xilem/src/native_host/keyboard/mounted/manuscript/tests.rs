use super::*;
use crate::attention::{EditingTarget, PassageField};
use xilem::masonry::core::{Ime as RootIme, TextEvent};
use xilem::masonry::widgets::TextArea;
use xilem::winit::keyboard::{Key as NativeKey, KeyCode, PhysicalKey};

fn fields() -> (
    RenderRoot,
    mpsc::Receiver<RenderRootSignal>,
    EditingTarget,
    EditingTarget,
) {
    let (mut root, signals) = mounted();
    root.handle_text_event(TextEvent::WindowFocusChange(true));
    let writer = Writer::from_session(Session::examples().unwrap(), None);
    let a = EditingTarget::capture(&writer).unwrap();
    let b = EditingTarget::passage(&writer, "another".into());
    for (tag, target) in [(SCRIPT, &a), (NAVIGATION, &b)] {
        let id = root.get_widget_with_tag(tag).unwrap().id();
        root.edit_widget(id, |mut widget| {
            widget.insert_prop(PassageField(Some(target.clone())));
        });
    }
    let (mut context, _) = context();
    context.editing = Some(a.clone());
    input::scope(&mut root, context);
    (root, signals, a, b)
}
fn actions(signals: &mpsc::Receiver<RenderRootSignal>) -> Vec<Action> {
    signals
        .try_iter()
        .filter_map(|signal| {
            let RenderRootSignal::Action(action, _) = signal else {
                return None;
            };
            action
                .downcast::<Invocation>()
                .ok()
                .map(|invocation| invocation.action)
        })
        .collect()
}

#[test]
fn restoration_acknowledges_the_already_focused_field_once_and_allows_the_next_switch() {
    let (mut root, signals, a, b) = fields();
    let mut keyboard = Keyboard::default();
    keyboard.synchronize(&mut root);
    assert_eq!(actions(&signals), [Action::FieldFocus(Some(a.clone()))]);
    let (mut restoring, _) = context();
    restoring.editing = Some(a.clone());
    restoring.field_restoration = Some(a.clone());
    input::scope(&mut root, restoring);
    keyboard.synchronize(&mut root);
    assert_eq!(actions(&signals), [Action::FieldFocus(Some(a.clone()))]);
    let (mut context, _) = context();
    context.editing = Some(a);
    input::scope(&mut root, context);
    keyboard.synchronize(&mut root);
    assert!(actions(&signals).is_empty());
    root.focus_on(Some(field(&root, NAVIGATION)));
    keyboard.synchronize(&mut root);
    assert_eq!(actions(&signals), [Action::FieldFocus(Some(b))]);
}

#[test]
fn blur_clears_the_latest_intent_even_though_masonry_retains_native_focus() {
    let (mut root, signals, _, b) = fields();
    let mut keyboard = Keyboard::default();
    keyboard.synchronize(&mut root);
    actions(&signals);
    let field = field(&root, NAVIGATION);
    root.focus_on(Some(field));
    keyboard.synchronize(&mut root);
    assert_eq!(actions(&signals), [Action::FieldFocus(Some(b))]);
    keyboard.event(&mut root, &WindowEvent::Focused(false));
    root.handle_text_event(TextEvent::WindowFocusChange(false));
    keyboard.synchronize(&mut root);
    assert_eq!(root.focused_widget(), Some(field));
    assert_eq!(actions(&signals), [Action::FieldFocus(None)]);
    keyboard.synchronize(&mut root);
    assert!(actions(&signals).is_empty());
}

#[test]
fn composing_text_stays_with_its_original_native_field_until_commit_or_cancel() {
    for commit in [false, true] {
        let (mut root, signals, _, _) = fields();
        let mut keyboard = Keyboard::default();
        keyboard.synchronize(&mut root);
        actions(&signals);
        let a = field(&root, SCRIPT);
        let b = field(&root, NAVIGATION);
        let original = root
            .get_widget(a)
            .unwrap()
            .downcast::<TextArea<true>>()
            .unwrap()
            .text()
            .to_string();
        keyboard.event(&mut root, &WindowEvent::Ime(Ime::Preedit("é".into(), None)));
        root.handle_text_event(TextEvent::Ime(RootIme::Preedit("é".into(), None)));
        root.focus_on(Some(b));
        keyboard.synchronize(&mut root);
        assert_eq!(root.focused_widget(), Some(a));
        assert!(
            actions(&signals).is_empty(),
            "preedit must not request an implicit Apply"
        );
        if commit {
            keyboard.event(&mut root, &WindowEvent::Ime(Ime::Commit("é".into())));
            root.handle_text_event(TextEvent::Ime(RootIme::Commit("é".into())));
        } else {
            keyboard.event(&mut root, &WindowEvent::Ime(Ime::Disabled));
            root.handle_text_event(TextEvent::Ime(RootIme::Disabled));
        }
        let text = root
            .get_widget(a)
            .unwrap()
            .downcast::<TextArea<true>>()
            .unwrap()
            .text()
            .to_string();
        assert_eq!(
            text,
            if commit {
                format!("é{original}")
            } else {
                original
            }
        );
        assert_eq!(
            root.get_widget(b)
                .unwrap()
                .downcast::<TextArea<true>>()
                .unwrap()
                .text(),
            "Scene names"
        );
    }
}

#[test]
fn an_unadmitted_press_owns_its_release_after_focus_moves_to_a_button() {
    for (key, code) in [
        (NativeKey::Named(NamedKey::Enter), KeyCode::Enter),
        (NativeKey::Character(" ".into()), KeyCode::Space),
        (NativeKey::Character("v".into()), KeyCode::KeyV),
    ] {
        let (mut root, signals, _, _) = fields();
        let mut keyboard = Keyboard::default();
        keyboard.synchronize(&mut root);
        actions(&signals);
        root.focus_on(Some(field(&root, NAVIGATION)));
        let press = Keystroke {
            logical_key: &key,
            physical_key: PhysicalKey::Code(code),
            state: ElementState::Pressed,
            repeat: false,
        };
        assert!(keyboard.key_event(&root, press, false));
        root.focus_on(Some(root.get_widget_with_tag(MAP).unwrap().id()));
        assert!(keyboard.key_event(
            &root,
            Keystroke {
                state: ElementState::Released,
                ..press
            },
            false
        ));
        assert!(actions(&signals).is_empty());
    }
}
