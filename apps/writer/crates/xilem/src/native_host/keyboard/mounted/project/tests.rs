use super::*;
use xilem::winit::keyboard::{Key as NativeKey, KeyCode, PhysicalKey as NativePhysical};

#[test]
fn project_enter_owns_its_release_and_leaves_composition_with_the_native_field() {
    let (mut root, _signals) = mounted();
    let project_input = root.get_widget_with_tag(QUERY).unwrap().id();
    root.edit_widget(project_input, |mut widget| {
        widget.insert_prop(crate::app::ProjectField(true));
        widget.remove_prop::<QueryField>();
    });
    let project_field = field(&root, QUERY);
    root.focus_on(Some(project_field));
    let (mut context, messages) = context();
    context.project_form_ready = true;
    context.writer_active = false;
    input::scope(&mut root, context);
    let mut keyboard = Keyboard::default();
    let enter = NativeKey::Named(NamedKey::Enter);
    let stroke = Keystroke {
        logical_key: &enter,
        physical_key: NativePhysical::Code(KeyCode::Enter),
        state: ElementState::Pressed,
        repeat: false,
    };
    assert!(keyboard.key_event(&root, stroke, false));
    let invocation = messages
        .try_recv()
        .unwrap()
        .downcast::<Invocation>()
        .unwrap();
    assert_eq!(invocation.action, Action::SubmitProject);
    assert!(keyboard.key_event(
        &root,
        Keystroke {
            repeat: true,
            ..stroke
        },
        false
    ));
    assert!(messages.try_recv().is_err());
    let (context, _) = super::context();
    input::scope(&mut root, context);
    assert!(keyboard.key_event(
        &root,
        Keystroke {
            state: ElementState::Released,
            ..stroke
        },
        false
    ));
    assert!(messages.try_recv().is_err());

    let (mut context, messages) = super::context();
    context.project_form_ready = true;
    context.writer_active = false;
    input::scope(&mut root, context);
    keyboard.event(&mut root, &WindowEvent::Ime(Ime::Preedit("é".into(), None)));
    assert!(!keyboard.key_event(&root, stroke, false));
    assert!(messages.try_recv().is_err());
}
