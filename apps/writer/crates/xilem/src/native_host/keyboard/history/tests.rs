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
