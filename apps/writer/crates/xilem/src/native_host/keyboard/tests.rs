use super::*;
use std::sync::{Arc, mpsc};
use xilem::core::{MessageProxy, ProxyError, RawProxy, SendMessage, ViewId};
use xilem::masonry::core;
use xilem::masonry::testing;
use xilem::masonry::theme;
use xilem::masonry::widgets;
#[path = "mounted/tests.rs"]
mod mounted;

#[derive(Debug)]
struct Recorder(mpsc::Sender<SendMessage>);
impl RawProxy for Recorder {
    fn send_message(&self, _path: Arc<[ViewId]>, message: SendMessage) -> Result<(), ProxyError> {
        self.0
            .send(message)
            .map_err(|error| ProxyError::DriverFinished(error.0))
    }
    fn dyn_debug(&self) -> &dyn std::fmt::Debug {
        self
    }
}
pub(super) fn context() -> (Context, mpsc::Receiver<SendMessage>) {
    let (sender, receiver) = mpsc::channel();
    (
        Context {
            shortcuts: Default::default(),
            generation: Default::default(),
            writer_active: true,
            screen: Some(crate::workspaces::Screen::Write),
            picker_open: false,
            project_form_ready: false,
            document_menu: Default::default(),
            attention: Default::default(),
            editing: None,
            field_restoration: None,
            keymap: recite_config::Keymap::default(),
            proxy: MessageProxy::new(Arc::new(Recorder(sender)), Arc::from([])),
        },
        receiver,
    )
}
fn stroke(key: Key<'_>) -> Stroke<'_> {
    Stroke {
        input: Input {
            key,
            physical: PhysicalKey::Other,
            modifiers: Modifiers {
                primary: true,
                ..Modifiers::default()
            },
        },
        enter: false,
        action: None,
        first_press: true,
    }
}

#[test]
fn picker_submission_posts_once_and_does_not_fall_through_to_underlying_writer_commands() {
    let (mut context, receiver) = context();
    context.picker_open = true;
    context.writer_active = false;
    let mut keyboard = Keyboard::default();
    for first_press in [true, false] {
        let mut enter = stroke(Key::Other);
        enter.input.modifiers = Modifiers::default();
        enter.enter = true;
        enter.action = Some(Action::SubmitPicker);
        enter.first_press = first_press;
        assert!(keyboard.dispatch(&context, None, focus::Kind::Other, enter));
    }
    assert_eq!(
        receiver
            .try_recv()
            .unwrap()
            .downcast::<Invocation>()
            .unwrap()
            .action,
        Action::SubmitPicker
    );
    assert!(receiver.try_recv().is_err());
}

#[test]
fn saved_bindings_take_effect_and_repeat_release_and_native_undo_do_not_post_actions() {
    let (mut context, receiver) = context();
    let mut keyboard = Keyboard::default();
    assert!(keyboard.dispatch(
        &context,
        None,
        focus::Kind::Other,
        stroke(Key::Character("s"))
    ));
    let action = receiver
        .try_recv()
        .unwrap()
        .downcast::<Invocation>()
        .unwrap();
    assert_eq!(action.action, Action::Command(WriterCommand::Save));
    assert_eq!(action.generation, context.generation);
    let mut repeat = stroke(Key::Character("s"));
    repeat.first_press = false;
    assert!(keyboard.dispatch(&context, None, focus::Kind::Other, repeat));
    assert!(receiver.try_recv().is_err());
    context
        .shortcuts
        .rebind(
            WriterCommand::Save,
            "Primary+Alt+S".to_owned().try_into().unwrap(),
        )
        .unwrap();
    assert!(!keyboard.dispatch(
        &context,
        None,
        focus::Kind::Other,
        stroke(Key::Character("s"))
    ));
    let mut rebound = stroke(Key::Character("s"));
    rebound.input.modifiers.alt = true;
    assert!(keyboard.dispatch(&context, None, focus::Kind::Other, rebound));
    assert_eq!(
        receiver
            .try_recv()
            .unwrap()
            .downcast::<Invocation>()
            .unwrap()
            .action,
        Action::Command(WriterCommand::Save)
    );
    for character in ["z", "y", "c", "v", "x", "a"] {
        assert!(!keyboard.dispatch(
            &context,
            None,
            focus::Kind::Other,
            stroke(Key::Character(character))
        ));
    }
    assert!(receiver.try_recv().is_err());
}

#[test]
fn apply_belongs_to_the_active_manuscript_and_composition_bypasses_commands() {
    let (mut context, receiver) = context();
    let mut keyboard = Keyboard::default();
    let harness = testing::TestHarness::create(
        theme::default_property_set(),
        core::NewWidget::new(widgets::Label::new("Composition owner")),
    );
    let focus = Some(harness.root_id());
    keyboard.composition = Composition::Preedit(focus);
    assert!(!keyboard.dispatch(
        &context,
        focus,
        focus::Kind::Manuscript,
        stroke(Key::Character("s"))
    ));
    assert!(receiver.try_recv().is_err());
    // Moving focus retires the old field's preedit guard.
    assert!(keyboard.dispatch(
        &context,
        None,
        focus::Kind::Other,
        stroke(Key::Character("s"))
    ));
    receiver.try_recv().unwrap();
    for (active, manuscript, expected) in [
        (true, false, false),
        (false, true, false),
        (true, true, true),
    ] {
        context.writer_active = active;
        let mut enter = stroke(Key::Other);
        enter.enter = true;
        assert_eq!(
            keyboard.dispatch(
                &context,
                focus,
                if manuscript {
                    focus::Kind::Manuscript
                } else {
                    focus::Kind::Other
                },
                enter
            ),
            expected
        );
        if expected {
            assert_eq!(
                receiver
                    .try_recv()
                    .unwrap()
                    .downcast::<Invocation>()
                    .unwrap()
                    .action,
                Action::Command(WriterCommand::Apply)
            );
        } else {
            assert!(receiver.try_recv().is_err());
        }
    }
    keyboard.modifiers = ModifiersState::CONTROL;
    keyboard.composition = Composition::Preedit(focus);
    keyboard.reset();
    assert_eq!(keyboard.normalized_modifiers(), Modifiers::default());
    assert!(matches!(keyboard.composition, Composition::Idle));
}

#[test]
fn standard_f6_cycles_panes_in_both_directions_and_releases_do_not_repeat() {
    let (context, messages) = context();
    let mut keyboard = Keyboard::default();
    for shift in [false, true] {
        let mut input = stroke(Key::Function(6));
        input.input.modifiers = Modifiers {
            shift,
            ..Modifiers::default()
        };
        assert!(keyboard.dispatch(&context, None, focus::Kind::Other, input));
        assert_eq!(
            messages
                .try_recv()
                .unwrap()
                .downcast::<Invocation>()
                .unwrap()
                .action,
            Action::CyclePane { forward: !shift }
        );
        let mut release = stroke(Key::Function(6));
        release.input.modifiers = Modifiers {
            shift,
            ..Modifiers::default()
        };
        release.first_press = false;
        assert!(keyboard.dispatch(&context, None, focus::Kind::Other, release));
        assert!(messages.try_recv().is_err());
    }
}
