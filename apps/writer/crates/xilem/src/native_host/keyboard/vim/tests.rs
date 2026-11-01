use super::*;
use crate::native_host::keyboard::tests::context;
use xilem::winit::keyboard::KeyCode;

#[test]
fn pane_prefix_cancels_on_scope_change_and_owns_invalid_terminal_releases() {
    let (mut context, messages) = context();
    context.keymap = recite_config::Keymap::Vim;
    let mut vim = Vim::default();
    let w = keyboard::PhysicalKey::Code(KeyCode::KeyW);
    let enter = keyboard::PhysicalKey::Code(KeyCode::Enter);
    assert!(vim.press(&context, None, Chord::Control("w"), w));
    assert!(matches!(
        messages
            .try_recv()
            .unwrap()
            .downcast::<Invocation>()
            .unwrap()
            .action,
        Action::PaneHint(true)
    ));
    assert!(vim.release(w, ElementState::Released));
    assert!(vim.press(&context, None, Chord::Other, enter));
    assert!(matches!(
        messages
            .try_recv()
            .unwrap()
            .downcast::<Invocation>()
            .unwrap()
            .action,
        Action::PaneHint(false)
    ));
    assert!(messages.try_recv().is_err());
    assert!(vim.release(enter, ElementState::Pressed));
    assert!(vim.release(enter, ElementState::Released));
    assert!(!vim.release(enter, ElementState::Released));
    assert!(vim.press(&context, None, Chord::Control("w"), w));
    messages.try_recv().unwrap();
    context.picker_open = true;
    vim.synchronize(&context, None);
    assert!(vim.pending.is_none());
    assert!(matches!(
        messages
            .try_recv()
            .unwrap()
            .downcast::<Invocation>()
            .unwrap()
            .action,
        Action::PaneHint(false)
    ));
}

#[test]
fn pane_direction_uses_product_commands_and_clears_the_hint_before_dispatch() {
    let (mut context, messages) = context();
    context.keymap = recite_config::Keymap::Vim;
    let mut vim = Vim::default();
    assert!(vim.press(
        &context,
        None,
        Chord::Control("w"),
        keyboard::PhysicalKey::Code(KeyCode::KeyW)
    ));
    messages.try_recv().unwrap();
    assert!(vim.press(
        &context,
        None,
        Chord::Plain("l"),
        keyboard::PhysicalKey::Code(KeyCode::KeyL)
    ));
    assert!(matches!(
        messages
            .try_recv()
            .unwrap()
            .downcast::<Invocation>()
            .unwrap()
            .action,
        Action::PaneHint(false)
    ));
    assert!(matches!(
        messages
            .try_recv()
            .unwrap()
            .downcast::<Invocation>()
            .unwrap()
            .action,
        Action::Command(WriterCommand::PaneRight)
    ));
    assert!(vim.pending.is_none());
    assert!(messages.try_recv().is_err());
}

#[test]
fn configured_commands_clear_the_prefix_hint_before_retiring_the_keyboard_scope() {
    let (mut context, messages) = context();
    context.keymap = recite_config::Keymap::Vim;
    let mut keyboard = Keyboard::default();
    keyboard.vim.press(
        &context,
        None,
        Chord::Control("w"),
        keyboard::PhysicalKey::Code(KeyCode::KeyW),
    );
    messages.try_recv().unwrap();
    assert!(keyboard.dispatch(
        &context,
        None,
        focus::Kind::Other,
        Stroke {
            input: Input {
                key: Key::Character("s"),
                physical: PhysicalKey::Other,
                modifiers: Modifiers {
                    primary: true,
                    ..Modifiers::default()
                }
            },
            enter: false,
            action: None,
            first_press: true,
        }
    ));
    assert!(matches!(
        messages
            .try_recv()
            .unwrap()
            .downcast::<Invocation>()
            .unwrap()
            .action,
        Action::PaneHint(false)
    ));
    assert!(matches!(
        messages
            .try_recv()
            .unwrap()
            .downcast::<Invocation>()
            .unwrap()
            .action,
        Action::Command(WriterCommand::Save)
    ));
}
