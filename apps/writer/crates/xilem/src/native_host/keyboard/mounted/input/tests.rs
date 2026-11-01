use super::*;
use recite_config::Keymap;
use xilem::winit::keyboard::{Key as NativeKey, KeyCode, PhysicalKey as NativePhysical};

pub(super) fn scope(root: &mut RenderRoot, context: Context) {
    let id = root.get_widget_with_tag(TAG).unwrap().id();
    root.edit_widget(id, |mut widget| {
        widget.insert_prop(KeyboardContext(Some(Arc::new(context))));
    });
}
fn press(key: &NativeKey, physical: KeyCode) -> Keystroke<'_> {
    Keystroke {
        logical_key: key,
        physical_key: NativePhysical::Code(physical),
        state: ElementState::Pressed,
        repeat: false,
    }
}
fn action(messages: &mpsc::Receiver<SendMessage>) -> Action {
    messages
        .try_recv()
        .unwrap()
        .downcast::<Invocation>()
        .unwrap()
        .action
}
fn primary() -> ModifiersState {
    if cfg!(target_os = "macos") {
        ModifiersState::SUPER
    } else {
        ModifiersState::CONTROL
    }
}

#[test]
fn document_menu_escape_owns_its_release_and_yields_to_an_open_picker() {
    let (mut root, _signals) = mounted();
    let (mut context, messages) = context();
    context.document_menu = crate::attention::DocumentMenu::Open;
    scope(&mut root, context);
    let mut keyboard = Keyboard::default();
    let escape = NativeKey::Named(NamedKey::Escape);
    let key = press(&escape, KeyCode::Escape);
    assert!(keyboard.key_event(&root, key, false));
    assert_eq!(action(&messages), Action::DismissDocumentMenu);
    let (context, _) = super::context();
    scope(&mut root, context);
    assert!(keyboard.key_event(
        &root,
        Keystroke {
            state: ElementState::Released,
            ..key
        },
        false
    ));
    assert!(messages.try_recv().is_err());
    let (mut context, _) = super::context();
    context.document_menu = crate::attention::DocumentMenu::Open;
    context.picker_open = true;
    assert!(document_menu::escape(&context, &escape, Modifiers::default()).is_none());
}

#[test]
fn native_commands_own_the_press_repeat_and_release_even_after_the_scope_changes() {
    let (mut root, _signals) = mounted();
    let (context, messages) = context();
    scope(&mut root, context);
    let mut keyboard = Keyboard::default();
    keyboard.synchronize(&mut root);
    keyboard.modifiers = primary();
    let save = NativeKey::Character("s".into());
    let key = press(&save, KeyCode::KeyS);
    assert!(!keyboard.key_event(&root, key, true));
    assert!(messages.try_recv().is_err());
    assert!(keyboard.key_event(&root, key, false));
    assert_eq!(action(&messages), Action::Command(WriterCommand::Save));
    assert!(keyboard.key_event(
        &root,
        Keystroke {
            repeat: true,
            ..key
        },
        false
    ));
    assert!(messages.try_recv().is_err());
    let id = root.get_widget_with_tag(TAG).unwrap().id();
    root.edit_widget(id, |mut widget| {
        widget.remove_prop::<KeyboardContext>();
    });
    assert!(keyboard.key_event(
        &root,
        Keystroke {
            state: ElementState::Released,
            ..key
        },
        true
    ));
    assert!(!keyboard.key_event(&root, key, false));
    assert!(messages.try_recv().is_err());
}

#[test]
fn picker_enter_never_leaks_a_release_and_composition_stays_with_its_field() {
    let (mut root, _signals) = mounted();
    let (mut context, messages) = context();
    context.picker_open = true;
    context.writer_active = false;
    let proxy = context.proxy.clone();
    scope(&mut root, context);
    let mut keyboard = Keyboard::default();
    keyboard.synchronize(&mut root);
    let enter = NativeKey::Named(NamedKey::Enter);
    let key = press(&enter, KeyCode::Enter);
    assert!(keyboard.key_event(&root, key, false));
    assert_eq!(action(&messages), Action::SubmitPicker);
    let (mut context, _) = super::context();
    context.proxy = proxy;
    context.writer_active = true;
    scope(&mut root, context);
    keyboard.synchronize(&mut root);
    assert!(keyboard.key_event(
        &root,
        Keystroke {
            state: ElementState::Released,
            ..key
        },
        false
    ));
    assert!(messages.try_recv().is_err());
    let script = field(&root, SCRIPT);
    root.focus_on(Some(script));
    keyboard.modifiers = primary();
    keyboard.event(&mut root, &WindowEvent::Ime(Ime::Preedit("é".into(), None)));
    assert!(!keyboard.key_event(&root, key, false));
    assert!(messages.try_recv().is_err());
    keyboard.event(&mut root, &WindowEvent::Ime(Ime::Commit("é".into())));
    assert!(keyboard.key_event(&root, key, false));
    assert_eq!(action(&messages), Action::Command(WriterCommand::Apply));
    assert!(keyboard.key_event(
        &root,
        Keystroke {
            state: ElementState::Released,
            ..key
        },
        false
    ));
    assert_eq!(root.focused_widget(), Some(script));
}

#[test]
fn vim_prefix_is_bound_to_focus_and_ime_while_native_text_and_repeat_keep_their_owner() {
    let (mut root, _signals) = mounted();
    let (mut context, messages) = context();
    context.keymap = Keymap::Vim;
    scope(&mut root, context);
    let mut keyboard = Keyboard::default();
    keyboard.synchronize(&mut root);
    let w = NativeKey::Character("w".into());
    let prefix = press(&w, KeyCode::KeyW);
    keyboard.modifiers = ModifiersState::CONTROL;
    assert!(keyboard.key_event(&root, prefix, false));
    assert_eq!(action(&messages), Action::PaneHint(true));
    assert!(keyboard.key_event(
        &root,
        Keystroke {
            repeat: true,
            ..prefix
        },
        false
    ));
    assert!(keyboard.key_event(
        &root,
        Keystroke {
            state: ElementState::Released,
            ..prefix
        },
        true
    ));
    assert!(messages.try_recv().is_err());
    keyboard.modifiers = ModifiersState::default();
    let right = NativeKey::Character("l".into());
    let direction = press(&right, KeyCode::KeyL);
    assert!(keyboard.key_event(&root, direction, false));
    assert_eq!(action(&messages), Action::PaneHint(false));
    assert_eq!(action(&messages), Action::Command(WriterCommand::PaneRight));
    assert!(keyboard.key_event(
        &root,
        Keystroke {
            state: ElementState::Released,
            ..direction
        },
        false
    ));
    // Ordinary manuscript letters retain native editing ownership.
    assert!(!keyboard.key_event(&root, direction, false));
    keyboard.modifiers = ModifiersState::CONTROL;
    assert!(keyboard.key_event(&root, prefix, false));
    assert_eq!(action(&messages), Action::PaneHint(true));
    root.focus_on(Some(field(&root, NAVIGATION)));
    keyboard.synchronize(&mut root);
    assert_eq!(action(&messages), Action::PaneHint(false));
    assert!(keyboard.key_event(
        &root,
        Keystroke {
            state: ElementState::Released,
            ..prefix
        },
        false
    ));
    assert!(keyboard.key_event(&root, prefix, false));
    assert_eq!(action(&messages), Action::PaneHint(true));
    keyboard.event(
        &mut root,
        &WindowEvent::Ime(Ime::Preedit("pré".into(), None)),
    );
    keyboard.synchronize(&mut root);
    assert_eq!(action(&messages), Action::PaneHint(false));
    assert!(!keyboard.key_event(&root, direction, false));
    assert!(messages.try_recv().is_err());
    keyboard.event(&mut root, &WindowEvent::Focused(false));
    assert_eq!(keyboard.modifiers, ModifiersState::default());
}

#[test]
fn mouse_history_posts_once_and_does_not_interrupt_composition_or_a_picker() {
    use xilem::winit::event::MouseButton;
    let (mut root, _signals) = mounted();
    let (context, messages) = context();
    scope(&mut root, context);
    let mut keyboard = Keyboard::default();
    keyboard.synchronize(&mut root);
    for (button, command) in [
        (MouseButton::Back, WriterCommand::Back),
        (MouseButton::Forward, WriterCommand::Forward),
    ] {
        assert!(keyboard.history_mouse(&root, button, ElementState::Pressed));
        assert_eq!(action(&messages), Action::Command(command));
        assert!(keyboard.history_mouse(&root, button, ElementState::Released));
        assert!(messages.try_recv().is_err());
    }
    assert!(!keyboard.history_mouse(&root, MouseButton::Left, ElementState::Pressed));
    keyboard.event(&mut root, &WindowEvent::Ime(Ime::Preedit("é".into(), None)));
    assert!(!keyboard.history_mouse(&root, MouseButton::Back, ElementState::Pressed));
    keyboard.event(&mut root, &WindowEvent::Ime(Ime::Disabled));
    let (mut context, _) = super::context();
    context.picker_open = true;
    scope(&mut root, context);
    assert!(!keyboard.history_mouse(&root, MouseButton::Back, ElementState::Pressed));
    assert!(messages.try_recv().is_err());
}

#[test]
fn personal_function_and_shifted_physical_bindings_reach_the_native_command_host() {
    let (mut root, _signals) = mounted();
    let mut keyboard = Keyboard::default();
    let functions = [
        NamedKey::F1,
        NamedKey::F2,
        NamedKey::F3,
        NamedKey::F4,
        NamedKey::F5,
        NamedKey::F6,
        NamedKey::F7,
        NamedKey::F8,
        NamedKey::F9,
        NamedKey::F10,
        NamedKey::F11,
        NamedKey::F12,
    ];
    for (index, function) in functions.into_iter().enumerate() {
        if function == NamedKey::F6 {
            continue;
        } // F6 is reserved for native pane cycling.
        let (mut context, messages) = context();
        context
            .shortcuts
            .rebind(
                WriterCommand::Save,
                format!("F{}", index + 1).try_into().unwrap(),
            )
            .unwrap();
        scope(&mut root, context);
        keyboard.synchronize(&mut root);
        keyboard.modifiers = ModifiersState::default();
        let logical = NativeKey::Named(function);
        let key = press(&logical, KeyCode::F1);
        assert!(keyboard.key_event(&root, key, false));
        assert_eq!(action(&messages), Action::Command(WriterCommand::Save));
        assert!(keyboard.key_event(
            &root,
            Keystroke {
                state: ElementState::Released,
                ..key
            },
            false
        ));
        assert!(messages.try_recv().is_err());
    }
    // Shifted symbols retain the configured physical number/comma chord.
    let logical = NativeKey::Character("!".into());
    for (physical, binding) in [
        (KeyCode::Digit0, "0"),
        (KeyCode::Digit1, "1"),
        (KeyCode::Digit2, "2"),
        (KeyCode::Digit3, "3"),
        (KeyCode::Digit4, "4"),
        (KeyCode::Digit5, "5"),
        (KeyCode::Digit6, "6"),
        (KeyCode::Digit7, "7"),
        (KeyCode::Digit8, "8"),
        (KeyCode::Digit9, "9"),
        (KeyCode::Comma, ","),
    ] {
        let (mut context, messages) = context();
        context
            .shortcuts
            .rebind(
                WriterCommand::Save,
                format!("Primary+Shift+{binding}").try_into().unwrap(),
            )
            .unwrap();
        scope(&mut root, context);
        keyboard.modifiers = primary() | ModifiersState::SHIFT;
        let key = press(&logical, physical);
        assert!(keyboard.key_event(&root, key, false), "{physical:?}");
        assert_eq!(action(&messages), Action::Command(WriterCommand::Save));
        assert!(keyboard.key_event(
            &root,
            Keystroke {
                state: ElementState::Released,
                ..key
            },
            false
        ));
    }
}
