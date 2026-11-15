use super::*;
use crate::native_host::keyboard::tests::context;

#[test]
fn alt_arrow_history_requires_an_active_writer_without_a_picker_or_extra_modifiers() {
    let (mut context, _) = context();
    let alt = Modifiers {
        alt: true,
        ..Modifiers::default()
    };
    for (key_name, command) in [
        (NamedKey::ArrowLeft, WriterCommand::Back),
        (NamedKey::ArrowRight, WriterCommand::Forward),
    ] {
        let arrow = keyboard::Key::Named(key_name);
        assert_eq!(key(&context, &arrow, alt), Some(Action::Command(command)));
        assert!(key(&context, &arrow, Modifiers::default()).is_none());
        assert!(key(&context, &arrow, Modifiers { shift: true, ..alt }).is_none());
        context.picker_open = true;
        assert!(key(&context, &arrow, alt).is_none());
        context.picker_open = false;
        context.screen = None;
        assert!(key(&context, &arrow, alt).is_none());
        context.screen = Some(crate::workspaces::Screen::Write);
    }
}

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
