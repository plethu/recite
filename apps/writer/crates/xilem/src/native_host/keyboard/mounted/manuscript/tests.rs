use super::super::super::manuscript as native_fields;
use super::*;
use crate::attention::{EditingTarget, FieldRestoration, PassageField};
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
fn draft_authority_does_not_steal_focus_and_explicit_reveal_is_consumed_once() {
    let (mut root, _signals, a, b) = fields();
    let selector = root.get_widget_with_tag(MAP).unwrap().id();
    root.focus_on(Some(selector));
    let mut focus = focus::Focus::default();
    let mut attention = crate::attention::Attention::default();
    focus.synchronize_manuscript(&mut root, Some(&a), None);
    assert_eq!(root.focused_widget(), Some(selector));
    attention.request_edit(a.clone());
    focus.synchronize_manuscript(&mut root, Some(&a), attention.edit_request());
    assert_eq!(root.focused_widget(), Some(field(&root, SCRIPT)));
    root.focus_on(Some(selector));
    focus.synchronize_manuscript(&mut root, Some(&a), attention.edit_request());
    assert_eq!(root.focused_widget(), Some(selector));
    // A retained request for another admitted passage cannot redirect focus.
    focus.synchronize_manuscript(&mut root, Some(&b), attention.edit_request());
    assert_eq!(root.focused_widget(), Some(selector));
    attention.request_edit(b.clone());
    focus.synchronize_manuscript(&mut root, Some(&b), attention.edit_request());
    assert_eq!(root.focused_widget(), Some(field(&root, NAVIGATION)));
    focus.synchronize_manuscript(&mut root, None, None);
    assert_eq!(root.focused_widget(), None);
}

#[test]
fn restoration_acknowledges_the_already_focused_field_once_and_allows_the_next_switch() {
    let (mut root, signals, a, b) = fields();
    let mut keyboard = Keyboard::default();
    keyboard.synchronize(&mut root);
    assert_eq!(actions(&signals), [Action::FieldFocus(Some(a.clone()))]);
    let script = field(&root, SCRIPT);
    root.edit_widget(script, |mut widget| {
        TextArea::<true>::select_byte_range(&mut widget.downcast(), 1, 4)
    });
    let (mut stale_clear, _) = context();
    stale_clear.editing = Some(a.clone());
    stale_clear.field_restoration = Some(FieldRestoration::Clear);
    input::scope(&mut root, stale_clear);
    keyboard.synchronize(&mut root);
    assert_eq!(
        root.focused_widget(),
        Some(script),
        "an old clear request cannot dismiss a newly admitted field"
    );
    assert_eq!(actions(&signals), [Action::FieldFocus(Some(a.clone()))]);
    let (mut stale, _) = context();
    stale.editing = Some(a.clone());
    stale.field_restoration = Some(FieldRestoration::Passage(b.clone()));
    input::scope(&mut root, stale);
    keyboard.synchronize(&mut root);
    assert_eq!(
        root.focused_widget(),
        Some(script),
        "a stale repair cannot change the admitted passage"
    );
    assert_eq!(actions(&signals), [Action::FieldFocus(Some(a.clone()))]);
    let (mut restoring, _) = context();
    restoring.editing = Some(a.clone());
    restoring.field_restoration = Some(FieldRestoration::Passage(a.clone()));
    input::scope(&mut root, restoring);
    keyboard.synchronize(&mut root);
    assert_eq!(actions(&signals), [Action::FieldFocus(Some(a.clone()))]);
    root.handle_text_event(TextEvent::ClipboardPaste("X".into()));
    assert_eq!(
        root.get_widget(script)
            .unwrap()
            .downcast::<TextArea<true>>()
            .unwrap()
            .text(),
        "KX this manuscript",
        "restoration preserves the original selection"
    );
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

#[test]
fn refused_focus_without_an_admitted_passage_clears_only_prose_and_acknowledges_once() {
    for prose_focused in [true, false] {
        let (mut root, signals, _, _) = fields();
        let mut keyboard = Keyboard::default();
        keyboard.synchronize(&mut root);
        actions(&signals);
        let destination = if prose_focused {
            field(&root, SCRIPT)
        } else {
            root.get_widget_with_tag(MAP).unwrap().id()
        };
        root.focus_on(Some(destination));
        let (mut clearing, _) = context();
        clearing.field_restoration = Some(FieldRestoration::Clear);
        input::scope(&mut root, clearing);
        keyboard.synchronize(&mut root);
        assert_eq!(
            root.focused_widget(),
            (!prose_focused).then_some(destination)
        );
        assert_eq!(actions(&signals), [Action::FieldFocus(None)]);
        let (idle, _) = context();
        input::scope(&mut root, idle);
        keyboard.synchronize(&mut root);
        assert!(actions(&signals).is_empty());
        assert!(!native_fields::unadmitted(&root));
    }
}

#[test]
fn picker_fallback_uses_only_the_explicitly_admitted_passage() {
    let (mut root, _signals, a, _) = fields();
    let (mut context, _) = context();
    context.editing = Some(a);
    let mut focus = focus::Focus::default();
    root.focus_on(Some(field(&root, NAVIGATION)));
    focus.synchronize(&mut root, &context);
    context.picker_open = true;
    focus.synchronize(&mut root, &context);
    let origin = root.get_widget_with_tag(NAVIGATION).unwrap().id();
    root.edit_widget(origin, |mut widget| widget.ctx.set_disabled(true));
    context.picker_open = false;
    focus.synchronize(&mut root, &context);
    assert_eq!(root.focused_widget(), Some(field(&root, SCRIPT)));
}
