use super::*;
use crate::native_host::keyboard::tests::context;

#[test]
fn primary_arrows_step_between_beats_only_in_the_writing_screen_without_a_picker() {
    let (mut context, _) = context();
    let primary = Modifiers {
        primary: true,
        ..Modifiers::default()
    };
    let down = keyboard::Key::Named(NamedKey::ArrowDown);
    let up = keyboard::Key::Named(NamedKey::ArrowUp);
    assert_eq!(
        beat_key(&context, &down, primary),
        Some(Action::StepBeat { forward: true })
    );
    assert_eq!(
        beat_key(&context, &up, primary),
        Some(Action::StepBeat { forward: false })
    );
    assert!(beat_key(&context, &down, Modifiers::default()).is_none());
    context.picker_open = true;
    assert!(beat_key(&context, &down, primary).is_none());
    context.picker_open = false;
    context.screen = Some(crate::workspaces::Screen::Localisation);
    assert!(beat_key(&context, &down, primary).is_none());
}

#[test]
fn escape_leaves_focus_mode_unless_a_picker_takes_it() {
    let (mut context, _) = context();
    let escape = keyboard::Key::Named(NamedKey::Escape);
    assert!(focus_key(&context, &escape, Modifiers::default()).is_none());
    let mut writer =
        crate::Writer::from_session(crate::session::Session::examples().unwrap(), None);
    writer.toggle_focus();
    context.attention = writer.attention.clone();
    assert_eq!(
        focus_key(&context, &escape, Modifiers::default()),
        Some(Action::Command(WriterCommand::Focus))
    );
    context.picker_open = true;
    assert!(focus_key(&context, &escape, Modifiers::default()).is_none());
}

#[test]
fn escape_dismisses_a_modal_surface_unless_a_picker_takes_it() {
    let (mut context, _) = context();
    let escape = keyboard::Key::Named(NamedKey::Escape);
    assert!(modal_key(&context, &escape, Modifiers::default()).is_none());
    for modal in [Modal::Settings, Modal::CloseDecision] {
        context.modal = Some(modal);
        assert_eq!(
            modal_key(&context, &escape, Modifiers::default()),
            Some(Action::DismissModal(modal))
        );
        context.picker_open = true;
        assert!(modal_key(&context, &escape, Modifiers::default()).is_none());
        context.picker_open = false;
    }
}
