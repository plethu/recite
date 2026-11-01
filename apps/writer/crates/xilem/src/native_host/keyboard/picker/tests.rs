use super::*;

#[test]
fn enter_submits_picker_from_query_or_active_result_but_preserves_other_native_input() {
    let (mut context, _) = super::super::tests::context();
    let enter = keyboard::Key::Named(NamedKey::Enter);
    let modifiers = Modifiers::default();
    assert!(action(&context, focus::Kind::Query, &enter, modifiers).is_none());
    context.picker_open = true;
    for kind in [focus::Kind::Query, focus::Kind::ActiveResult] {
        assert_eq!(
            action(&context, kind, &enter, modifiers),
            Some(Action::SubmitPicker)
        );
    }
    assert!(action(&context, focus::Kind::Text, &enter, modifiers).is_none());
    assert!(
        action(
            &context,
            focus::Kind::Query,
            &enter,
            Modifiers {
                shift: true,
                ..modifiers
            }
        )
        .is_none()
    );
}
